use super::{StepResult, TickStep};
use domains::ResolutionPhase;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct StepContext {
    pub tick_number: u32,
    pub step_data: HashMap<TickStep, StepResult>,
    pub shared_data: HashMap<String, serde_json::Value>,
}

impl StepContext {
    pub fn new(tick_number: u32) -> Self {
        Self {
            tick_number,
            step_data: HashMap::new(),
            shared_data: HashMap::new(),
        }
    }

    pub fn store<T: Serialize>(&mut self, key: &str, data: &T) -> Result<(), String> {
        let value = serde_json::to_value(data)
            .map_err(|e| format!("Failed to serialize data for key {}: {}", key, e))?;
        self.shared_data.insert(key.to_string(), value);
        Ok(())
    }

    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, String> {
        let value = self
            .shared_data
            .get(key)
            .ok_or_else(|| format!("No data found for key: {}", key))?;
        serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to deserialize data for key {}: {}", key, e))
    }

    pub fn get_categorized_intentions(
        &self,
    ) -> Result<HashMap<ResolutionPhase, Vec<SimIntention>>, String> {
        let string_map: HashMap<String, Vec<SimIntention>> = self.get("categorized_intentions")?;

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
        self.get("intentions")
    }
    pub fn get_all_actions(&self) -> Result<Vec<ActionRecord>, String> {
        self.get("all_actions")
    }
    pub fn get_all_effects(&self) -> Result<Vec<StateEffect>, String> {
        self.get("all_effects")
    }
    pub fn get_action_to_effect_indices(&self) -> Result<HashMap<usize, Vec<usize>>, String> {
        self.get("action_to_effect_indices")
    }
    pub fn get_trades(&self) -> Result<Vec<Trade>, String> {
        self.get("trades")
    }
    pub fn get_market_snapshots(&self) -> Result<HashMap<MarketId, MarketView>, String> {
        use std::str::FromStr;
        let string_map: HashMap<String, MarketView> = self.get("market_snapshots")?;
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
