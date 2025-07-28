#[cfg(test)]
mod tests {
    use crate::state::SimState;
    use crate::{SimAction, AgentFactory, GoodId};
    use crate::TransactionExecutor;
    use crate::execution::domain::{ExecutionDomain, DomainRegistry, banking::BankingDomain};
    use rand::prelude::*;
    use shared::*;
    use uuid::Uuid;

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
        };
        
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
        
        let consumer_id = state.consumers.first().unwrap().id.clone();
        let bank_id = state.financial_system.commercial_banks.values().next().unwrap().id.clone();
        
        let domain = BankingDomain::new();
        
        let action = SimAction::Deposit {
            agent_id: consumer_id,
            bank: bank_id,
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
        
        let consumer_id = state.consumers.first().unwrap().id.clone();
        let firm_id = state.firms.first().unwrap().id.clone();
        let bank_id = state.financial_system.commercial_banks.values().next().unwrap().id.clone();
        
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
            good_id: GoodId::generic(),
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
        
        let consumer_id = state.consumers.first().unwrap().id.clone();
        let bank_id = state.financial_system.commercial_banks.values().next().unwrap().id.clone();
        
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
mod fs_tests {
    use crate::AgentFactory;
    use crate::state::SimState;
    use rand::prelude::*;
    use crate::execution::domain::{ExecutionDomain, banking::BankingDomain};
    use crate::TransactionExecutor;
    use shared::*;
    use uuid::Uuid;
    
    fn test_economy_with_liquidity() -> SimState {
        let mut rng = StdRng::from_os_rng();
        let mut state = SimState::default();

        let central_bank_id = state.financial_system.central_bank.id.clone();

        let (_bank_id, consumer_id) = {
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
        
        let consumer_id = state.consumers.first().unwrap().id.clone();
        let bank_id = state.financial_system.commercial_banks.values().next().unwrap().id.clone();

        let domain = BankingDomain::new();

        let action = SimAction::Deposit {
            agent_id: consumer_id.clone(),
            bank: bank_id.clone(),
            amount: 500.0,
        };
        assert!(domain.validate(&action, &state));

        let action = SimAction::Deposit {
            agent_id: consumer_id.clone(),
            bank: bank_id.clone(),
            amount: 2000.0,
        };
        assert!(!domain.validate(&action, &state));

        let action = SimAction::Deposit {
            agent_id: consumer_id.clone(),
            bank: bank_id.clone(),
            amount: -100.0,
        };
        assert!(!domain.validate(&action, &state));
    }

    #[test]
    fn test_deposit_execution() {
        let state = test_economy_with_liquidity();
        
        let consumer_id = state.consumers.first().unwrap().id.clone();
        let bank_id = state.financial_system.commercial_banks.values().next().unwrap().id.clone();

        let domain = BankingDomain::new();

        let action = SimAction::Deposit {
            agent_id: consumer_id,
            bank: bank_id,
            amount: 500.0,
        };

        let result = domain.execute(&action, &state);
        assert!(result.success);
        assert!(!result.effects.is_empty());
        assert!(result.effects.len() >= 2);
    }
    
    #[test]
    fn test_withdraw_and_apply() {
        let mut state = test_economy_with_liquidity();
        let domain = BankingDomain::new();
        
        let consumer_id = state.consumers.first().unwrap().id.clone();
        let bank_id = state.financial_system.commercial_banks.values().next().unwrap().id.clone();
        
        let cash_pre = state.financial_system.get_cash_assets(&consumer_id);
        
        let deposit_action = SimAction::Deposit {
            agent_id: consumer_id.clone(),
            bank: bank_id.clone(),
            amount: 500.0,
        };
        
        let deposit_result = domain.execute(&deposit_action, &state);
        assert!(deposit_result.success);
        assert!(deposit_result.errors.is_empty(), "Deposit should succeed: {:?}", deposit_result.errors);
        
        TransactionExecutor::apply_effects(&deposit_result.effects, &mut state).unwrap();
        
        let cash_inter = state.financial_system.get_cash_assets(&consumer_id);
        println!("\n\nCash before: {}, Cash after deposit: {}", cash_pre, cash_inter);
        assert_eq!(cash_inter, 500.0, "Should have 500 cash after depositing 500");
        
        let withdraw_action = SimAction::Withdraw {
            agent_id: consumer_id.clone(),
            bank: bank_id.clone(),
            amount: 200.0,
        };
        
        let withdraw_result = domain.execute(&withdraw_action, &state);
        assert!(withdraw_result.success, "Withdrawal failed: {:?}", withdraw_result.errors);
        assert!(!withdraw_result.effects.is_empty());
        
        TransactionExecutor::apply_effects(&withdraw_result.effects, &mut state).unwrap();
        
        let cash_post = state.financial_system.get_cash_assets(&consumer_id);
        println!("Cash after withdrawal: {}", cash_post);
        println!("Effects: {:?}", withdraw_result.effects);
        
        assert_eq!(cash_post, 700.0, "Should have 700 cash (500 + 200 withdrawn)");
        
        let deposits = state.financial_system.get_deposits_at_bank(&consumer_id, &bank_id);
        assert_eq!(deposits, 300.0, "Should have 300 left in deposits");
    }
}