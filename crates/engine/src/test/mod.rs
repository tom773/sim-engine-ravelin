#[cfg(test)]
mod simulation_flow_tests {
    use crate::{SimConfig, SimState, TransactionExecutor, initialize_economy};
    use rand::prelude::*;
    use shared::*;
    use uuid::Uuid;

    #[derive(Debug)]
    struct TestScenario {
        ss: SimState,
        consumer_id: AgentId,
        firm_id: AgentId,
        bank_id: AgentId,
        cb_id: AgentId,
    }

    fn setup() -> TestScenario {
        let mut ss = initialize_economy(
            &SimConfig { consumer_count: 1, firm_count: 1, ..Default::default() },
            &mut StdRng::seed_from_u64(0),
        );

        let consumer_id = ss.consumers.first().unwrap().id.clone();
        let firm_id = ss.firms.first().unwrap().id.clone();
        let bank_id = ss.financial_system.commercial_banks.values().next().unwrap().id.clone();
        let cb_id = ss.financial_system.central_bank.id.clone();

        let cash = cash!(consumer_id.clone(), 5000.0, cb_id.clone(), 0);
        ss.financial_system.create_or_consolidate_instrument(cash).unwrap();

        if let Some(bs) = ss.financial_system.balance_sheets.get_mut(&firm_id) {
            bs.add_to_inventory(&good_id!("oil"), 100.0, 50.0);
        }

        TestScenario { ss, consumer_id, firm_id, bank_id, cb_id }
    }

    #[test]
    fn test_scenario_initialization() {
        let scenario = setup();
        let ss = &scenario.ss;

        assert_eq!(ss.consumers.len(), 1, "Should have one consumer");
        assert_eq!(ss.firms.len(), 1, "Should have one firm");
        assert_eq!(ss.financial_system.commercial_banks.len(), 2, "Should have two banks from default init");

        let bank = ss.financial_system.commercial_banks.get(&scenario.bank_id).unwrap();
        assert_eq!(bank.get_reserves(&ss.financial_system), 1000.0, "Bank initial reserves should be 1000.0");

        let consumer_cash = ss.financial_system.get_cash_assets(&scenario.consumer_id);
        assert_eq!(consumer_cash, 5000.0, "Consumer should have 5000.0 initial cash from test setup");
    }

    #[test]
    fn test_deposit_action_flow() {
        let mut scenario = setup();
        let (consumer_id, bank_id) = (scenario.consumer_id.clone(), scenario.bank_id.clone());
        let deposit_amount = 1000.0;

        let action =
            SimAction::Deposit { agent_id: consumer_id.clone(), bank: bank_id.clone(), amount: deposit_amount };

        let er = TransactionExecutor::execute(&action, &mut scenario.ss);
        assert!(er.success, "Deposit execution should succeed");
        assert!(!er.effects.is_empty(), "Execution should produce effects");

        TransactionExecutor::apply(&er.effects, &mut scenario.ss).unwrap();

        let fs = &scenario.ss.financial_system;
        let consumer_cash = fs.get_cash_assets(&consumer_id);
        let consumer_deposits = fs.get_deposits_at_bank(&consumer_id, &bank_id);
        let bank_reserves = fs.get_bank_reserves(&bank_id).unwrap_or(0.0);

        assert_eq!(consumer_cash, 4000.0, "Consumer cash should be reduced by deposit amount");
        assert_eq!(consumer_deposits, deposit_amount, "Consumer should have a new deposit");
        assert!(
            bank_reserves >= 1000.0,
            "Bank reserves would either remain flat or increase by 10% of the deposit amount"
        );
    }

    #[test]
    fn test_withdrawal_action_flow() {
        let mut scenario = setup();
        let (consumer_id, bank_id) = (scenario.consumer_id.clone(), scenario.bank_id.clone());
        let deposit_amount = 2000.0;
        let withdrawal_amount = 500.0;

        let deposit_action =
            SimAction::Deposit { agent_id: consumer_id.clone(), bank: bank_id.clone(), amount: deposit_amount };
        let er = TransactionExecutor::execute(&deposit_action, &mut scenario.ss);
        TransactionExecutor::apply(&er.effects, &mut scenario.ss).unwrap();

        let withdraw_action =
            SimAction::Withdraw { agent_id: consumer_id.clone(), bank: bank_id.clone(), amount: withdrawal_amount };

        let er = TransactionExecutor::execute(&withdraw_action, &mut scenario.ss);
        assert!(er.success, "Withdrawal should succeed");
        TransactionExecutor::apply(&er.effects, &mut scenario.ss).unwrap();

        let fs = &scenario.ss.financial_system;
        let consumer_cash = fs.get_cash_assets(&consumer_id);
        let consumer_deposits = fs.get_deposits_at_bank(&consumer_id, &bank_id);

        assert_eq!(consumer_cash, 3500.0, "Consumer cash should be correct after withdrawal");
        assert_eq!(consumer_deposits, 1500.0, "Consumer deposits should be reduced");
    }

