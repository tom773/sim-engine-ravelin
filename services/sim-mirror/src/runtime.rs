use crate::{DigestEvent, DigestEventKind, DigestMetrics, MirrorHandle, StateDigest};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use engine_v3::{SimulationEngine, scenario::Scenario};
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::Serialize;
use sim_core::prelude::*;
use sim_core::types::events::TickEventSummary;
use sim_core::types::instrument::{InstrumentRuntime, RealAssetState};
use sim_core::types::system::financial_system::FinancialSystem;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MirrorError {
    #[error("failed to read scenario config: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse scenario config: {0}")]
    Config(#[from] toml::de::Error),
}

#[derive(Error, Debug)]
pub enum ControlError {
    #[error("mirror runtime worker is not running")]
    Disconnected,
}

const DEFAULT_TICK_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug)]
enum ControlCommand {
    Pause,
    Resume,
    Step,
    SetInterval(Duration),
    Shutdown,
}

#[derive(Debug)]
struct MirrorStatus {
    running: AtomicBool,
    tick_interval_ms: AtomicU64,
}

impl MirrorStatus {
    fn new(initial_running: bool, tick_interval: Duration) -> Self {
        Self {
            running: AtomicBool::new(initial_running),
            tick_interval_ms: AtomicU64::new(tick_interval.as_millis().max(1) as u64),
        }
    }

    fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::Relaxed);
    }

    fn set_interval(&self, interval: Duration) {
        self.tick_interval_ms.store(interval.as_millis().max(1) as u64, Ordering::Relaxed);
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MirrorControlStatus {
    pub running: bool,
    pub tick_interval_ms: u64,
}

#[derive(Clone)]
pub struct MirrorController {
    tx: Sender<ControlCommand>,
    status: Arc<MirrorStatus>,
}

impl MirrorController {
    fn new(tx: Sender<ControlCommand>, status: Arc<MirrorStatus>) -> Self {
        Self { tx, status }
    }

    fn send(&self, cmd: ControlCommand) -> Result<(), ControlError> {
        self.tx.send(cmd).map_err(|_| ControlError::Disconnected)
    }

    pub fn pause(&self) -> Result<(), ControlError> {
        self.status.set_running(false);
        self.send(ControlCommand::Pause)
    }

    pub fn resume(&self) -> Result<(), ControlError> {
        self.status.set_running(true);
        self.send(ControlCommand::Resume)
    }

    pub fn step(&self) -> Result<(), ControlError> {
        self.status.set_running(false);
        self.send(ControlCommand::Step)
    }

    pub fn set_interval(&self, interval: Duration) -> Result<(), ControlError> {
        self.status.set_interval(interval);
        self.send(ControlCommand::SetInterval(interval))
    }

    pub fn status(&self) -> MirrorControlStatus {
        MirrorControlStatus {
            running: self.status.running.load(Ordering::Relaxed),
            tick_interval_ms: self.status.tick_interval_ms.load(Ordering::Relaxed),
        }
    }

    fn shutdown(&self) {
        let _ = self.tx.send(ControlCommand::Shutdown);
    }
}

pub struct MirrorRuntime {
    mirror: MirrorHandle,
    engine: Arc<parking_lot::RwLock<SimulationEngine>>,
    scenario: Arc<Scenario>,
    controller: MirrorController,
    worker: Option<JoinHandle<()>>,
}

impl MirrorRuntime {
    pub fn start_from_config(config_path: impl AsRef<Path>) -> Result<Self, MirrorError> {
        Self::start_from_config_with_interval(config_path, DEFAULT_TICK_INTERVAL)
    }

    pub fn start_from_config_with_interval(
        config_path: impl AsRef<Path>, tick_interval: Duration,
    ) -> Result<Self, MirrorError> {
        let config_str = fs::read_to_string(config_path)?;
        let scenario = Scenario::from_toml_str(&config_str)?;
        Self::start_from_scenario(Arc::new(scenario), tick_interval)
    }

    pub fn start_from_scenario(scenario: Arc<Scenario>, tick_interval: Duration) -> Result<Self, MirrorError> {
        let engine = scenario.initialize_engine();
        let mirror = MirrorHandle::new();
        let engine = Arc::new(parking_lot::RwLock::new(engine));

        {
            let engine_guard = engine.read();
            let digest = build_state_digest(&engine_guard.state, &[], &engine_guard.state.financial_system);
            mirror.publish(digest);
        }

        let (control_tx, control_rx) = unbounded();
        let status = Arc::new(MirrorStatus::new(true, tick_interval));
        let controller = MirrorController::new(control_tx.clone(), status.clone());
        let worker = spawn_tick_thread(
            mirror.clone(),
            engine.clone(),
            scenario.clone(),
            tick_interval,
            control_rx,
            status.clone(),
        );

        Ok(Self { mirror, engine, scenario, controller, worker: Some(worker) })
    }

    pub fn mirror_handle(&self) -> MirrorHandle {
        self.mirror.clone()
    }

    pub fn engine(&self) -> Arc<parking_lot::RwLock<SimulationEngine>> {
        self.engine.clone()
    }

    pub fn scenario(&self) -> Arc<Scenario> {
        self.scenario.clone()
    }

    pub fn controller(&self) -> MirrorController {
        self.controller.clone()
    }
}

impl Drop for MirrorRuntime {
    fn drop(&mut self) {
        self.controller.shutdown();
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

fn spawn_tick_thread(
    mirror: MirrorHandle, engine: Arc<parking_lot::RwLock<SimulationEngine>>, scenario: Arc<Scenario>,
    tick_interval: Duration, control_rx: Receiver<ControlCommand>, status: Arc<MirrorStatus>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut rng = StdRng::seed_from_u64(scenario.config.seed);
        let mut restart_counter: u64 = 0;
        let mut running = true;
        let mut step_once = false;
        let mut interval = tick_interval.max(Duration::from_millis(1));
        status.set_running(true);
        status.set_interval(interval);

        let handle_command =
            |cmd: ControlCommand, running: &mut bool, step_once: &mut bool, interval: &mut Duration| -> bool {
                match cmd {
                    ControlCommand::Pause => {
                        *running = false;
                        status.set_running(false);
                    }
                    ControlCommand::Resume => {
                        *running = true;
                        status.set_running(true);
                    }
                    ControlCommand::Step => {
                        *step_once = true;
                        *running = false;
                        status.set_running(false);
                    }
                    ControlCommand::SetInterval(new_interval) => {
                        let clamped = new_interval.max(Duration::from_millis(1));
                        *interval = clamped;
                        status.set_interval(clamped);
                    }
                    ControlCommand::Shutdown => return true,
                }
                false
            };

        loop {
            while let Ok(cmd) = control_rx.try_recv() {
                if handle_command(cmd, &mut running, &mut step_once, &mut interval) {
                    return;
                }
            }

            if !running && !step_once {
                match control_rx.recv_timeout(Duration::from_millis(10)) {
                    Ok(cmd) => {
                        if handle_command(cmd, &mut running, &mut step_once, &mut interval) {
                            return;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
                continue;
            }

            let digest = {
                let mut eng = engine.write();
                if eng.state.ticknum >= eng.state.config.iterations {
                    let mut fresh = scenario.initialize_engine();
                    std::mem::swap(&mut *eng, &mut fresh);
                    restart_counter = restart_counter.wrapping_add(1);
                    let new_seed = scenario.config.seed.wrapping_add(restart_counter);
                    rng = StdRng::seed_from_u64(new_seed);
                    build_state_digest(&eng.state, &[], &eng.state.financial_system)
                } else {
                    let (_result, events) = eng.run_tick(&mut rng);
                    build_state_digest(&eng.state, &events, &eng.state.financial_system)
                }
            };

            mirror.publish(digest);

            if step_once {
                step_once = false;
                running = false;
                status.set_running(false);
            }

            if let Ok(cmd) = control_rx.try_recv() {
                if handle_command(cmd, &mut running, &mut step_once, &mut interval) {
                    return;
                }
            }

            if !running {
                continue;
            }

            let deadline = Instant::now() + interval;
            while running {
                let now = Instant::now();
                if now >= deadline {
                    break;
                }
                let wait = (deadline - now).min(Duration::from_millis(10));
                match control_rx.recv_timeout(wait) {
                    Ok(cmd) => {
                        if handle_command(cmd, &mut running, &mut step_once, &mut interval) {
                            return;
                        }
                        if step_once {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        }
    })
}

fn build_state_digest(state: &SimState, events: &[SimEvent], system: &FinancialSystem) -> StateDigest {
    let total_agents = (state.agents.banks.len() + state.agents.firms.len() + state.agents.consumers.len()) as u32;

    let mut total_cash = 0.0;
    let mut total_inventory = 0.0;
    let instruments = &system.instruments.instruments;

    for balance_sheet in system.balance_sheets.values() {
        for (inst_id, position) in &balance_sheet.assets {
            if let Some(inst) = instruments.get(inst_id) {
                match inst.state() {
                    InstrumentRuntime::Cash(_) => {
                        total_cash += position.quantity;
                    }
                    InstrumentRuntime::RealAsset(RealAssetState::Inventory { goods, .. }) => {
                        total_inventory += goods.values().map(|item| item.quantity).sum::<f64>();
                    }
                    _ => {}
                }
            }
        }
    }

    let metrics = DigestMetrics::new(total_agents, total_cash, total_inventory);
    let summary = TickEventSummary::from_events(events);

    let mut highlights = Vec::with_capacity(4);
    highlights.push(DigestEvent::info("tick", format!("tick {} complete", state.ticknum)));
    highlights.push(DigestEvent::info("date", state.current_date.format("%Y-%m-%d").to_string()));
    highlights.push(DigestEvent::info(
        "events",
        format!("{} events ({} kinds)", summary.total_events, summary.by_kind.len()),
    ));

    if let Some(top) = summary.by_kind.first() {
        highlights.push(DigestEvent {
            kind: DigestEventKind::Info("top_event".to_string()),
            message: format!("{:?}: {} occurrences", top.kind, top.count),
        });
    }

    StateDigest::new(state.ticknum, state.current_date.format("%Y-%m-%d").to_string(), metrics, highlights)
}
