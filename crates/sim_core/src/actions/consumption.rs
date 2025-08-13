use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsumptionAction {
    Purchase { agent_id: AgentId, seller: AgentId, good_id: GoodId, amount: f64 },
    // New: PurchaseAtBest (Market order) (Point 3)
    PurchaseAtBest { agent_id: AgentId, good_id: GoodId, max_notional: f64 },
    Consume { agent_id: AgentId, good_id: GoodId, amount: f64 },
    NoAction { agent_id: AgentId },
}

impl ConsumptionAction {
    pub fn name(&self) -> &'static str {
        match self {
            ConsumptionAction::Purchase { .. } => "Purchase",
            ConsumptionAction::PurchaseAtBest { .. } => "PurchaseAtBest",
            ConsumptionAction::Consume { .. } => "Consume",
            ConsumptionAction::NoAction { .. } => "NoAction",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            ConsumptionAction::Purchase { agent_id, .. } => *agent_id,
            ConsumptionAction::PurchaseAtBest { agent_id, .. } => *agent_id,
            ConsumptionAction::Consume { agent_id, .. } => *agent_id,
            ConsumptionAction::NoAction { agent_id } => *agent_id,
        }
    }
}