use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsumptionAction {
    Consume { agent_id: AgentId, good_id: GoodId, amount: f64 },
    NoAction { agent_id: AgentId },
}

impl ConsumptionAction {
    pub fn name(&self) -> &'static str {
        match self {
            ConsumptionAction::Consume { .. } => "Consume",
            ConsumptionAction::NoAction { .. } => "NoAction",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            ConsumptionAction::Consume { agent_id, .. } => *agent_id,
            ConsumptionAction::NoAction { agent_id } => *agent_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsumptionIntention {
    SpendOnGood { agent_id: AgentId, good_id: GoodId, max_notional: f64 },
    ConsumeGood { agent_id: AgentId, good_id: GoodId, quantity: f64 },
}

impl ConsumptionIntention {
    pub fn name(&self) -> &'static str {
        match self {
            ConsumptionIntention::SpendOnGood { .. } => "SpendOnGood",
            ConsumptionIntention::ConsumeGood { .. } => "ConsumeGood",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            ConsumptionIntention::SpendOnGood { agent_id, .. } => *agent_id,
            ConsumptionIntention::ConsumeGood { agent_id, .. } => *agent_id,
        }
    }
}
