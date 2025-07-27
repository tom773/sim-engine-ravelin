use crate::StateEffect;
use crate::domain::ExecutionDomain;
use crate::{effects::ExecutionResult, state::SimState};
use shared::*;
use uuid::Uuid;
pub struct BankingDomain {
    reserve_calculator: ReserveCalculator,
    validator: BankingValidator,
}

impl BankingDomain {
    pub fn new() -> Self {
        BankingDomain {
            reserve_calculator: ReserveCalculator::new(),
            validator: BankingValidator::new(),
        }
    }
    fn execute_deposit(
        &self,
        depositor: &AgentId,
        bank: &AgentId,
        amount: f64,
        state: &SimState,
    ) -> ExecutionResult {
        if let Err(e) = self
            .validator
            .validate_deposit(depositor, bank, amount, state)
        {
            return ExecutionResult {
                success: false,
                effects: vec![],
                errors: vec![e],
            };
        }

        let mut effects = vec![];

        if let Some(depositor_bs) = state.financial_system.balance_sheets.get(depositor) {
            if let Some((cash_id, cash_inst)) = depositor_bs.assets.iter().find(|(_, inst)| {
                matches!(inst.instrument_type, InstrumentType::Cash) && inst.principal >= amount
            }) {
                if cash_inst.principal == amount {
                    effects.push(StateEffect::TransferInstrument {
                        id: cash_id.clone(),
                        new_creditor: bank.clone(),
                    });
                } else {
                    effects.push(StateEffect::UpdateInstrument {
                        id: cash_id.clone(),
                        new_principal: cash_inst.principal - amount,
                    });

                    let bank_cash = cash!(
                        bank.clone(),
                        amount,
                        state.financial_system.central_bank.id.clone(),
                        state.ticknum
                    );
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

                let reserve_effects = self
                    .reserve_calculator
                    .calculate_reserve_effects(bank, amount, state);
                effects.extend(reserve_effects);
            }
        }

        let success = !effects.is_empty();
        ExecutionResult {
            success,
            effects,
            errors: if !success {
                vec!["Failed to process deposit: could not find sufficient cash".to_string()]
            } else {
                vec![]
            },
        }
    }
    fn execute_inject_liquidity(&self, state: &SimState) -> ExecutionResult {
        let mut effects = vec![];
        for consumer in &state.consumers {
            let cash = cash!(
                consumer.id.clone(),
                1000.0,
                state.financial_system.central_bank.id.clone(),
                state.ticknum
            );
            effects.push(StateEffect::CreateInstrument(cash));
        }
        println!("Injecting liquidity: {:?}", effects);
        let success = !effects.is_empty();
        ExecutionResult {
            success,
            effects,
            errors: if !success {
                vec!["Failed to execute transfer".to_string()]
            } else {
                vec![]
            },
        }
    }
    fn execute_withdraw(&self, account_holder: &AgentId, bank: &AgentId, amount: f64, state: &SimState) -> ExecutionResult {
        if let Err(e) = self.validator.validate_withdraw(account_holder, bank, amount, state) {
            return ExecutionResult {
                success: false,
                effects: vec![],
                errors: vec![e],
            };
        }
        
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
                vec!["Failed to process withdrawal".to_string()]
            } else {
                vec![]
            },
        }
    }
    fn execute_transfer(
        &self,
        from: &AgentId,
        to: &AgentId,
        amount: f64,
        state: &SimState,
    ) -> ExecutionResult {
        ExecutionResult {
            success: false,
            effects: vec![],
            errors: vec!["Transfer not yet implemented".to_string()],
        }
    }

    fn execute_update_reserves(
        &self,
        bank: &AgentId,
        amount_change: f64,
        state: &SimState,
    ) -> ExecutionResult {
        ExecutionResult {
            success: false,
            effects: vec![],
            errors: vec!["Reserve update not yet implemented".to_string()],
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
        match action {
            SimAction::Deposit { agent_id, bank, amount } => {
                self.validator.validate_deposit(agent_id, bank, *amount, state).is_ok()
            }
            SimAction::Withdraw { agent_id, bank, amount } => {
                self.validator.validate_withdraw(agent_id, bank, *amount, state).is_ok()
            }
            SimAction::Transfer { from, to, amount, .. } => {
                self.validator.validate_transfer(from, to, *amount, state).is_ok()
            }
            SimAction::InjectLiquidity => true,
            SimAction::UpdateReserves { .. } => true,
            _ => false,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        match action {
            SimAction::Deposit {
                agent_id,
                bank,
                amount,
            } => self.execute_deposit(agent_id, bank, *amount, state),
            SimAction::Withdraw {
                agent_id,
                bank,
                amount,
            } => self.execute_withdraw(agent_id, bank, *amount, state),
            SimAction::Transfer {
                from, to, amount, ..
            } => self.execute_transfer(from, to, *amount, state),
            SimAction::UpdateReserves {
                bank,
                amount_change,
            } => self.execute_update_reserves(bank, *amount_change, state),
            SimAction::InjectLiquidity => self.execute_inject_liquidity(state),
            _ => ExecutionResult {
                success: false,
                effects: vec![],
                errors: vec![format!(
                    "Banking domain cannot handle action: {}",
                    action.name()
                )],
            },
        }
    }
}

struct ReserveCalculator;

impl ReserveCalculator {
    fn new() -> Self {
        Self
    }

    fn calculate_reserve_effects(
        &self,
        bank: &AgentId,
        deposit_amount: f64,
        state: &SimState,
    ) -> Vec<StateEffect> {
        let mut effects = vec![];

        let reserve_requirement = state.financial_system.central_bank.reserve_requirement;
        let current_reserves = state
            .financial_system
            .get_bank_reserves(bank)
            .unwrap_or(0.0);
        let total_deposits_after =
            state.financial_system.get_total_liabilities(bank) + deposit_amount;
        let total_required_reserves = total_deposits_after * reserve_requirement;

        if current_reserves < total_required_reserves {
            let reserve_shortfall = total_required_reserves - current_reserves;

            let reserves = reserves!(
                bank.clone(),
                state.financial_system.central_bank.id.clone(),
                reserve_shortfall,
                state.financial_system.central_bank.policy_rate - 50.0,
                state.ticknum
            );
            effects.push(StateEffect::CreateInstrument(reserves));
        }

        effects
    }
}

struct BankingValidator;

impl BankingValidator {
    fn new() -> Self { Self }
    
    fn validate_positive_amount(&self, amount: f64) -> Result<(), String> {
        if amount <= 0.0 {
            Err("Amount must be positive".to_string())
        } else {
            Ok(())
        }
    }
    
    fn validate_is_bank(&self, bank_id: &AgentId, state: &SimState) -> Result<(), String> {
        if state.financial_system.commercial_banks.contains_key(bank_id) {
            Ok(())
        } else {
            Err("Target is not a valid commercial bank".to_string())
        }
    }
    
    fn validate_has_balance_sheet(&self, agent_id: &AgentId, state: &SimState) -> Result<(), String> {
        if state.financial_system.balance_sheets.contains_key(agent_id) {
            Ok(())
        } else {
            Err(format!("Agent {} does not have a balance sheet", &agent_id.0.to_string()[..8]))
        }
    }
    
    fn validate_deposit(&self, depositor: &AgentId, bank: &AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        self.validate_positive_amount(amount)?;
        self.validate_is_bank(bank, state)?;
        self.validate_has_balance_sheet(depositor, state)?;
        
        let cash_holdings = state.financial_system.get_cash_assets(depositor);
        if cash_holdings < amount {
            Err(format!("Insufficient cash: ${:.2} < ${:.2}", cash_holdings, amount))
        } else {
            Ok(())
        }
    }
    
    fn validate_withdraw(&self, account_holder: &AgentId, bank: &AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        self.validate_positive_amount(amount)?;
        self.validate_is_bank(bank, state)?;
        self.validate_has_balance_sheet(account_holder, state)?;
        
        let deposits = state.financial_system.get_deposits_at_bank(account_holder, bank);
        if deposits < amount {
            return Err(format!("Insufficient deposits: ${:.2} < ${:.2}", deposits, amount));
        }
        
        let bank_liquidity = state.financial_system.liquidity(bank);
        if bank_liquidity < amount {
            return Err(format!("Bank has insufficient liquidity: ${:.2} < ${:.2}", bank_liquidity, amount));
        }
        
        Ok(())
    }
    
    fn validate_transfer(&self, from: &AgentId, to: &AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        self.validate_positive_amount(amount)?;
        self.validate_has_balance_sheet(from, state)?;
        self.validate_has_balance_sheet(to, state)?;
        
        let from_liquidity = state.financial_system.liquidity(from);
        if from_liquidity < amount {
            Err(format!("Insufficient funds: ${:.2} < ${:.2}", from_liquidity, amount))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AgentFactory;
    use crate::state::SimState;
    use rand::prelude::*;
    use crate::execution::{TransactionExecutor, domain::banking::BankingDomain};
    fn test_economy_with_liquidity() -> SimState {
        let mut rng = StdRng::from_os_rng();
        let mut state = SimState::default();

        let central_bank_id = state.financial_system.central_bank.id.clone();

        let (bank_id, consumer_id) = {
            let mut factory = AgentFactory::new(&mut state, &mut rng);
            let bank = factory.create_bank();
            let consumer = factory.create_consumer(bank.id.clone());
            (bank.id.clone(), consumer.id.clone())
        };

        let cash = cash!(consumer_id, 1000.0, central_bank_id, 0);
        state.financial_system.create_instrument(cash).unwrap();

        state
    }

    #[test]
    fn test_deposit_validation() {
        let state = test_economy_with_liquidity();
        let consumer = state.consumers.first().unwrap();
        let bank = state
            .financial_system
            .commercial_banks
            .values()
            .next()
            .unwrap();

        let domain = BankingDomain::new();

        let action = SimAction::Deposit {
            agent_id: consumer.id.clone(),
            bank: bank.id.clone(),
            amount: 500.0,
        };
        assert!(domain.validate(&action, &state));

        let action = SimAction::Deposit {
            agent_id: consumer.id.clone(),
            bank: bank.id.clone(),
            amount: 2000.0,
        };
        assert!(!domain.validate(&action, &state));

        let action = SimAction::Deposit {
            agent_id: consumer.id.clone(),
            bank: bank.id.clone(),
            amount: -100.0,
        };
        assert!(!domain.validate(&action, &state));
    }

    #[test]
    fn test_deposit_execution() {
        let state = test_economy_with_liquidity();
        let consumer = state.consumers.first().unwrap();
        let bank = state
            .financial_system
            .commercial_banks
            .values()
            .next()
            .unwrap();

        let domain = BankingDomain::new();

        let action = SimAction::Deposit {
            agent_id: consumer.id.clone(),
            bank: bank.id.clone(),
            amount: 500.0,
        };

        let result = domain.execute(&action, &state);
        assert!(result.success);
        assert!(!result.effects.is_empty());

        assert!(result.effects.len() >= 2);
    }
    #[test]
    fn test_withdraw(){
        let mut state = test_economy_with_liquidity();
        let domain = BankingDomain::new();
        
        let consumer = state.consumers.first().unwrap();
        let bank = state.financial_system.commercial_banks.values().next().unwrap();
        // Deposit some cash first 
        let cash_pre = state.financial_system.get_cash_assets(&consumer.id);
        let action = SimAction::Deposit {
            agent_id: consumer.id.clone(),
            bank: bank.id.clone(),
            amount: 500.0,
        };
        let result = domain.execute(&action, &state);
        
        assert!(result.success);
        assert!(result.errors.is_empty(), "Deposit should succeed: {:?}", result.errors);

        let cash_inter = state.financial_system.get_cash_assets(&consumer.id);
        
        println!("\n\nCash before: {}, Cash after deposit: {}", cash_pre, cash_inter);
        
        // Now withdraw some cash
        let action = SimAction::Withdraw {
            agent_id: consumer.id.clone(),
            bank: bank.id.clone(),
            amount: 200.0,
        };
        // Execute the withdrawal
        let result = domain.execute(&action, &state);
        let cash_post = state.financial_system.get_cash_assets(&consumer.id);
        
        println!("\n\nCash before withdrawal: {}, Cash after: {}", cash_inter, cash_post);
        println!("Effects: {:?}", result.effects);
        println!("Errors: {:?}\n\n", result.errors);

        assert!(result.success);
        assert!(!result.effects.is_empty());
        assert!(cash_post > cash_pre);
        assert_eq!(cash_post - cash_pre, 200.0);
        

    }
}
