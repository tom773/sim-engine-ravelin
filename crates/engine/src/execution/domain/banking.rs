use crate::domain::ExecutionDomain;
use crate::{effects::ExecutionResult, state::SimState};
use shared::*;

pub struct BankingDomain {
    // Banking domain specific fields
}

impl BankingDomain {
    pub fn new() -> Self {
        BankingDomain {
            // Initialize fields if necessary
        }
    }
}

impl ExecutionDomain for BankingDomain {
    fn name(&self) -> &'static str {
        "BankingDomain"
    }
    fn can_handle(&self, action: &SimAction) -> bool {
        matches!(
            action,
            SimAction::Deposit { .. }
                | SimAction::Withdraw { .. }
                | SimAction::Transfer { .. }
                | SimAction::UpdateReserves { .. }
        )
    }

    fn validate(&self, action: &SimAction, state: &SimState) -> bool {
        match action {
            SimAction::Deposit { agent_id, bank, amount } => {
                let agent_cash = state.financial_system.get_liquid_assets(&agent_id.clone());
                if agent_cash < *amount { 
                    println!("\n[DEPOSIT] from [{}] at Bank [{}] for amt ${:.2} - FAILED: Insufficient funds ${:.2}\n", 
                        &agent_id.0.to_string()[0..3], &bank.0.to_string()[0..3], amount, agent_cash);
                    return false 
                }
                println!("\n[DEPOSIT] from [{}] at Bank [{}] for amt ${:.2}, Balance: {:.2} | Proceeding\n", 
                    &agent_id.0.to_string()[0..3], &bank.0.to_string()[0..3], amount, agent_cash);
            }
            _ => (),
        } 
        true
    }
    fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        
        ExecutionResult {
            success: true,
            effects: vec![],
            errors: vec![],
        }
    }
}