    #[test]
    fn test_invalid_deposit_fails_validation() {
        let mut scenario = setup();
        let (consumer_id, bank_id) = (scenario.consumer_id.clone(), scenario.bank_id.clone());

        let action = SimAction::Deposit { agent_id: consumer_id.clone(), bank: bank_id.clone(), amount: 9999.0 };

        let er = TransactionExecutor::execute(&action, &mut scenario.ss);

        assert!(!er.success, "Execution should fail due to validation");
        assert!(!er.errors.is_empty(), "Execution should return validation errors");
        assert!(er.effects.is_empty(), "Failed execution should have no effects");
    }

    #[test]
    fn test_consumer_personality() {
        let scenario = setup();
        let consumer = scenario.ss.consumers.first().unwrap();

        assert_eq!(consumer.personality, PersonalityArchetype::Balanced, "Consumer should have balanced personality");
        assert_eq!(consumer.personality.get_params().prop_to_consume, 0.7, "Consumer should have correct propensity to consume");
    }
    
    #[test]
    fn test_labour_market() {
        let mut scenario = setup();
        let (firm_id, consumer_id) = (scenario.firm_id.clone(), scenario.consumer_id.clone());
        let firms = &scenario.ss.firms;
        let firm = firms.iter().find(|f| f.id == firm_id).unwrap();
        
        let action = SimAction::PayWages {
            agent_id: firm_id.clone(),
            employee: consumer_id.clone(),
            amount: 100.0,
        };
        let m0 = scenario.ss.financial_system.m0();
        let m1 = scenario.ss.financial_system.m1();


        let er = TransactionExecutor::execute(&action, &mut scenario.ss);
        TransactionExecutor::apply(&er.effects, &mut scenario.ss).unwrap();

        let consumer_cash_after = scenario.ss.financial_system.get_cash_assets(&consumer_id);
        let firm_cash_after = scenario.ss.financial_system.get_cash_assets(&firm_id);
        let m0_after = scenario.ss.financial_system.m0();
        let m1_after = scenario.ss.financial_system.m1();

        assert_eq!(consumer_cash_after, 5100.0, "Consumer should receive wages");
        assert_eq!(firm_cash_after, 19900.0, "Firm should pay wages");
        assert_eq!(m0_after, m0, "M0 should not decrease");
        assert_eq!(m1_after, m1, "M1 should increase by wages paid");
        assert_eq!(scenario.ss.financial_system.get_bs_by_id(&firm_id).unwrap().assets.len(), 1, "Firm should still have one asset after paying wages");
        assert_eq!(scenario.ss.financial_system.get_bs_by_id(&consumer_id).unwrap().assets.len(), 1, "Consumer should still have one asset after receiving wages");
        
    }
    #[test]
    fn test_firm_validation() {
        let mut scenario = setup();
        let firm_id = scenario.firm_id.clone();
        let firm = scenario.ss.firms.iter().find(|f| f.id == firm_id).unwrap();

        // Test production validation
        let action = SimAction::Produce {
            agent_id: firm_id.clone(),
            recipe_id: firm.recipe.clone().unwrap(),
            batches: 1,
        };
        let er = TransactionExecutor::execute(&action, &mut scenario.ss);
        assert!(er.success, "Production should be valid");
        
        // Test hiring validation
        let hire_action = SimAction::Hire {
            agent_id: firm_id.clone(),
            count: 5,
        };
        let er = TransactionExecutor::execute(&hire_action, &mut scenario.ss);
        assert!(er.success, "Hiring should be valid");
        
        let post_ask_action = SimAction::PostAsk {
            agent_id: firm_id.clone(),
            market_id: MarketId::Goods(good_id!("oil")),
            price: 100.0,
            quantity: 10.0,
        };
        let er = TransactionExecutor::execute(&post_ask_action, &mut scenario.ss);
        assert!(er.success, "Posting ask should be valid");

        let fail_post_ask_action = SimAction::PostAsk {
            agent_id: firm_id.clone(),
            market_id: MarketId::Goods(good_id!("oil")),
            price: 100.0,
            quantity: 201.0,
        };
        let er = TransactionExecutor::execute(&fail_post_ask_action, &mut scenario.ss);
        assert!(!er.success, "Posting ask for more goods than have should fail validation");

    } 
}
