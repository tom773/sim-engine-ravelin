use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsumptionAction {
    Purchase { agent_id: AgentId, seller: AgentId, good_id: GoodId, amount: f64 },
    Consume { agent_id: AgentId, good_id: GoodId, amount: f64 },
}

impl ConsumptionAction {
    pub fn name(&self) -> &'static str {
        match self {
            ConsumptionAction::Purchase { .. } => "Purchase",
            ConsumptionAction::Consume { .. } => "Consume",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            ConsumptionAction::Purchase { agent_id, .. } => *agent_id,
            ConsumptionAction::Consume { agent_id, .. } => *agent_id,
        }
    }
}