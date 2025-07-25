
use super::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimAction {
    IssueIncome {
        recipient: AgentId,
        amount: f64,
    },
    
    DepositCash {
        depositor: AgentId,
        bank: AgentId,
        amount: f64,
    },
    
    WithdrawCash {
        account_holder: AgentId,
        bank: AgentId,
        amount: f64,
    },
    
    Transfer {
        from: AgentId,
        to: AgentId,
        amount: f64,
    },
    
    Purchase {
        buyer: AgentId,
        seller: AgentId,
        good_id: String,
        amount: f64,
    },
    
    UpdateReserves {
        bank: AgentId,
        amount_change: f64, // positive for increase, negative for decrease
    },
}

impl SimAction {
    pub fn name(&self) -> String {
        match self {
            SimAction::IssueIncome { .. } => "Issue Income".to_string(),
            SimAction::DepositCash { .. } => "Deposit Cash".to_string(),
            SimAction::WithdrawCash { .. } => "Withdraw Cash".to_string(),
            SimAction::Transfer { .. } => "Transfer Funds".to_string(),
            SimAction::Purchase { .. } => "Purchase Good".to_string(),
            SimAction::UpdateReserves { .. } => "Update Reserves".to_string(),
        }
    }
}

pub fn agent_action_to_sim_actions(
    agent_id: &AgentId,
    action: &Action,
    state: &SimState,
) -> Vec<SimAction> {
    let mut sim_actions = Vec::new();
    match action {
        Action::DepositCash { amount } => {
            if let Some(consumer) = state.consumers.iter().find(|c| c.id == *agent_id) {
                sim_actions.push(SimAction::DepositCash {
                    depositor: agent_id.clone(),
                    bank: consumer.bank_id.clone(),
                    amount: *amount,
                });
            }
        }
        
        Action::WithdrawCash { amount } => {
            if let Some(consumer) = state.consumers.iter().find(|c| c.id == *agent_id) {
                sim_actions.push(SimAction::WithdrawCash {
                    account_holder: agent_id.clone(),
                    bank: consumer.bank_id.clone(),
                    amount: *amount,
                });
            }
        }
        
        Action::Buy { good_id, quantity, amount } => {
            sim_actions.push(SimAction::Purchase {
                buyer: agent_id.clone(),
                seller: AgentId(Uuid::new_v4()), // Placeholder for seller
                good_id: good_id.clone(),
                amount: *amount,
            });
        }
        _ => {},    
    }
    sim_actions
}