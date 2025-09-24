use crate::{MirrorHandle, build_state_digest, control::MirrorControl};
use chrono::Utc;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use engine_v3::{SimulationEngine, scenario::Scenario};
use metrics::histogram;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
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
    #[error("control transport error: {0}")]
    Transport(String),
    #[error("control request timed out")]
    Timeout,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    remote_control: Mutex<Option<crate::JetStreamControlServer>>,
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
        let engine_instance = scenario.initialize_engine();
        let initial_digest = build_state_digest(&engine_instance, &[]);
        let mirror = MirrorHandle::with_initial(initial_digest);
        let engine = Arc::new(parking_lot::RwLock::new(engine_instance));

        let (control_tx, control_rx) = unbounded();
        let initial_running = false;
        let status = Arc::new(MirrorStatus::new(initial_running, tick_interval));
        let controller = MirrorController::new(control_tx.clone(), status.clone());
        let worker = spawn_tick_thread(
            mirror.clone(),
            engine.clone(),
            scenario.clone(),
            tick_interval,
            control_rx,
            status.clone(),
            initial_running,
        );

        Ok(Self { mirror, engine, scenario, controller, worker: Some(worker), remote_control: Mutex::new(None) })
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

    pub fn attach_jetstream(&self, config: crate::JetStreamConfig) -> Result<(), crate::JetStreamError> {
        self.attach_jetstream_with_schema(config, crate::JetStreamSchema::default())
    }

    pub fn attach_jetstream_with_schema(
        &self, config: crate::JetStreamConfig, schema: crate::JetStreamSchema,
    ) -> Result<(), crate::JetStreamError> {
        let publisher_config = config.clone();
        let mut control_config = config;
        control_config.connection_name = format!("{}-control", control_config.connection_name);

        let publisher = crate::JetStreamPublisher::connect(publisher_config, schema.clone())?;
        self.mirror.attach_publisher(publisher);
        let mut guard = self.remote_control.lock().expect("remote control guard");
        if guard.is_none() {
            let server = crate::JetStreamControlServer::start(control_config, schema, self.controller())?;
            *guard = Some(server);
        }
        Ok(())
    }
}

impl Drop for MirrorRuntime {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.remote_control.lock() {
            guard.take();
        }
        self.controller.shutdown();
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

impl MirrorControl for MirrorController {
    fn pause(&self) -> Result<(), ControlError> {
        MirrorController::pause(self)
    }

    fn resume(&self) -> Result<(), ControlError> {
        MirrorController::resume(self)
    }

    fn step(&self) -> Result<(), ControlError> {
        MirrorController::step(self)
    }

    fn set_interval(&self, interval: Duration) -> Result<(), ControlError> {
        MirrorController::set_interval(self, interval)
    }

    fn status(&self) -> MirrorControlStatus {
        MirrorController::status(self)
    }
}

fn spawn_tick_thread(
    mirror: MirrorHandle, engine: Arc<parking_lot::RwLock<SimulationEngine>>, scenario: Arc<Scenario>,
    tick_interval: Duration, control_rx: Receiver<ControlCommand>, status: Arc<MirrorStatus>, initial_running: bool,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut rng = StdRng::seed_from_u64(scenario.config.seed);
        let mut restart_counter: u64 = 0;
        let mut running = initial_running;
        let mut step_once = false;
        let mut interval = tick_interval.max(Duration::from_millis(1));
        status.set_running(initial_running);
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
                    build_state_digest(&eng, &[])
                } else {
                    let (_result, events) = eng.run_tick(&mut rng);
                    build_state_digest(&eng, &events)
                }
            };

            let snapshot = mirror.publish(digest);
            let publish_latency_ms = Utc::now()
                .signed_duration_since(snapshot.digest.timings.generated_at)
                .num_microseconds()
                .unwrap_or_default() as f64
                / 1_000.0;
            histogram!("mirror.publish.latency_ms", publish_latency_ms, "source" => "runtime");

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
