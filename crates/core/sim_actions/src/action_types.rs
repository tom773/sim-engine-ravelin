use crate::*;
use sim_types::AgentId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimAction {
    Banking(BankingAction),
    Trading(TradingAction),
    Production(ProductionAction),
    Consumption(ConsumptionAction),
}

impl SimAction {
    pub fn name(&self) -> String {
        match self {
            SimAction::Banking(action) => format!("Banking::{}", action.name()),
            SimAction::Trading(action) => format!("Trading::{}", action.name()),
            SimAction::Production(action) => format!("Production::{}", action.name()),
            SimAction::Consumption(action) => format!("Consumption::{}", action.name()),
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            SimAction::Banking(action) => action.agent_id(),
            SimAction::Trading(action) => action.agent_id(),
            SimAction::Production(action) => action.agent_id(),
            SimAction::Consumption(action) => action.agent_id(),
        }
    }
}