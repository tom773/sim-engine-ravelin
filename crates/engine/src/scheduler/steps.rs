use serde::{Serialize, Deserialize};
use std::fmt;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TickStep {
    Upkeep,
    GatherIntentions,
    ResolveIndependentPhase,
    ResolveMarketPhase,
    ApplyMarketEffectsForPriceDiscovery,
    ResolveDependentPhase,
    ClearMarkets,
    SettleTrades,
    ApplyAllEffects,
    UpdateHistory,
    PersistData,
}

impl TickStep {
    /// Returns all steps in dependency order
    pub fn all() -> Vec<Self> {
        use TickStep::*;
        vec![
            Upkeep,
            GatherIntentions,
            ResolveIndependentPhase,
            ResolveMarketPhase,
            ApplyMarketEffectsForPriceDiscovery,
            ResolveDependentPhase,
            ClearMarkets,
            SettleTrades,
            ApplyAllEffects,
            UpdateHistory,
            PersistData,
        ]
    }

    /// Returns the dependencies for this step
    pub fn dependencies(&self) -> Vec<Self> {
        use TickStep::*;
        match self {
            Upkeep => vec![],
            GatherIntentions => vec![Upkeep],
            ResolveIndependentPhase => vec![GatherIntentions],
            ResolveMarketPhase => vec![ResolveIndependentPhase],
            ApplyMarketEffectsForPriceDiscovery => vec![ResolveMarketPhase],
            ResolveDependentPhase => vec![ApplyMarketEffectsForPriceDiscovery],
            ClearMarkets => vec![ResolveDependentPhase],
            SettleTrades => vec![ClearMarkets],
            ApplyAllEffects => vec![SettleTrades],
            UpdateHistory => vec![ApplyAllEffects],
            PersistData => vec![UpdateHistory],
        }
    }

    /// Returns whether this step should abort the tick on failure
    pub fn should_abort_on_failure(&self) -> bool {
        use TickStep::*;
        match self {
            // Critical steps that should abort the tick
            Upkeep | ApplyAllEffects => true,
            // Steps that can fail without aborting
            PersistData => false,
            // Most steps should abort by default
            _ => true,
        }
    }

    /// Returns whether this step can potentially be run in parallel with others
    pub fn can_run_parallel(&self) -> bool {
        use TickStep::*;
        match self {
            // Steps that need exclusive access to the engine
            Upkeep | ApplyAllEffects | UpdateHistory => false,
            // Steps that could potentially be parallelized (future enhancement)
            GatherIntentions | ResolveIndependentPhase | ResolveMarketPhase 
            | ResolveDependentPhase | ClearMarkets | SettleTrades => false, // Conservative for now
            // Safe parallel steps
            ApplyMarketEffectsForPriceDiscovery | PersistData => true,
        }
    }
}

impl fmt::Display for TickStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
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
        Self {
            success: true,
            duration_ms,
            metadata,
            error: None,
        }
    }

    pub fn failure(duration_ms: u64, error: String) -> Self {
        Self {
            success: false,
            duration_ms,
            metadata: serde_json::Value::Null,
            error: Some(error),
        }
    }

    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        if let serde_json::Value::Object(ref mut map) = self.metadata {
            map.insert(key.to_string(), value);
        } else {
            let mut map = serde_json::Map::new();
            map.insert(key.to_string(), value);
            self.metadata = serde_json::Value::Object(map);
        }
        self
    }
}

/// Trait for step handlers that execute individual tick steps
pub trait StepHandler: Send + Sync {
    /// Execute the step with the given engine, context, and RNG
    fn execute(
        &self,
        engine: &mut crate::SimulationEngine,
        context: &mut super::StepContext,
        rng: &mut dyn rand::RngCore,
    ) -> StepResult;

    /// Whether this step can run in parallel with other steps
    fn can_run_parallel(&self) -> bool {
        false
    }

    /// Validate preconditions before running this step
    fn validates_preconditions(&self, _context: &super::StepContext) -> Result<(), String> {
        Ok(())
    }

    /// Get the name of this handler for debugging
    fn name(&self) -> &'static str {
        "UnnamedHandler"
    }
}