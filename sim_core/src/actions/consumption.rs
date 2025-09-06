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