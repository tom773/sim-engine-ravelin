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
    BuildSettlementObligations,
    RunRTGS,
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
            BuildSettlementObligations,
            RunRTGS,
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
        BuildSettlementObligations => vec![ClearMarkets],
        
        ApplyAllEffects => vec![BuildSettlementObligations],
        RunRTGS => vec![ApplyAllEffects],
        UpdateHistory => vec![RunRTGS],
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
    pub metadata: serde_json::Value,
    pub error: Option<String>,
}

impl StepResult {
    pub fn success(duration_ms: u64, metadata: serde_json::Value) -> Self {
        Self { success: true, duration_ms, metadata, error: None }
    }
    pub fn failure(duration_ms: u64, error: String) -> Self {
        Self { success: false, duration_ms, metadata: serde_json::Value::Null, error: Some(error) }
    }
}

pub trait StepHandler: Send + Sync + Debug {
    fn execute(
        &self, engine: &mut crate::executor::SimulationEngine, context: &mut super::StepContext,
        rng: &mut dyn rand::RngCore,
    ) -> StepResult;
}