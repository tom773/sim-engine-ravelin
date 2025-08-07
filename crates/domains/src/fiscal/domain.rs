use serde::{Deserialize, Serialize};
use sim_core::*;
use sim_macros::SimDomain;

#[derive(Clone, Debug, Serialize, Deserialize, Default, SimDomain)]
pub struct FiscalDomain {}

#[derive(Debug, Clone)]
pub struct FiscalResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl FiscalDomain {
    pub fn new() -> Self {
        Self {}
    }
    pub fn can_handle(&self, action: &FiscalAction) -> bool {
        match action {
            FiscalAction::ChangeTaxRate { .. } => true,
            FiscalAction::IssueDebt { .. } => true,
            FiscalAction::SetSpendingTarget { .. } => true,
        }
    }
    pub fn validate(&self, _action: &FiscalAction, _state: &SimState) -> FiscalResult {
        let errors = vec![];

        FiscalResult { success: errors.is_empty(), effects: vec![], errors }
    }
    pub fn execute(&self, action: &FiscalAction, state: &SimState) -> FiscalResult {
        let mut effects = vec![];

        match action {
            FiscalAction::ChangeTaxRate { government_id, tax_type, new_rate } => {
                println!("[FISCAL DOMAIN] Executing ChangeTaxRate for {} | Setting {:?} to {}", government_id, tax_type, new_rate);
            }
            FiscalAction::IssueDebt { government_id, tenor, face_value } => {
                let maturity_date = tenor.add_to_date(state.current_date);
                let bond = bond!(
                    *government_id,
                    *government_id,
                    *face_value,
                    0.04,
                    maturity_date,
                    *face_value,
                    BondType::Government,
                    2,
                    *tenor,
                    state.current_date
                );
                effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(bond)));
            }
            FiscalAction::SetSpendingTarget { government_id, .. } => {
                println!("[FISCAL DOMAIN] Executing SetSpendingTarget for {}", government_id);
            }
        }

        FiscalResult { success: true, effects, errors: vec![] }
    }
}