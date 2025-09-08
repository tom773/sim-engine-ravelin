pub mod banking;
pub mod consumption;
pub mod fiscal;
pub mod production;
pub mod validation;
pub mod transaction;
pub mod monetary;


pub use consumption::*;
pub use fiscal::*;
pub use production::*;
pub use banking::*;
pub use validation::*;
pub use transaction::*;
pub use monetary::*;

use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimAction {
    Banking(BankingAction),
    Consumption(ConsumptionAction),
    Fiscal(FiscalAction),
    Production(ProductionAction),
    Transaction(TransactionAction),
    Monetary(MonetaryAction),
}

impl SimAction {
    pub fn name(&self) -> String {
        match self {
            SimAction::Banking(action) => format!("Banking::{}", action.name()),
            SimAction::Consumption(action) => format!("Consumption::{}", action.name()),
            SimAction::Fiscal(action) => format!("Fiscal::{}", action.name()),
            SimAction::Production(action) => format!("Production::{}", action.name()),
            SimAction::Transaction(action) => format!("Transaction::{}", action.name()),
            SimAction::Monetary(action) => format!("Monetary::{}", action.name()),
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            SimAction::Banking(action) => action.agent_id(),
            SimAction::Consumption(action) => action.agent_id(),
            SimAction::Fiscal(action) => action.agent_id(),
            SimAction::Production(action) => action.agent_id(),
            SimAction::Transaction(action) => action.agent_id(),
            SimAction::Monetary(action) => action.agent_id(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimIntention {
    Banking(BankingIntention),
    Consumption(ConsumptionIntention),
    Fiscal(FiscalIntention),
    Production(ProductionIntention),
    Transaction(TransactionIntention),
    Monetary(MonetaryIntention),
}

impl SimIntention {
    pub fn name(&self) -> String {
        match self {
            SimIntention::Banking(intention) => format!("Banking::{}", intention.name()),
            SimIntention::Consumption(intention) => format!("Consumption::{}", intention.name()),
            SimIntention::Fiscal(intention) => format!("Fiscal::{}", intention.name()),
            SimIntention::Production(intention) => format!("Production::{}", intention.name()),
            SimIntention::Transaction(intention) => format!("Transaction::{}", intention.name()),
            SimIntention::Monetary(intention) => format!("Monetary::{}", intention.name()),
        }
    }
    pub fn agent_id(&self) -> AgentId {
        match self {
            SimIntention::Banking(intention) => intention.agent_id(),
            SimIntention::Consumption(intention) => intention.agent_id(),
            SimIntention::Fiscal(intention) => intention.agent_id(),
            SimIntention::Production(intention) => intention.agent_id(),
            SimIntention::Transaction(intention) => intention.agent_id(),
            SimIntention::Monetary(intention) => intention.agent_id(),
        }
    } 
}