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

        let er = TransactionExecutor::execute_action(&action, &scenario.ss);
        assert!(er.success, "Deposit execution should succeed");
        assert!(!er.effects.is_empty(), "Execution should produce effects");

        TransactionExecutor::apply_effects(&er.effects, &mut scenario.ss).unwrap();

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
        let er = TransactionExecutor::execute_action(&deposit_action, &scenario.ss);
        TransactionExecutor::apply_effects(&er.effects, &mut scenario.ss).unwrap();

        let withdraw_action =
            SimAction::Withdraw { agent_id: consumer_id.clone(), bank: bank_id.clone(), amount: withdrawal_amount };

        let er = TransactionExecutor::execute_action(&withdraw_action, &scenario.ss);
        assert!(er.success, "Withdrawal should succeed");
        TransactionExecutor::apply_effects(&er.effects, &mut scenario.ss).unwrap();

        let fs = &scenario.ss.financial_system;
        let consumer_cash = fs.get_cash_assets(&consumer_id);
        let consumer_deposits = fs.get_deposits_at_bank(&consumer_id, &bank_id);

        assert_eq!(consumer_cash, 3500.0, "Consumer cash should be correct after withdrawal");
        assert_eq!(consumer_deposits, 1500.0, "Consumer deposits should be reduced");
    }

    #[test]
    fn test_invalid_deposit_fails_validation() {
        let scenario = setup();
        let (consumer_id, bank_id) = (scenario.consumer_id.clone(), scenario.bank_id.clone());

        let action = SimAction::Deposit { agent_id: consumer_id.clone(), bank: bank_id.clone(), amount: 9999.0 };

        let er = TransactionExecutor::execute_action(&action, &scenario.ss);

        assert!(!er.success, "Execution should fail due to validation");
        assert!(!er.errors.is_empty(), "Execution should return validation errors");
        assert!(er.effects.is_empty(), "Failed execution should have no effects");
        println!("Validation failed as expected: {:?}", er.errors);
    }

    #[test]
    fn test_full_tick_flow() {
        let mut scenario = setup();
        let initial_firm_employees = scenario.ss.firms.first().unwrap().employees;
        let initial_tick = scenario.ss.ticknum;

        let (_ss, actions, effects) = crate::tick(&mut scenario.ss);

        assert_eq!(scenario.ss.ticknum, initial_tick + 1, "Tick number should increment");
        assert!(!actions.is_empty(), "A tick should generate actions");
        assert!(!effects.is_empty(), "A tick should generate and apply effects");

        let final_firm_employees = scenario.ss.firms.first().unwrap().employees;
        assert!(final_firm_employees > initial_firm_employees, "Firm should have hired new employees");

        let consumer_deposits =
            scenario.ss.financial_system.get_deposits_at_bank(&scenario.consumer_id, &scenario.bank_id);
        assert!(consumer_deposits > 0.0, "Consumer should have deposited some savings");
    }
}
