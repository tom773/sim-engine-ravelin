use super::*;
use shared::*;

pub struct TransactionExecutor;

impl TransactionExecutor {
    pub fn execute_action(action: &SimAction, state: &SimState) -> ExecutionResult {
        let registry = DomainRegistry::new();
        registry.execute(action, state)
    }
     pub fn apply_effects(effects: &[StateEffect], state: &mut SimState) -> Result<(), String> {
        for effect in effects {
            match effect {
                StateEffect::CreateInstrument(instrument) => {
                    state.financial_system
                        .create_or_consolidate_instrument(instrument.clone())
                        .map_err(|e| format!("Failed to create instrument: {}", e))?;
                }
                StateEffect::UpdateInstrument { id, new_principal } => {
                    let instrument = state.financial_system.instruments.get(id)
                        .ok_or_else(|| format!("Instrument {} not found", &id.0.to_string()[..8]))?
                        .clone(); // Clone to avoid borrow issues
                    
                    if let Some(inst) = state.financial_system.instruments.get_mut(id) {
                        inst.principal = *new_principal;
                    }
                    
                    if let Some(creditor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.creditor) {
                        if let Some(asset) = creditor_bs.assets.get_mut(id) {
                            asset.principal = *new_principal;
                        }
                    }
                    
                    if let Some(debtor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.debtor) {
                        if let Some(liability) = debtor_bs.liabilities.get_mut(id) {
                            liability.principal = *new_principal;
                        }
                    }
                }
                
                StateEffect::TransferInstrument { id, new_creditor } => {
                    state.financial_system
                        .transfer_instrument(id, new_creditor.clone())?;
                }
                
                StateEffect::RemoveInstrument(id) => {
                    if let Some(instrument) = state.financial_system.instruments.remove(id) {
                        if let Some(creditor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.creditor) {
                            creditor_bs.assets.remove(id);
                        }
                        if let Some(debtor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.debtor) {
                            debtor_bs.liabilities.remove(id);
                        }
                    }
                }
                
                StateEffect::SwapInstrument { id, new_debtor, new_creditor } => {
                    if let Some(instrument) = state.financial_system.instruments.get(id).cloned() {
                        let old_creditor = instrument.creditor.clone();
                        let old_debtor = instrument.debtor.clone();
                        
                        if let Some(inst) = state.financial_system.instruments.get_mut(id) {
                            inst.debtor = new_debtor.clone();
                            inst.creditor = new_creditor.clone();
                        }
                        
                        if let Some(old_creditor_bs) = state.financial_system.balance_sheets.get_mut(&old_creditor) {
                            old_creditor_bs.assets.remove(id);
                        }
                        if let Some(old_debtor_bs) = state.financial_system.balance_sheets.get_mut(&old_debtor) {
                            old_debtor_bs.liabilities.remove(id);
                        }
                        
                        if let Some(new_creditor_bs) = state.financial_system.balance_sheets.get_mut(new_creditor) {
                            if let Some(updated_inst) = state.financial_system.instruments.get(id) {
                                new_creditor_bs.assets.insert(id.clone(), updated_inst.clone());
                            }
                        }
                        if let Some(new_debtor_bs) = state.financial_system.balance_sheets.get_mut(new_debtor) {
                            if let Some(updated_inst) = state.financial_system.instruments.get(id) {
                                new_debtor_bs.liabilities.insert(id.clone(), updated_inst.clone());
                            }
                        }
                    }
                }
                
                StateEffect::RecordTransaction(tx) => {
                    println!("Transaction recorded: {:?}", tx);
                }
                
                StateEffect::Hire { firm, count } => {
                    println!("[HIRE] Firm {} hiring {} employees", &firm.0.to_string()[..4], count);
                }
                
                StateEffect::Produce { firm, amount, good_id } => {
                    println!("[PRODUCE] Firm {} producing {} of {}", 
                        &firm.0.to_string()[..4], amount, &good_id.0);
                }
                
                _ => {
                    return Err(format!("Effect {:?} is not implemented", effect.name()));
                }
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SimState;
    use crate::AgentFactory;
    use crate::TransactionExecutor;
    use crate::execution::domain::banking::BankingDomain;
    use rand::prelude::*;
    
    fn test_economy_with_liquidity() -> SimState {
        let mut rng = StdRng::from_os_rng();
        let mut state = SimState::default();
        
        let central_bank_id = state.financial_system.central_bank.id.clone();
        
        let agent_ids = {
            let mut factory = AgentFactory::new(&mut state, &mut rng);
            let mut agents = Vec::new();
            for _ in 0..2 {
                let bank = factory.create_bank();
                let consumer = factory.create_consumer(bank.id.clone());
                let firm = factory.create_firm(bank.id.clone());
                agents.push((bank.id.clone(), consumer.id.clone(), firm.id.clone()));
            }
            agents
        }; // factory dropped here
        
        for (_, consumer_id, _) in &agent_ids {
            let cash = cash!(
                consumer_id.clone(),
                1000.0,
                central_bank_id.clone(),
                0
            );
            state.financial_system.create_instrument(cash).unwrap();
        }
        
        state
    }
    
    #[test]
    fn test_banking_domain_directly() {
        let state = test_economy_with_liquidity();
        let consumer = state.consumers.first().unwrap();
        let bank = state.financial_system.commercial_banks.values().next().unwrap();
        
        let domain = BankingDomain::new();
        
        let action = SimAction::Deposit {
            agent_id: consumer.id.clone(),
            bank: bank.id.clone(),
            amount: 500.0,
        };
        assert!(domain.validate(&action, &state));

        let result = domain.execute(&action, &state);
        assert!(result.success, "Deposit failed: {:?}", result.errors);
        assert!(!result.effects.is_empty());
    }
    
    #[test]
    fn test_full_action_flow() {
        let mut state = test_economy_with_liquidity();
        
        let consumer = state.consumers.first().unwrap();
        let firm = state.firms.first().unwrap();
        let bank = state.financial_system.commercial_banks.values().next().unwrap();
        
        let consumer_id = consumer.id.clone();
        let firm_id = firm.id.clone();
        let bank_id = bank.id.clone();
        
        let deposit_action = SimAction::Deposit {
            agent_id: consumer_id.clone(),
            bank: bank_id.clone(),
            amount: 500.0,
        };
        
        let registry = DomainRegistry::new();
        let d_result = registry.execute(&deposit_action, &state);
        assert!(d_result.success, "Deposit action failed: {:?}", d_result.errors);
        
        TransactionExecutor::apply_effects(&d_result.effects, &mut state).unwrap();
        
        let consumer_cash_after = state.financial_system.get_cash_assets(&consumer_id);
        assert_eq!(consumer_cash_after, 500.0, "Consumer should have 500 cash left");
        
        let consumer_deposits = state.financial_system.get_deposits_at_bank(&consumer_id, &bank_id);
        assert_eq!(consumer_deposits, 500.0, "Consumer should have 500 in deposits");
        
        let hire_action = SimAction::Hire {
            agent_id: firm_id.clone(),
            count: 5,
        };
        
        let produce_action = SimAction::Produce {
            agent_id: firm_id.clone(),
            good_id: GoodId::generic(), // Use the provided method
            amount: 100.0,
        };
        
        let h_result = registry.execute(&hire_action, &state);
        let p_result = registry.execute(&produce_action, &state);
        
        if h_result.success {
            TransactionExecutor::apply_effects(&h_result.effects, &mut state).unwrap();
        }
        if p_result.success {
            TransactionExecutor::apply_effects(&p_result.effects, &mut state).unwrap();
        }
    }
    
    #[test]
    fn test_monetary_aggregates_after_deposit() {
        let mut state = test_economy_with_liquidity();
        let initial_m1 = state.financial_system.m1();
        
        let consumer = state.consumers.first().unwrap();
        let bank = state.financial_system.commercial_banks.values().next().unwrap();
        let consumer_id = consumer.id.clone();
        let bank_id = bank.id.clone();
        
        let domain = BankingDomain::new();
        let action = SimAction::Deposit {
            agent_id: consumer_id.clone(),
            bank: bank_id.clone(),
            amount: 500.0,
        };
        
        let result = domain.execute(&action, &state);
        assert!(result.success);
        
        TransactionExecutor::apply_effects(&result.effects, &mut state).unwrap();
        
        let final_m1 = state.financial_system.m1();
        assert_eq!(initial_m1, final_m1, "M1 should remain constant when cash becomes deposits");
        
        let consumer_cash = state.financial_system.get_cash_assets(&consumer_id);
        let consumer_deposits = state.financial_system.get_deposits_at_bank(&consumer_id, &bank_id);
        assert_eq!(consumer_cash + consumer_deposits, 1000.0, "Total liquid assets should be preserved");
    }
}

#[cfg(test)]
mod consolidation_tests {
    use super::*;
    use uuid::Uuid;
    
    #[test]
    fn test_cash_consolidation() {
        let agent1 = AgentId(Uuid::new_v4());
        let agent2 = AgentId(Uuid::new_v4());
        let cb = AgentId(Uuid::new_v4());
        
        let cash1 = cash!(agent1.clone(), 100.0, cb.clone(), 0);
        let cash2 = cash!(agent1.clone(), 200.0, cb.clone(), 0);
        let cash3 = cash!(agent2.clone(), 300.0, cb.clone(), 0);
        
        assert!(cash1.can_consolidate_with(&cash2), "Same holder cash should consolidate");
        assert!(!cash1.can_consolidate_with(&cash3), "Different holder cash should not consolidate");
    }
    
    #[test]
    fn test_deposit_consolidation() {
        let depositor = AgentId(Uuid::new_v4());
        let bank = AgentId(Uuid::new_v4());
        
        let deposit1 = deposit!(depositor.clone(), bank.clone(), 100.0, 2.0, 0);
        let deposit2 = deposit!(depositor.clone(), bank.clone(), 200.0, 2.0, 0);
        let deposit3 = deposit!(depositor.clone(), bank.clone(), 300.0, 3.0, 0); // Different rate
        
        assert!(deposit1.can_consolidate_with(&deposit2), "Same rate deposits should consolidate");
        assert!(!deposit1.can_consolidate_with(&deposit3), "Different rate deposits should not consolidate");
    }
    
    #[test]
    fn test_loan_never_consolidates() {
        let lender = AgentId(Uuid::new_v4());
        let borrower = AgentId(Uuid::new_v4());
        
        let loan1 = loan!(lender.clone(), borrower.clone(), 1000.0, 5.0, 360, LoanType::Personal, 0);
        let loan2 = loan!(lender.clone(), borrower.clone(), 2000.0, 5.0, 360, LoanType::Personal, 0);
        
        assert!(!loan1.can_consolidate_with(&loan2), "Loans should never consolidate");
    }
}