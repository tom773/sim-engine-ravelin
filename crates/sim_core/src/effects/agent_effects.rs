use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentEffect {
    EstablishEmployment {
        firm_id: AgentId,
        consumer_id: AgentId,
        contract: EmploymentContract,
    },
    TerminateEmployment {
        firm_id: AgentId,
        consumer_id: AgentId,
    },
    UpdateIncome { id: AgentId, new_income: f64 },
    RecordDividendIncome { recipient: AgentId, amount: f64 },
    UpdateRevenue { id: AgentId, revenue: f64 },
    Produce { firm: AgentId, good_id: GoodId, amount: f64 },
}

impl AgentEffect {
    pub fn name(&self) -> &'static str {
        match self {
            AgentEffect::EstablishEmployment { .. } => "EstablishEmployment",
            AgentEffect::TerminateEmployment { .. } => "TerminateEmployment",
            AgentEffect::UpdateIncome { .. } => "UpdateIncome",
            AgentEffect::RecordDividendIncome { .. } => "RecordDividendIncome",
            AgentEffect::UpdateRevenue { .. } => "UpdateRevenue",
            AgentEffect::Produce { .. } => "Produce",
        }
    }
}