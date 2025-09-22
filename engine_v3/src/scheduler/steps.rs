use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TickStep {
    Upkeep,
    GatherIntentions,
    ResolveIndependentPhase,
    ResolveMarketPhase,
    ApplyMarketEffectsForPriceDiscovery,
    ResolveDependentPhase,
    Auction,
    ClearMarkets,
    ClearOvernightMarkets,
    SettleTrades,
    ServiceDeposits,
    ServiceGovernmentDebt,
    ServiceCredit,
    ApplyPaymentQueuing,
    RunRTGS,
    ReconcileCredit,
    ApplyAllEffects,
    UpdateHistory,
}

impl TickStep {
    pub fn all() -> Vec<Self> {
        use TickStep::*;
        vec![
            Upkeep,
            GatherIntentions,
            ResolveIndependentPhase,
            ResolveMarketPhase,
            ApplyMarketEffectsForPriceDiscovery,
            ResolveDependentPhase,
            Auction,
            ClearMarkets,
            ClearOvernightMarkets,
            SettleTrades,
            ServiceDeposits,
            ServiceGovernmentDebt,
            ServiceCredit,
            ApplyPaymentQueuing,
            RunRTGS,
            ReconcileCredit,
            ApplyAllEffects,
            UpdateHistory,
        ]
    }

    pub fn dependencies(&self) -> Vec<Self> {
        use TickStep::*;
        match self {
            Upkeep => vec![],
            GatherIntentions => vec![Upkeep],
            ResolveIndependentPhase => vec![GatherIntentions],
            ResolveMarketPhase => vec![ResolveIndependentPhase],
            ApplyMarketEffectsForPriceDiscovery => vec![ResolveMarketPhase],
            ResolveDependentPhase => vec![ApplyMarketEffectsForPriceDiscovery],
            Auction => vec![ResolveDependentPhase],
            ClearMarkets => vec![Auction],
            ClearOvernightMarkets => vec![ClearMarkets],
            SettleTrades => vec![ClearMarkets],
            ServiceDeposits => vec![SettleTrades],
            ServiceGovernmentDebt => vec![SettleTrades],
            ServiceCredit => vec![ServiceDeposits, ServiceGovernmentDebt],
            ApplyPaymentQueuing => vec![ServiceCredit],
            RunRTGS => vec![ApplyPaymentQueuing],
            ReconcileCredit => vec![RunRTGS],
            ApplyAllEffects => vec![ReconcileCredit],
            UpdateHistory => vec![ApplyAllEffects],
        }
    }
    pub fn should_abort_on_failure(&self) -> bool {
        true
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub success: bool,
    pub duration_ms: u64,
    pub telemetry: StepTelemetry,
    pub error: Option<String>,
}

impl StepResult {
    pub fn success(duration_ms: u64, telemetry: StepTelemetry) -> Self {
        Self { success: true, duration_ms, telemetry, error: None }
    }
    pub fn failure(duration_ms: u64, error: String) -> Self {
        Self { success: false, duration_ms, telemetry: StepTelemetry::default(), error: Some(error) }
    }

