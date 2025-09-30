use wasm_bindgen::prelude::*;

use arrow::array::{ArrayRef, Float32Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::Result as ArrowResult;
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::RecordBatch;
use chrono::Utc;
#[cfg(target_arch = "wasm32")]
use console_error_panic_hook;
#[cfg(target_arch = "wasm32")]
use engine_v3::scheduler::{TickExecutionResult, TickStep};
use engine_v3::{Scenario, SimulationEngine};
#[cfg(target_arch = "wasm32")]
use js_sys::{Date, Uint8Array};
use rand::{SeedableRng, rngs::StdRng};
#[cfg(target_arch = "wasm32")]
use rmp_serde::to_vec_named;
use serde::Serialize;
use serde_wasm_bindgen::to_value;
use sim_core::types::events::TickEventSummary;
use sim_core::types::state::TickRecord;
#[cfg(target_arch = "wasm32")]
use sim_mirror::DigestPhaseTiming;
use sim_mirror::{
    BehaviourDigest, BehaviourTickDigest, DigestTimings, MirrorHandle, StateDigest, StateSnapshot,
    TickDetailDigest, build_state_digest, build_state_digest_with_metrics,
};
use std::cell::{Cell, RefCell};
use std::sync::Arc;
use wasm_bindgen::JsValue;

const BEHAVIOUR_HISTORY_EXPORT_MAX: usize = 500;
const BEHAVIOUR_DETAIL_EXPORT_LIMIT: usize = 10;
const BEHAVIOUR_EVENT_EXPORT_LIMIT: usize = 200;

#[derive(Serialize)]
struct MirrorStatus {
    running: bool,
    tick_interval_ms: u32,
}

#[derive(Clone, Debug, Default)]
struct StepMetrics {
    run_tick_ms: f64,
    digest_build_ms: f64,
    #[cfg(target_arch = "wasm32")]
    digest_phases: Vec<DigestPhaseTiming>,
    #[cfg(target_arch = "wasm32")]
    engine_phases: Vec<EnginePhaseTiming>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, Default)]
struct EnginePhaseTiming {
    step: String,
    duration_ms: f64,
}

struct StepOutcome {
    digest: StateDigest,
    metrics: StepMetrics,
    was_reset: bool,
}

#[cfg(target_arch = "wasm32")]
struct StepTimer {
    start_ms: f64,
}

#[cfg(target_arch = "wasm32")]
impl StepTimer {
    fn start() -> Self {
        Self { start_ms: Date::now() }
    }

    fn elapsed_ms(&self) -> f64 {
        Date::now() - self.start_ms
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct StepTimer {
    start: std::time::Instant,
}

#[cfg(not(target_arch = "wasm32"))]
impl StepTimer {
    fn start() -> Self {
        Self { start: std::time::Instant::now() }
    }

    fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1_000.0
    }
}

struct EngineCtx {
    scenario: Scenario,
    engine: SimulationEngine,
    rng: StdRng,
}
impl EngineCtx {
    fn new(scenario: Scenario) -> Self {
        let mut engine = scenario.initialize_engine();
        engine.set_tick_logging(false);
        let rng = StdRng::seed_from_u64(scenario.config.seed);
        Self { scenario, engine, rng }
    }

    fn snapshot(&self) -> StateDigest {
        build_state_digest(&self.engine, &[])
    }

    fn reset(&mut self) -> StateDigest {
        let mut engine = self.scenario.initialize_engine();
        engine.set_tick_logging(false);
        self.engine = engine;
        self.rng = StdRng::seed_from_u64(self.scenario.config.seed);
        self.snapshot()
    }

    fn step(&mut self) -> Result<StepOutcome, String> {
        if self.engine.state.ticknum >= self.engine.state.config.iterations {
            let reset_start = StepTimer::start();
            let digest = self.reset();
            let metrics = StepMetrics {
                run_tick_ms: 0.0,
                digest_build_ms: reset_start.elapsed_ms(),
                #[cfg(target_arch = "wasm32")]
                digest_phases: Vec::new(),
                #[cfg(target_arch = "wasm32")]
                engine_phases: Vec::new(),
            };
            return Ok(StepOutcome { digest, metrics, was_reset: true });
        }

        let (execution_result, events) = self.engine.run_tick(&mut self.rng);
        prune_history(&mut self.engine);
        let run_tick_ms = execution_result.total_duration.as_secs_f64() * 1_000.0;

        let (digest, phases) = build_state_digest_with_metrics(&self.engine, &events);
        #[cfg(target_arch = "wasm32")]
        let digest_phases = phases;
        #[cfg(not(target_arch = "wasm32"))]
        let _ = phases;
        #[cfg(target_arch = "wasm32")]
        let engine_phases = collect_engine_phases(&execution_result);
        let digest_build_ms = digest.timings.build_duration_ms;

        let metrics = StepMetrics {
            run_tick_ms,
            digest_build_ms,
            #[cfg(target_arch = "wasm32")]
            digest_phases,
            #[cfg(target_arch = "wasm32")]
            engine_phases,
        };
        Ok(StepOutcome { digest, metrics, was_reset: false })
    }
}

#[wasm_bindgen]
pub struct WasmMirror {
    mirror: MirrorHandle,
    running: Cell<bool>,
    tick_interval_ms: Cell<u32>,
    engine: RefCell<Option<EngineCtx>>,
}

#[wasm_bindgen]
impl WasmMirror {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmMirror {
        init_panic_hook();
        WasmMirror {
            mirror: MirrorHandle::new(),
            running: Cell::new(false),
            tick_interval_ms: Cell::new(1_000),
            engine: RefCell::new(None),
        }
    }

