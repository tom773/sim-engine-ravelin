use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentEffect {
    Hire { firm: AgentId, count: u32 },
    UpdateIncome { id: AgentId, new_income: f64 },
    UpdateRevenue { id: AgentId, revenue: f64 },
    Produce { firm: AgentId, good_id: GoodId, amount: f64 },
}

impl AgentEffect {
    pub fn name(&self) -> &'static str {
        match self {
            AgentEffect::Hire { .. } => "Hire",
            AgentEffect::UpdateIncome { .. } => "UpdateIncome",
            AgentEffect::UpdateRevenue { .. } => "UpdateRevenue",
            AgentEffect::Produce { .. } => "Produce",
        }
    }
}