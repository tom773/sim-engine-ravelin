use serde::{Serialize, Deserialize};
use crate::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimAction {
    Wages {
        agent_id: AgentId,
        amount: f64,
    },
    Deposit {
        agent_id: AgentId,
        bank: AgentId,
        amount: f64,
    },
    
    Withdraw {
        agent_id: AgentId,
        bank: AgentId,
        amount: f64,
    },
    
    Transfer {
        agent_id: AgentId,
        from: AgentId,
        to: AgentId,
        amount: f64,
    },
    Hire {
        agent_id: AgentId,
        count: u32,
    },
    Produce {
        agent_id: AgentId,
        good_id: GoodId,
        amount: f64,
    },
    Purchase {
        agent_id: AgentId,
        seller: AgentId,
        good_id: String,
        amount: f64,
    },
    Consume {
        agent_id: AgentId,
        good_id: GoodId,
        amount: f64,
    }, 
    UpdateReserves {
        bank: AgentId,
        amount_change: f64,
    },
}

impl SimAction {
    pub fn name(&self) -> String {
        match self {
            SimAction::Wages { .. } => "Issue Income".to_string(),
            SimAction::Deposit { .. } => "Deposit Cash".to_string(),
            SimAction::Withdraw { .. } => "Withdraw Cash".to_string(),
            SimAction::Transfer { .. } => "Transfer Funds".to_string(),
            SimAction::Purchase { .. } => "Purchase Good".to_string(),
            SimAction::UpdateReserves { .. } => "Update Reserves".to_string(),
            SimAction::Hire { .. } => "Hire Employees".to_string(),
            SimAction::Produce { .. } => "Produce Goods".to_string(),
            SimAction::Consume { .. } => "Consume Goods".to_string(),
        }
    }
}