    #[wasm_bindgen]
    pub fn greeting(&self) -> String {
        format!("Hello from sim-wasm! Current tick: {}", self.latest_tick())
    }

    #[wasm_bindgen]
    pub fn initialize(&self, scenario_toml: &str) -> Result<JsValue, JsValue> {
        let scenario = Scenario::from_toml_str(scenario_toml)
            .map_err(|err| JsValue::from_str(&format!("failed to parse scenario: {err}")))?;
        let ctx = EngineCtx::new(scenario);
        let digest = ctx.snapshot();
        let js_value = self.publish_snapshot(digest.clone())?;
        *self.engine.borrow_mut() = Some(ctx);
        self.running.set(false);
        self.tick_interval_ms.set(50);
        Ok(js_value)
    }

    #[wasm_bindgen]
    pub fn pause(&self) {
        self.running.set(false);
    }

    #[wasm_bindgen]
    pub fn resume(&self) {
        self.running.set(true);
    }

    #[wasm_bindgen]
    pub fn is_running(&self) -> bool {
        self.running.get()
    }

    #[wasm_bindgen]
    pub fn set_tick_interval_ms(&self, interval: u32) {
        let clamped = interval.max(1);
        self.tick_interval_ms.set(clamped);
    }

    #[wasm_bindgen]
    pub fn tick_interval_ms(&self) -> u32 {
        self.tick_interval_ms.get()
    }

    #[wasm_bindgen]
    pub fn reset(&self) -> Result<JsValue, JsValue> {
        let mut guard = self.engine.borrow_mut();
        let ctx = guard.as_mut().ok_or_else(|| JsValue::from_str("simulation not initialised"))?;
        let digest = ctx.reset();
        self.running.set(false);
        self.publish_snapshot(digest)
    }

    #[wasm_bindgen]
    pub fn step(&self) -> Result<JsValue, JsValue> {
        let mut guard = self.engine.borrow_mut();
        let ctx = guard.as_mut().ok_or_else(|| JsValue::from_str("simulation not initialised"))?;

        let outcome = ctx.step().map_err(|err| JsValue::from_str(&err))?;

        let mut digest = outcome.digest;
        digest.timings.generated_at = Utc::now();
        self.log_step_metrics(digest.tick, &outcome.metrics, &digest.timings, outcome.was_reset);
        self.publish_snapshot(digest)
    }

    #[wasm_bindgen]
    pub fn latest_tick(&self) -> u32 {
        self.mirror.latest().digest.tick
    }
    #[wasm_bindgen]
    pub fn arrow_ex(&self) -> Result<JsValue, JsValue> {
        let payload =
            get_arrow_pl().map_err(|err| JsValue::from_str(&format!("failed to build arrow payload: {err}")))?;

        #[cfg(target_arch = "wasm32")]
        {
            Ok(Uint8Array::from(payload.as_slice()).into())
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            to_value(&payload).map_err(|err| JsValue::from_str(&format!("failed to convert arrow payload: {err}")))
        }
    }
    #[wasm_bindgen]
    pub fn latest_snapshot(&self) -> Result<JsValue, JsValue> {
        let snapshot = self.mirror.latest();
        encode_snapshot(snapshot.as_ref())
    }

    #[wasm_bindgen]
    pub fn status(&self) -> Result<JsValue, JsValue> {
        let status = MirrorStatus { running: self.running.get(), tick_interval_ms: self.tick_interval_ms.get() };
        to_value(&status).map_err(|err| JsValue::from_str(&err.to_string()))
    }

    #[wasm_bindgen]
    pub fn behaviour_history(&self, limit: u32) -> Result<JsValue, JsValue> {
        let guard = self.engine.borrow();
        let ctx = guard.as_ref().ok_or_else(|| JsValue::from_str("simulation not initialised"))?;
        let limit = limit.max(1).min(BEHAVIOUR_HISTORY_EXPORT_MAX as u32) as usize;
        let digest = build_behaviour_history(&ctx.engine, limit);
        encode_payload(&digest, "behaviour history")
    }

