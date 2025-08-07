use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FiscalAction {
    ChangeTaxRate {
        government_id: AgentId,
        tax_type: TaxType,
        new_rate: f64,
    },
    IssueDebt {
        government_id: AgentId,
        tenor: Tenor,
        face_value: f64,
    },
    SetSpendingTarget {
        government_id: AgentId,
        target: SpendingTargets,
        new_level: f64,
    }
}

impl FiscalAction {
    pub fn name(&self) -> &'static str {
        match self {
            FiscalAction::ChangeTaxRate { .. } => "ChangeTaxRate",
            FiscalAction::IssueDebt { .. } => "IssueDebt",
            FiscalAction::SetSpendingTarget { .. } => "SetSpendingTarget",
        }
    }
    pub fn agent_id(&self) -> AgentId {
        match self {
            FiscalAction::ChangeTaxRate { government_id, .. } => *government_id,
            FiscalAction::IssueDebt { government_id, .. } => *government_id,
            FiscalAction::SetSpendingTarget { government_id, .. } => *government_id,
        }
    }
}