use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InventoryEffect {
    AddInventory { owner: AgentId, good_id: GoodId, quantity: f64, unit_cost: f64 },
    RemoveInventory { owner: AgentId, good_id: GoodId, quantity: f64 },
}

impl InventoryEffect {
    pub fn name(&self) -> &'static str {
        match self {
            InventoryEffect::AddInventory { .. } => "AddInventory",
            InventoryEffect::RemoveInventory { .. } => "RemoveInventory",
        }
    }
}
