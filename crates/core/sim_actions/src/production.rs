use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProductionAction {
    Hire { agent_id: AgentId, count: u32 },
    Produce { agent_id: AgentId, recipe_id: RecipeId, batches: u32 },
}

impl ProductionAction {
    pub fn name(&self) -> &'static str {
        match self {
            ProductionAction::Hire { .. } => "Hire",
            ProductionAction::Produce { .. } => "Produce",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            ProductionAction::Hire { agent_id, .. } => *agent_id,
            ProductionAction::Produce { agent_id, .. } => *agent_id,
        }
    }
}