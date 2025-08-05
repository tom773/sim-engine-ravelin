use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FirmDecision {
    Produce { recipe_id: RecipeId, batches: u32 },
    Hire { count: u32 },
    SetPrice { good_id: GoodId, price: f64 },
    PayWages { employee: AgentId, amount: f64 },
    SellInventory { good_id: GoodId, quantity: f64 },
}

impl FirmDecision {
    pub fn name(&self) -> &'static str {
        match self {
            FirmDecision::Produce { .. } => "Produce",
            FirmDecision::Hire { .. } => "Hire",
            FirmDecision::SetPrice { .. } => "SetPrice",
            FirmDecision::PayWages { .. } => "PayWages",
            FirmDecision::SellInventory { .. } => "SellInventory",
        }
    }
}