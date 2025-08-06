pub mod banking;
pub mod trading;
pub mod production;
pub mod consumption;
pub mod validation;
pub mod fiscal;
pub mod settlement;

pub use settlement::*;
pub use fiscal::*;
pub use banking::*;
pub use trading::*;
pub use production::*;
pub use consumption::*;
pub use validation::*;

use sim_types::AgentId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimAction {
    Banking(BankingAction),
    Trading(TradingAction),
    Production(ProductionAction),
    Consumption(ConsumptionAction),
    Fiscal(FiscalAction),
    Settlement(SettlementAction),
}

impl SimAction {
    pub fn name(&self) -> String {
        match self {
            SimAction::Banking(action) => format!("Banking::{}", action.name()),
            SimAction::Trading(action) => format!("Trading::{}", action.name()),
            SimAction::Production(action) => format!("Production::{}", action.name()),
            SimAction::Consumption(action) => format!("Consumption::{}", action.name()),
            SimAction::Fiscal(_) => "Fiscal".to_string(), // + Handle new variant
            SimAction::Settlement(action) => format!("Settlement::{}", action.name()),
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            SimAction::Banking(action) => action.agent_id(),
            SimAction::Trading(action) => action.agent_id(),
            SimAction::Production(action) => action.agent_id(),
            SimAction::Consumption(action) => action.agent_id(),
            SimAction::Fiscal(action) => match action { // + Handle new variant
                FiscalAction::ChangeTaxRate { government_id, .. } => *government_id,
                FiscalAction::IssueDebt { government_id, .. } => *government_id,
                FiscalAction::SetSpendingTarget { government_id, .. } => *government_id,
            }
            SimAction::Settlement(action) => action.agent_id(),
        }
    }
}