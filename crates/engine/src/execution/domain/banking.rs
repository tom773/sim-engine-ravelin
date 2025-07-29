use crate::domain::ExecutionDomain;
use crate::{EffectError, StateEffect};
use crate::{effects::ExecutionResult, state::SimState};
use shared::validation::FinancialValidator;
use shared::*;
use uuid::Uuid;
pub struct BankingDomain {}

impl BankingDomain {
    pub fn new() -> Self {
        BankingDomain {}
    }
    fn execute_deposit(&self, depositor: &AgentId, bank: &AgentId, amount: f64, state: &SimState) -> ExecutionResult {
        let mut effects = vec![];
        if let Some(depositor_bs) = state.financial_system.balance_sheets.get(depositor) {
            if let Some((cash_id, cash_inst)) = depositor_bs
                .assets
                .iter()
                .find(|(_, inst)| matches!(inst.instrument_type, InstrumentType::Cash) && inst.principal >= amount)
            {
                if cash_inst.principal == amount {
                    effects.push(StateEffect::TransferInstrument { id: cash_id.clone(), new_creditor: bank.clone() });
                } else {
                    effects.push(StateEffect::UpdateInstrument {
                        id: cash_id.clone(),
                        new_principal: cash_inst.principal - amount,
                    });

                    let bank_cash =
                        cash!(bank.clone(), amount, state.financial_system.central_bank.id.clone(), state.ticknum);
                    effects.push(StateEffect::CreateInstrument(bank_cash));
                }

                let deposit = deposit!(
                    depositor.clone(),
                    bank.clone(),
                    amount,
                    state.financial_system.central_bank.policy_rate - 200.0,
                    state.ticknum
                );
                effects.push(StateEffect::CreateInstrument(deposit));
            }
        }

        let success = !effects.is_empty();
        ExecutionResult {
            success,
            effects,
            errors: if !success {
                vec![EffectError::TransactionFailure("Deposit".to_string(), "Deposit failed".to_string())]
            } else {
                vec![]
            },
        }
    }
    fn execute_inject_liquidity(&self, state: &SimState) -> ExecutionResult {
        let mut effects = vec![];
        for consumer in &state.consumers {
            let cash =
                cash!(consumer.id.clone(), 1000.0, state.financial_system.central_bank.id.clone(), state.ticknum);
            effects.push(StateEffect::CreateInstrument(cash));
        }
        println!("Injecting liquidity: {:?}", effects);
        let success = !effects.is_empty();
        ExecutionResult {
            success,
            effects,
            errors: if !success {
                vec![EffectError::TransactionFailure(
                    "Inject Liquidity".to_string(),
                    "Failed to inject liquidity".to_string(),
                )]
            } else {
                vec![]
            },
        }
    }
    fn execute_withdraw(
        &self, account_holder: &AgentId, bank: &AgentId, amount: f64, state: &SimState,
    ) -> ExecutionResult {
        let mut effects = vec![];

        if let Some(account_bs) = state.financial_system.balance_sheets.get(account_holder) {
            if let Some((deposit_id, deposit)) = account_bs.assets.iter().find(|(_, inst)| {
                inst.debtor == *bank
                    && matches!(inst.instrument_type, InstrumentType::DemandDeposit)
                    && inst.principal >= amount
            }) {
                if deposit.principal == amount {
                    effects.push(StateEffect::RemoveInstrument(deposit_id.clone()));
                } else {
                    effects.push(StateEffect::UpdateInstrument {
                        id: deposit_id.clone(),
                        new_principal: deposit.principal - amount,
                    });
                }

                if let Some(bank_bs) = state.financial_system.balance_sheets.get(bank) {
                    if let Some((cash_id, cash_inst)) = bank_bs.assets.iter().find(|(_, inst)| {
                        matches!(inst.instrument_type, InstrumentType::Cash) && inst.principal >= amount
                    }) {
                        if cash_inst.principal == amount {
                            effects.push(StateEffect::TransferInstrument {
                                id: cash_id.clone(),
                                new_creditor: account_holder.clone(),
                            });
                        } else {
                            effects.push(StateEffect::UpdateInstrument {
                                id: cash_id.clone(),
                                new_principal: cash_inst.principal - amount,
                            });

                            let withdrawn_cash = cash!(
                                account_holder.clone(),
                                amount,
                                state.financial_system.central_bank.id.clone(),
                                state.ticknum
                            );
                            effects.push(StateEffect::CreateInstrument(withdrawn_cash));
                        }
                    }
                }
            }
        }
        let eclone = effects.clone();
        ExecutionResult {
            success: !eclone.is_empty(),
            effects,
            errors: if eclone.is_empty() {
                vec![EffectError::TransactionFailure(
                    "Withdraw".to_string(),
                    "Failed to process withdrawal".to_string(),
                )]
            } else {
                vec![]
            },
        }
    }
    fn execute_transfer(&self, from: &AgentId, to: &AgentId, amount: f64, state: &SimState) -> ExecutionResult {
        ExecutionResult {
            success: false,
            effects: vec![],
            errors: vec![EffectError::UnimplementedAction("Transfer not yet implemented".to_string())],
        }
    }

