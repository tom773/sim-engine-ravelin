use sim_prelude::*;
use crate::{BankingValidator, BankingOperations};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankingDomain {
    validator: BankingValidator,
    operations: BankingOperations,
}

impl BankingDomain {
    pub fn new() -> Self {
        Self {
            validator: BankingValidator::new(),
            operations: BankingOperations::new(),
        }
    }

    pub fn can_handle(&self, action: &BankingAction) -> bool {
        match action {
            BankingAction::Deposit { .. } => true,
            BankingAction::Withdraw { .. } => true,
            BankingAction::Transfer { .. } => true,
            BankingAction::PayWages { .. } => true,
            BankingAction::UpdateReserves { .. } => true,
            BankingAction::InjectLiquidity => true,
        }
    }

    pub fn validate(&self, action: &BankingAction, state: &SimState) -> Result<(), String> {
        self.validator.validate(action, state)
    }

    pub fn execute(&self, action: &BankingAction, state: &SimState) -> BankingResult {
        // First validate the action
        if let Err(error) = self.validate(action, state) {
            return BankingResult {
                success: false,
                effects: vec![],
                errors: vec![error],
            };
        }

        // Then execute the operation
        match action {
            BankingAction::Deposit { agent_id, bank, amount } => {
                self.operations.execute_deposit(*agent_id, *bank, *amount, state)
            }
            BankingAction::Withdraw { agent_id, bank, amount } => {
                self.operations.execute_withdraw(*agent_id, *bank, *amount, state)
            }
            BankingAction::Transfer { from, to, amount } => {
                self.operations.execute_transfer(*from, *to, *amount, state)
            }
            BankingAction::PayWages { agent_id, employee, amount } => {
                self.operations.execute_transfer(*agent_id, *employee, *amount, state)
            }
            BankingAction::UpdateReserves { bank, amount_change } => {
                self.operations.execute_update_reserves(*bank, *amount_change, state)
            }
            BankingAction::InjectLiquidity => {
                self.operations.execute_inject_liquidity(state)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct BankingResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl Default for BankingDomain {
    fn default() -> Self {
        Self::new()
    }
}