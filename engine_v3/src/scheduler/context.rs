// engine_v3/src/scheduler/context.rs
use super::{StepResult, TickStep};
use domains::ResolutionPhase;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::collections::HashMap;

// Typed keys to prevent typo bugs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StepKey {
    CategorizedIntentions,
    Intentions,
    AllActions,
    AllEffects,
    ActionToEffectIdx,
    Trades,
    MarketSnapshots,
}

#[derive(Debug)]
pub struct StepContext {
    pub tick_number: u32,
    pub step_data: HashMap<TickStep, StepResult>,
    pub shared_data: HashMap<StepKey, serde_json::Value>, // Typed keys instead of strings
}

impl StepContext {
    pub fn new(tick_number: u32) -> Self {
        Self {
            tick_number,
            step_data: HashMap::new(),
            shared_data: HashMap::new(),
        }
    }

    // Typed store method
    pub fn store_typed<T: Serialize>(&mut self, key: StepKey, data: &T) -> Result<(), String> {
        let value = serde_json::to_value(data)
            .map_err(|e| format!("Failed to serialize data for key {:?}: {}", key, e))?;
        self.shared_data.insert(key, value);
        Ok(())
    }

    // Typed get method  
    pub fn get_typed<T: for<'de> Deserialize<'de>>(&self, key: StepKey) -> Result<T, String> {
        let value = self
            .shared_data
            .get(&key)
            .ok_or_else(|| format!("No data found for key: {:?}", key))?;
        serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to deserialize data for key {:?}: {}", key, e))
    }

    // Legacy store method (for backwards compat during migration)
    pub fn store<T: Serialize>(&mut self, key: &str, data: &T) -> Result<(), String> {
        let step_key = match key {
            "categorized_intentions" => StepKey::CategorizedIntentions,
            "intentions" => StepKey::Intentions,
            "all_actions" => StepKey::AllActions,
            "all_effects" => StepKey::AllEffects,
            "action_to_effect_indices" => StepKey::ActionToEffectIdx,
            "trades" => StepKey::Trades,
            "market_snapshots" => StepKey::MarketSnapshots,
            _ => return Err(format!("Unknown legacy key: {}", key)),
        };
        self.store_typed(step_key, data)
    }

    // Legacy get method (for backwards compat during migration)
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, String> {
        let step_key = match key {
            "categorized_intentions" => StepKey::CategorizedIntentions,
            "intentions" => StepKey::Intentions,
            "all_actions" => StepKey::AllActions,
            "all_effects" => StepKey::AllEffects,
            "action_to_effect_indices" => StepKey::ActionToEffectIdx,
            "trades" => StepKey::Trades,
            "market_snapshots" => StepKey::MarketSnapshots,
            _ => return Err(format!("Unknown legacy key: {}", key)),
        };
        self.get_typed(step_key)
    }

    // Convenience methods with typed keys
    pub fn get_categorized_intentions(&self) -> Result<HashMap<ResolutionPhase, Vec<SimIntention>>, String> {
        // Handle the string key to ResolutionPhase conversion
        let string_map: HashMap<String, Vec<SimIntention>> = self.get_typed(StepKey::CategorizedIntentions)?;
        string_map
            .into_iter()
            .map(|(key, value)| {
                let phase = match key.as_str() {
                    "Independent" => Ok(ResolutionPhase::Independent),
                    "Market" => Ok(ResolutionPhase::Market),
                    "Dependent" => Ok(ResolutionPhase::Dependent),
                    _ => Err(format!("Unknown ResolutionPhase key: {}", key)),
                }?;
                Ok((phase, value))
            })
            .collect()
    }

    pub fn get_intentions(&self) -> Result<Vec<SimIntention>, String> {
        self.get_typed(StepKey::Intentions)
    }

    pub fn get_all_actions(&self) -> Result<Vec<ActionRecord>, String> {
        self.get_typed(StepKey::AllActions)
    }

    pub fn get_all_effects(&self) -> Result<Vec<StateEffect>, String> {
        self.get_typed(StepKey::AllEffects)
    }

    pub fn get_action_to_effect_indices(&self) -> Result<HashMap<usize, Vec<usize>>, String> {
        self.get_typed(StepKey::ActionToEffectIdx)
    }

    pub fn get_trades(&self) -> Result<Vec<Trade>, String> {
        self.get_typed(StepKey::Trades)
    }

    pub fn get_market_snapshots(&self) -> Result<HashMap<MarketId, MarketView>, String> {
        use std::str::FromStr;
        // Handle string key to MarketId conversion
        let string_map: HashMap<String, MarketView> = self.get_typed(StepKey::MarketSnapshots)?;
        string_map
            .into_iter()
            .map(|(k, v)| {
                MarketId::from_str(&k)
                    .map(|id| (id, v))
                    .map_err(|_| format!("Invalid MarketId in market_snapshots: {}", k))
            })
            .collect()
    }
}