    fn execute_update_reserves(&self, bank: &AgentId, amount_change: f64, state: &SimState) -> ExecutionResult {
        ExecutionResult {
            success: false,
            effects: vec![],
            errors: vec![EffectError::UnimplementedAction("Reserve update not yet implemented".to_string())],
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
                | SimAction::InjectLiquidity
        )
    }

    fn validate(&self, action: &SimAction, state: &SimState) -> bool {
        let validator = FinancialValidator::new(&state.financial_system);

        match action {
            SimAction::Deposit { agent_id, bank, amount } => {
                validator.validate_deposit(agent_id, bank, *amount).is_ok()
            }
            SimAction::Withdraw { agent_id, bank, amount } => {
                validator.validate_withdraw(agent_id, bank, *amount).is_ok()
            }
            SimAction::Transfer { from, to, amount, .. } => {
                Validator::positive_amount(*amount).is_ok()
                    && validator.ensure_has_balance_sheet(from).is_ok()
                    && validator.ensure_has_balance_sheet(to).is_ok()
            }
            _ => true,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        match action {
            SimAction::Deposit { agent_id, bank, amount } => self.execute_deposit(agent_id, bank, *amount, state),
            SimAction::Withdraw { agent_id, bank, amount } => self.execute_withdraw(agent_id, bank, *amount, state),
            SimAction::Transfer { from, to, amount, .. } => self.execute_transfer(from, to, *amount, state),
            SimAction::UpdateReserves { bank, amount_change } => {
                self.execute_update_reserves(bank, *amount_change, state)
            }
            SimAction::InjectLiquidity => self.execute_inject_liquidity(state),
            _ => ExecutionResult {
                success: false,
                effects: vec![],
                errors: vec![EffectError::InvalidState(format!(
                    "Banking Domain Doesn't Handle Action {}",
                    action.name()
                ))],
            },
        }
    }
}

#[cfg(test)]
#[cfg(test)]
mod banking_tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_state() -> SimState {
        let mut state = SimState::default();
        let agent_id = AgentId(Uuid::new_v4());
        state.financial_system.balance_sheets.insert(agent_id, BalanceSheet::new(agent_id));
        state
    }

    #[test]
    fn test_create_instrument_effect() {
        let mut state = create_test_state();
        let creditor = AgentId(Uuid::new_v4());
        let debtor = state.financial_system.central_bank.id.clone();

        // Add creditor to balance sheets
        state.financial_system.balance_sheets.insert(creditor, BalanceSheet::new(creditor));

        let inst = cash!(creditor, 1000.0, debtor, 0);
        let effect = StateEffect::CreateInstrument(inst);

        assert!(effect.apply(&mut state).is_ok());
        assert_eq!(state.financial_system.instruments.len(), 1);
    }
}
