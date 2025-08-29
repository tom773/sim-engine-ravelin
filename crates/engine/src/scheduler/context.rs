use super::{TickStep, StepResult};
use sim_core::*;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

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

    pub fn store_intentions(&mut self, intentions: Vec<SimIntention>) -> Result<(), String> {
        let value = serde_json::to_value(intentions)
            .map_err(|e| format!("Failed to serialize intentions: {}", e))?;
        self.shared_data.insert("intentions".to_string(), value);
        Ok(())
    }

    pub fn get_intentions(&self) -> Result<Vec<SimIntention>, String> {
        self.shared_data
            .get("intentions")
            .ok_or("No intentions found in context")?
            .as_array()
            .ok_or("Intentions not stored as array")?
            .iter()
            .map(|v| serde_json::from_value(v.clone())
                .map_err(|e| format!("Failed to deserialize intention: {}", e)))
            .collect()
    }

    pub fn store_categorized_intentions(&mut self, categorized: HashMap<domains::ResolutionPhase, Vec<SimIntention>>) -> Result<(), String> {
        let value = serde_json::to_value(categorized)
            .map_err(|e| format!("Failed to serialize categorized intentions: {}", e))?;
        self.shared_data.insert("categorized_intentions".to_string(), value);
        Ok(())
    }

    pub fn get_intentions_for_phase(&self, phase: domains::ResolutionPhase) -> Result<Vec<SimIntention>, String> {
        let categorized: HashMap<String, Vec<SimIntention>> = self.shared_data
            .get("categorized_intentions")
            .ok_or("No categorized intentions found")?
            .as_object()
            .ok_or("Categorized intentions not stored as object")?
            .iter()
            .map(|(k, v)| {
                let intentions: Vec<SimIntention> = serde_json::from_value(v.clone())
                    .map_err(|e| format!("Failed to deserialize intentions for phase {}: {}", k, e))?;
                Ok((k.clone(), intentions))
            })
            .collect::<Result<HashMap<_, _>, String>>()?;

        let phase_key = format!("{:?}", phase);
        Ok(categorized.get(&phase_key).cloned().unwrap_or_default())
    }

    pub fn store_phase_actions(&mut self, phase: domains::ResolutionPhase, actions: Vec<ActionRecord>) -> Result<(), String> {
        let key = format!("actions_{:?}", phase);
        let value = serde_json::to_value(actions)
            .map_err(|e| format!("Failed to serialize actions for phase {:?}: {}", phase, e))?;
        self.shared_data.insert(key, value);
        Ok(())
    }

    pub fn get_phase_actions(&self, phase: domains::ResolutionPhase) -> Result<Vec<ActionRecord>, String> {
        let key = format!("actions_{:?}", phase);
        self.shared_data
            .get(&key)
            .ok_or_else(|| format!("No actions found for phase {:?}", phase))?
            .as_array()
            .ok_or("Actions not stored as array")?
            .iter()
            .map(|v| serde_json::from_value(v.clone())
                .map_err(|e| format!("Failed to deserialize action: {}", e)))
            .collect()
    }

    pub fn store_phase_effects(&mut self, phase: domains::ResolutionPhase, effects: Vec<StateEffect>) -> Result<(), String> {
        let key = format!("effects_{:?}", phase);
        let value = serde_json::to_value(effects)
            .map_err(|e| format!("Failed to serialize effects for phase {:?}: {}", phase, e))?;
        self.shared_data.insert(key, value);
        Ok(())
    }

    pub fn get_phase_effects(&self, phase: domains::ResolutionPhase) -> Result<Vec<StateEffect>, String> {
        let key = format!("effects_{:?}", phase);
        self.shared_data
            .get(&key)
            .ok_or_else(|| format!("No effects found for phase {:?}", phase))?
            .as_array()
            .ok_or("Effects not stored as array")?
            .iter()
            .map(|v| serde_json::from_value(v.clone())
                .map_err(|e| format!("Failed to deserialize effect: {}", e)))
            .collect()
    }

    pub fn store_all_actions(&mut self, actions: Vec<ActionRecord>) -> Result<(), String> {
        let value = serde_json::to_value(actions)
            .map_err(|e| format!("Failed to serialize all actions: {}", e))?;
        self.shared_data.insert("all_actions".to_string(), value);
        Ok(())
    }

    pub fn get_all_actions(&self) -> Result<Vec<ActionRecord>, String> {
        self.shared_data
            .get("all_actions")
            .ok_or("No actions found in context")?
            .as_array()
            .ok_or("Actions not stored as array")?
            .iter()
            .map(|v| serde_json::from_value(v.clone())
                .map_err(|e| format!("Failed to deserialize action: {}", e)))
            .collect()
    }

    pub fn store_all_effects(&mut self, effects: Vec<StateEffect>) -> Result<(), String> {
        let value = serde_json::to_value(effects)
            .map_err(|e| format!("Failed to serialize all effects: {}", e))?;
        self.shared_data.insert("all_effects".to_string(), value);
        Ok(())
    }

    pub fn get_all_effects(&self) -> Result<Vec<StateEffect>, String> {
        self.shared_data
            .get("all_effects")
            .ok_or("No effects found in context")?
            .as_array()
            .ok_or("Effects not stored as array")?
            .iter()
            .map(|v| serde_json::from_value(v.clone())
                .map_err(|e| format!("Failed to deserialize effect: {}", e)))
            .collect()
    }

    pub fn store_action_to_effect_indices(&mut self, mapping: HashMap<usize, Vec<usize>>) -> Result<(), String> {
        let value = serde_json::to_value(mapping)
            .map_err(|e| format!("Failed to serialize action to effect mapping: {}", e))?;
        self.shared_data.insert("action_to_effect_indices".to_string(), value);
        Ok(())
    }

    pub fn get_action_to_effect_indices(&self) -> Result<HashMap<usize, Vec<usize>>, String> {
        self.shared_data
            .get("action_to_effect_indices")
            .ok_or("No action to effect mapping found")?
            .as_object()
            .ok_or("Mapping not stored as object")?
            .iter()
            .map(|(k, v)| {
                let key: usize = k.parse()
                    .map_err(|e| format!("Failed to parse action index {}: {}", k, e))?;
                let indices: Vec<usize> = serde_json::from_value(v.clone())
                    .map_err(|e| format!("Failed to deserialize effect indices: {}", e))?;
                Ok((key, indices))
            })
            .collect()
    }

    pub fn store_trades(&mut self, trades: Vec<Trade>) -> Result<(), String> {
        let value = serde_json::to_value(trades)
            .map_err(|e| format!("Failed to serialize trades: {}", e))?;
        self.shared_data.insert("trades".to_string(), value);
        Ok(())
    }

    pub fn get_trades(&self) -> Result<Vec<Trade>, String> {
        self.shared_data
            .get("trades")
            .ok_or("No trades found in context")?
            .as_array()
            .ok_or("Trades not stored as array")?
            .iter()
            .map(|v| serde_json::from_value(v.clone())
                .map_err(|e| format!("Failed to deserialize trade: {}", e)))
            .collect()
    }

    pub fn store_market_snapshots(
        &mut self,
        snapshots: HashMap<MarketId, MarketSnapshot>,
    ) -> Result<(), String> {
        let string_keyed_snapshots: HashMap<String, MarketSnapshot> =
            snapshots.into_iter().map(|(k, v)| (k.to_string(), v)).collect();

        let value = serde_json::to_value(string_keyed_snapshots)
            .map_err(|e| format!("Failed to serialize market snapshots: {}", e))?;
        self.shared_data.insert("market_snapshots".to_string(), value);
        Ok(())
    }

    pub fn get_market_snapshots(&self) -> Result<HashMap<MarketId, MarketSnapshot>, String> {
        let value = self.shared_data.get("market_snapshots").ok_or("No market snapshots found")?;
        
        let string_keyed_map: HashMap<String, MarketSnapshot> = serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to deserialize market snapshots map: {}", e))?;

        string_keyed_map
            .into_iter()
            .map(|(k, v)| {
                let market_id = k.parse::<MarketId>()
                    .map_err(|e| format!("Failed to parse MarketId from string '{}': {}", k, e))?;
                Ok((market_id, v))
            })
            .collect()
    }

    pub fn store<T: Serialize>(&mut self, key: &str, data: T) -> Result<(), String> {
        let value = serde_json::to_value(data)
            .map_err(|e| format!("Failed to serialize data for key {}: {}", key, e))?;
        self.shared_data.insert(key.to_string(), value);
        Ok(())
    }

    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, String> {
        let value = self.shared_data
            .get(key)
            .ok_or_else(|| format!("No data found for key: {}", key))?;
        
        serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to deserialize data for key {}: {}", key, e))
    }

    pub fn step_completed_successfully(&self, step: TickStep) -> bool {
        self.step_data.get(&step).map(|r| r.success).unwrap_or(false)
    }

    pub fn get_step_metadata(&self, step: TickStep) -> Option<&serde_json::Value> {
        self.step_data.get(&step).map(|r| &r.metadata)
    }

    pub fn get_step_error(&self, step: TickStep) -> Option<&str> {
        self.step_data.get(&step)?.error.as_deref()
    }

    pub fn clear_temporary_data(&mut self) {
        let keys_to_remove: Vec<_> = self.shared_data
            .keys()
            .filter(|k| k.starts_with("temp_") || k.contains("large_"))
            .cloned()
            .collect();
        
        for key in keys_to_remove {
            self.shared_data.remove(&key);
        }
    }
}