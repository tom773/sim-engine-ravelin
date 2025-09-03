use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProductionAction {
    Hire { agent_id: AgentId, count: u32 },
    Produce { agent_id: AgentId, recipe_id: RecipeId, batches: u32 },
    Fire { agent_id: AgentId, employee_id: AgentId },
}

impl ProductionAction {
    pub fn name(&self) -> &'static str {
        match self {
            ProductionAction::Hire { .. } => "Hire",
            ProductionAction::Produce { .. } => "Produce",
            ProductionAction::Fire { .. } => "Fire",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            ProductionAction::Hire { agent_id, .. } => *agent_id,
            ProductionAction::Produce { agent_id, .. } => *agent_id,
            ProductionAction::Fire { agent_id, .. } => *agent_id,
        }
    }
}