    #[wasm_bindgen]
    pub fn agent_catalogue(&self) -> Result<JsValue, JsValue> {
        let snapshot = self.mirror.latest();
        let catalogue =
            snapshot.digest.agents.catalogue.clone().ok_or_else(|| JsValue::from_str("agent catalogue unavailable"))?;
        encode_payload(&catalogue, "agent catalogue")
    }

    fn publish_snapshot(&self, digest: StateDigest) -> Result<JsValue, JsValue> {
        let snapshot = self.mirror.publish(digest);
        encode_snapshot(snapshot.as_ref())
    }

    fn log_step_metrics(&self, tick: u32, metrics: &StepMetrics, timings: &DigestTimings, was_reset: bool) {
        #[cfg(target_arch = "wasm32")]
        {
            let label = if was_reset { "reset" } else { "step" };
            let message = format!(
                "[sim-wasm] {label} tick={tick} run_tick={:.2}ms digest={:.2}ms total={:.2}ms",
                metrics.run_tick_ms,
                metrics.digest_build_ms,
                metrics.run_tick_ms + metrics.digest_build_ms
            );
            let digest_details = metrics
                .digest_phases
                .iter()
                .map(|phase| format!("{}={:.2}ms", phase.phase, phase.duration_ms))
                .collect::<Vec<_>>()
                .join(", ");
            let engine_details = metrics
                .engine_phases
                .iter()
                .map(|phase| format!("{}={:.2}ms", phase.step, phase.duration_ms))
                .collect::<Vec<_>>()
                .join(", ");

            let mut extended = message;
            if !digest_details.is_empty() {
                extended = format!("{extended} digest=[{digest_details}]");
            }
            if !engine_details.is_empty() {
                extended = format!("{extended} engine=[{engine_details}]");
            }

            web_sys::console::log_1(&JsValue::from_str(&extended));
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (tick, metrics.run_tick_ms, metrics.digest_build_ms, timings, was_reset);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn collect_engine_phases(result: &TickExecutionResult) -> Vec<EnginePhaseTiming> {
    let mut phases: Vec<EnginePhaseTiming> = TickStep::all()
        .into_iter()
        .filter_map(|step| {
            result.step_results.get(&step).map(|step_result| EnginePhaseTiming {
                step: tick_step_label(step).to_string(),
                duration_ms: step_result.duration_ms as f64,
            })
        })
        .collect();

    phases.sort_by(|a, b| b.duration_ms.partial_cmp(&a.duration_ms).unwrap_or(std::cmp::Ordering::Equal));
    phases
}

#[cfg(target_arch = "wasm32")]
fn tick_step_label(step: TickStep) -> &'static str {
    use TickStep::*;
    match step {
        Upkeep => "upkeep",
        GatherIntentions => "gather_intentions",
        ResolveIndependentPhase => "resolve_independent",
        ResolveMarketPhase => "resolve_market",
        ApplyMarketEffectsForPriceDiscovery => "apply_market_effects",
        ResolveDependentPhase => "resolve_dependent",
        Auction => "auction",
        ClearMarkets => "clear_markets",
        ClearOvernightMarkets => "clear_overnight",
        SettleTrades => "settle_trades",
        ServiceDeposits => "service_deposits",
        ServiceGovernmentDebt => "service_government_debt",
        ServiceCredit => "service_credit",
        ApplyPaymentQueuing => "apply_payment_queuing",
        RunRTGS => "run_rtgs",
        ReconcileCredit => "reconcile_credit",
        ApplyAllEffects => "apply_all_effects",
        UpdateHistory => "update_history",
    }
}

fn get_arrow_pl() -> ArrowResult<Vec<u8>> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("agent_id", DataType::Utf8, false),
        Field::new("assets", DataType::Float32, false),
        Field::new("liabilities", DataType::Float32, false),
        Field::new("net_worth", DataType::Float32, false),
    ]));

    let agent_ids = StringArray::from(vec!["agent_1", "agent_2", "agent_3"]);
    let asset_values = vec![1000.0_f32, 1500.5_f32, 2000.75_f32];
    let liability_values = vec![200.0_f32, 300.0_f32, 400.0_f32];
    let net_worth_values = asset_values.iter().zip(liability_values.iter()).map(|(a, l)| a - l).collect::<Vec<_>>();

    let assets = Float32Array::from(asset_values);
    let liabilities = Float32Array::from(liability_values);
    let net_worth = Float32Array::from(net_worth_values);

    let columns: Vec<ArrayRef> =
        vec![Arc::new(agent_ids), Arc::new(assets), Arc::new(liabilities), Arc::new(net_worth)];
    let batch = RecordBatch::try_new(schema.clone(), columns)?;
    let mut buffer = Vec::new();
    {
        let mut writer = StreamWriter::try_new(&mut buffer, schema.as_ref())?;
        writer.write(&batch)?;
        writer.finish()?;
    }

    Ok(buffer)
}