    pub fn record_metrics(&self, step: TickStep) {
        metrics::histogram!(duration_metric_name(step), self.duration_ms as f64);
        if !self.success {
            metrics::counter!(failure_metric_name(step), 1u64);
        }

        self.telemetry.emit_metrics(step);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StepTelemetry {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metrics: Vec<StepMetric>,
}

impl StepTelemetry {
    pub fn new() -> Self {
        Self { metrics: Vec::new() }
    }

    pub fn single<K, V>(key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<StepMetricValue>,
    {
        Self { metrics: vec![StepMetric::new(key, value)] }
    }

    pub fn with_metric<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<StepMetricValue>,
    {
        self.push_metric(key, value);
        self
    }

    pub fn push_metric<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<StepMetricValue>,
    {
        self.metrics.push(StepMetric::new(key, value));
    }

    pub fn emit_metrics(&self, step: TickStep) {
        for metric in &self.metrics {
            if let Some(metric_name) = telemetry_metric_name(step, metric.key.as_str()) {
                match &metric.value {
                    StepMetricValue::Integer(v) => {
                        metrics::gauge!(metric_name, *v as f64);
                    }
                    StepMetricValue::Float(v) => {
                        metrics::gauge!(metric_name, *v);
                    }
                    StepMetricValue::Bool(v) => {
                        let value = if *v { 1.0 } else { 0.0 };
                        metrics::gauge!(metric_name, value);
                    }
                    StepMetricValue::Text(_) => {
                        // textual metrics are not exported via metrics crate
                    }
                }
            }
        }
    }
}

fn duration_metric_name(step: TickStep) -> &'static str {
    use TickStep::*;
    match step {
        Upkeep => "engine.step.upkeep.duration_ms",
        GatherIntentions => "engine.step.gather_intentions.duration_ms",
        ResolveIndependentPhase => "engine.step.resolve_independent.duration_ms",
        ResolveMarketPhase => "engine.step.resolve_market.duration_ms",
        ApplyMarketEffectsForPriceDiscovery => "engine.step.apply_market_effects.duration_ms",
        ResolveDependentPhase => "engine.step.resolve_dependent.duration_ms",
        Auction => "engine.step.auction.duration_ms",
        ClearMarkets => "engine.step.clear_markets.duration_ms",
        ClearOvernightMarkets => "engine.step.clear_overnight.duration_ms",
        SettleTrades => "engine.step.settle_trades.duration_ms",
        ServiceDeposits => "engine.step.service_deposits.duration_ms",
        ServiceGovernmentDebt => "engine.step.service_government_debt.duration_ms",
        ServiceCredit => "engine.step.service_credit.duration_ms",
        ApplyPaymentQueuing => "engine.step.apply_payment_queuing.duration_ms",
        RunRTGS => "engine.step.run_rtgs.duration_ms",
        ReconcileCredit => "engine.step.reconcile_credit.duration_ms",
        ApplyAllEffects => "engine.step.apply_all_effects.duration_ms",
        UpdateHistory => "engine.step.update_history.duration_ms",
    }
}

fn failure_metric_name(step: TickStep) -> &'static str {
    use TickStep::*;
    match step {
        Upkeep => "engine.step.upkeep.failures_total",
        GatherIntentions => "engine.step.gather_intentions.failures_total",
        ResolveIndependentPhase => "engine.step.resolve_independent.failures_total",
        ResolveMarketPhase => "engine.step.resolve_market.failures_total",
        ApplyMarketEffectsForPriceDiscovery => "engine.step.apply_market_effects.failures_total",
        ResolveDependentPhase => "engine.step.resolve_dependent.failures_total",
        Auction => "engine.step.auction.failures_total",
        ClearMarkets => "engine.step.clear_markets.failures_total",
        ClearOvernightMarkets => "engine.step.clear_overnight.failures_total",
        SettleTrades => "engine.step.settle_trades.failures_total",
        ServiceDeposits => "engine.step.service_deposits.failures_total",
        ServiceGovernmentDebt => "engine.step.service_government_debt.failures_total",
        ServiceCredit => "engine.step.service_credit.failures_total",
        ApplyPaymentQueuing => "engine.step.apply_payment_queuing.failures_total",
        RunRTGS => "engine.step.run_rtgs.failures_total",
        ReconcileCredit => "engine.step.reconcile_credit.failures_total",
        ApplyAllEffects => "engine.step.apply_all_effects.failures_total",
        UpdateHistory => "engine.step.update_history.failures_total",
    }
}

fn telemetry_metric_name(step: TickStep, metric: &str) -> Option<&'static str> {
    use TickStep::*;
    match (step, metric) {
        (GatherIntentions, "total_intentions") => Some("engine.step.gather_intentions.total_intentions"),
        (ResolveIndependentPhase, "actions") => Some("engine.step.resolve_independent.actions"),
        (ResolveIndependentPhase, "effects") => Some("engine.step.resolve_independent.effects"),
        (ResolveMarketPhase, "actions") => Some("engine.step.resolve_market.actions"),
        (ResolveMarketPhase, "effects") => Some("engine.step.resolve_market.effects"),
        (ResolveDependentPhase, "actions") => Some("engine.step.resolve_dependent.actions"),
        (ResolveDependentPhase, "effects") => Some("engine.step.resolve_dependent.effects"),
        (ApplyMarketEffectsForPriceDiscovery, "market_effects_applied") => {
            Some("engine.step.apply_market_effects.market_effects_applied")
        }
        (ClearMarkets, "trades_generated") => Some("engine.step.clear_markets.trades_generated"),
        (ServiceGovernmentDebt, "payments_generated") => Some("engine.step.service_government_debt.payments_generated"),
        (SettleTrades, "trades_processed") => Some("engine.step.settle_trades.trades_processed"),
        (SettleTrades, "settlement_effects") => Some("engine.step.settle_trades.settlement_effects"),
        (ReconcileCredit, "credit_effects") => Some("engine.step.reconcile_credit.credit_effects"),
        (ServiceCredit, "serviced_loans") => Some("engine.step.service_credit.serviced_loans"),
        (ServiceDeposits, "payments_generated") => Some("engine.step.service_deposits.payments_generated"),
        (ApplyPaymentQueuing, "payments_and_settlements_queued") => {
            Some("engine.step.apply_payment_queuing.payments_and_settlements_queued")
        }
        (RunRTGS, "payments_settled") => Some("engine.step.run_rtgs.payments_settled"),
        (RunRTGS, "payments_remaining") => Some("engine.step.run_rtgs.payments_remaining"),
        (ApplyAllEffects, "total_effects_applied") => Some("engine.step.apply_all_effects.total_effects_applied"),
        (Auction, "auctions_processed") => Some("engine.step.auction.auctions_processed"),
        (ClearOvernightMarkets, "fedfunds_cleared") => Some("engine.step.clear_overnight.fedfunds_cleared"),
        (ClearOvernightMarkets, "repos_cleared") => Some("engine.step.clear_overnight.repos_cleared"),
        (ServiceGovernmentDebt, _) => None,
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepMetric {
    pub key: String,
    pub value: StepMetricValue,
}

impl StepMetric {
    pub fn new<K, V>(key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<StepMetricValue>,
    {
        Self { key: key.into(), value: value.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum StepMetricValue {
    Integer(i64),
    Float(f64),
    Bool(bool),
    Text(String),
}

impl From<usize> for StepMetricValue {
    fn from(value: usize) -> Self {
        StepMetricValue::Integer(value as i64)
    }
}

impl From<i64> for StepMetricValue {
    fn from(value: i64) -> Self {
        StepMetricValue::Integer(value)
    }
}

impl From<u64> for StepMetricValue {
    fn from(value: u64) -> Self {
        StepMetricValue::Integer(value as i64)
    }
}

impl From<u32> for StepMetricValue {
    fn from(value: u32) -> Self {
        StepMetricValue::Integer(value as i64)
    }
}

impl From<i32> for StepMetricValue {
    fn from(value: i32) -> Self {
        StepMetricValue::Integer(value as i64)
    }
}

impl From<f64> for StepMetricValue {
    fn from(value: f64) -> Self {
        StepMetricValue::Float(value)
    }
}

impl From<bool> for StepMetricValue {
    fn from(value: bool) -> Self {
        StepMetricValue::Bool(value)
    }
}

impl From<String> for StepMetricValue {
    fn from(value: String) -> Self {
        StepMetricValue::Text(value)
    }
}

impl From<&str> for StepMetricValue {
    fn from(value: &str) -> Self {
        StepMetricValue::Text(value.to_string())
    }
}

pub trait StepHandler: Send + Sync + Debug {
    fn execute(
        &self, engine: &mut crate::executor::SimulationEngine, context: &mut super::StepContext,
        rng: &mut dyn rand::RngCore,
    ) -> StepResult;
}