#[cfg(target_arch = "wasm32")]
fn encode_snapshot(snapshot: &StateSnapshot) -> Result<JsValue, JsValue> {
    encode_payload(snapshot, "snapshot")
}

#[cfg(not(target_arch = "wasm32"))]
fn encode_snapshot(snapshot: &StateSnapshot) -> Result<JsValue, JsValue> {
    encode_payload(snapshot, "snapshot")
}

#[cfg(target_arch = "wasm32")]
fn encode_payload<T: Serialize>(value: &T, context: &str) -> Result<JsValue, JsValue> {
    let bytes = to_vec_named(value).map_err(|err| JsValue::from_str(&format!("failed to encode {context}: {err}")))?;
    Ok(Uint8Array::from(bytes.as_slice()).into())
}

#[cfg(not(target_arch = "wasm32"))]
fn encode_payload<T: Serialize>(value: &T, context: &str) -> Result<JsValue, JsValue> {
    to_value(value).map_err(|err| JsValue::from_str(&format!("failed to encode {context}: {err}")))
}

fn build_behaviour_history(engine: &SimulationEngine, limit: usize) -> BehaviourDigest {
    let limit = limit.max(1).min(BEHAVIOUR_HISTORY_EXPORT_MAX);
    let history = &engine.state.history;
    let recent_ticks = history.get_recent_ticks(limit);
    let detail_limit = BEHAVIOUR_DETAIL_EXPORT_LIMIT.min(recent_ticks.len());
    let detail_threshold = recent_ticks.len().saturating_sub(detail_limit);

    let ticks = recent_ticks
        .iter()
        .enumerate()
        .map(|(idx, record)| behaviour_tick_from_record(record, idx >= detail_threshold))
        .collect();

    let mut recent_events = Vec::new();
    for record in recent_ticks.iter().rev() {
        for event in record.events.iter().rev() {
            if recent_events.len() >= BEHAVIOUR_EVENT_EXPORT_LIMIT {
                break;
            }
            recent_events.push(event.clone());
        }
        if recent_events.len() >= BEHAVIOUR_EVENT_EXPORT_LIMIT {
            break;
        }
    }
    recent_events.reverse();

    BehaviourDigest { ticks, recent_events }
}

fn behaviour_tick_from_record(record: &TickRecord, include_detail: bool) -> BehaviourTickDigest {
    let summary = TickEventSummary::from_events(&record.events);
    let detail = if include_detail {
        Some(TickDetailDigest {
            intentions: record.intentions.clone(),
            actions: record.actions.clone(),
            effects: record.effects.clone(),
            events: record.events.clone(),
            action_to_effect_indices: record.action_to_effect_indices.clone(),
            trades: record.trades.clone(),
        })
    } else {
        None
    };

    BehaviourTickDigest {
        tick: record.tick_number,
        date: record.date.format("%Y-%m-%d").to_string(),
        summary,
        intention_count: record.intentions.len(),
        action_count: record.actions.len(),
        effect_count: record.effects.len(),
        detail,
    }
}

#[wasm_bindgen]
pub fn hello_world() -> String {
    "Hello from sim-wasm".to_string()
}

#[cfg(target_arch = "wasm32")]
fn init_panic_hook() {
    static INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    INIT.get_or_init(|| {
        console_error_panic_hook::set_once();
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn init_panic_hook() {}

#[cfg(target_arch = "wasm32")]
fn prune_history(engine: &mut SimulationEngine) {
    const MAX_TICK_RECORDS: usize = 128;
    const MAX_TRANSACTIONS: usize = 1_000;
    const MAX_MARKET_TICKS: usize = 64;

    let history = &mut engine.state.history;

    while history.tick_records.len() > MAX_TICK_RECORDS {
        history.tick_records.pop_front();
    }

    if history.transactions.len() > MAX_TRANSACTIONS {
        let remove = history.transactions.len() - MAX_TRANSACTIONS;
        history.transactions.drain(0..remove.min(history.transactions.len()));
    }

    for ticks in history.market_ticks.values_mut() {
        while ticks.len() > MAX_MARKET_TICKS {
            ticks.pop_front();
        }
    }

    history.tick_records.shrink_to_fit();
    history.transactions.shrink_to_fit();
    history.market_ticks.shrink_to_fit();
}

#[cfg(not(target_arch = "wasm32"))]
fn prune_history(_engine: &mut SimulationEngine) {}
