#[cfg(test)]
mod simulation_flow_tests {
    use crate::{
        initialize_economy_from_scenario,
        state::Scenario,
        SimState, TransactionExecutor,
    };
    use rand::prelude::*;
    use ravelin_core::*;
    use std::collections::HashMap;

    #[derive(Debug)]
    struct TestHarness {
        ss: SimState,
        bank_ids: HashMap<String, AgentId>,
        firm_ids: HashMap<String, AgentId>,
        consumer_ids: HashMap<String, AgentId>,
        cb_id: AgentId,
    }

    fn setup() -> TestHarness {
        let test_toml = r#"
            name = "Test Harness Scenario"
            description = "A minimal, predictable scenario for integration testing."

            [config]
            iterations = 10
            treasury_tenors_to_register = ["T10Y"]

            [[banks]]
            id = "test_bank"
            name = "Test Bank"
            initial_reserves = 1000000.0
            initial_bonds = []

            [[firms]]
            id = "test_firm"
            name = "Test Firm"
            bank_id = "test_bank"
            recipe_name = "Oil Refining"
            initial_cash = 20000.0
            initial_inventory = [
                { good_slug = "oil", quantity = 100.0, unit_cost = 50.0 }
            ]

            [[consumers]]
            id = "test_consumer"
            bank_id = "test_bank"
            initial_cash = 5000.0
            income = 52000.0 # $1000/week
        "#;

        let scenario: Scenario = Scenario::from_toml(test_toml).expect("Failed to parse test TOML");
        let ss = initialize_economy_from_scenario(&scenario, &mut StdRng::seed_from_u64(0));

        let bank_ids: HashMap<String, AgentId> = ss
            .financial_system
            .commercial_banks
            .values()
            .filter_map(|b| {
                scenario.banks.iter().find(|bc| bc.name == b.name).map(|bc| (bc.id.clone(), b.id))
            })
            .collect();

        let firm_ids: HashMap<String, AgentId> = ss
            .firms
            .iter()
            .filter_map(|f| {
                scenario.firms.iter().find(|fc| fc.name == f.name).map(|fc| (fc.id.clone(), f.id))
            })
            .collect();

        let consumer_ids: HashMap<String, AgentId> = ss
            .consumers
            .iter()
            .filter_map(|c| {
                scenario
                    .consumers
                    .iter()
                    .find(|cc| (cc.income / 52.0 - c.income).abs() < 1e-6)
                    .map(|cc| (cc.id.clone(), c.id))
            })
            .collect();

        let cb_id = ss.financial_system.central_bank.id.clone();

        TestHarness { ss, bank_ids, firm_ids, consumer_ids, cb_id }
    }

    #[test]
    fn test_scenario_initialization() {
        let harness = setup();
        let ss = &harness.ss;

        assert_eq!(ss.consumers.len(), 1);
        assert_eq!(ss.firms.len(), 1);
        assert_eq!(ss.financial_system.commercial_banks.len(), 1);

        let bank_id = harness.bank_ids.get("test_bank").unwrap();
        let bank = ss.financial_system.commercial_banks.get(bank_id).unwrap();
        assert_eq!(bank.get_reserves(&ss.financial_system), 1_000_000.0);

        let consumer_id = harness.consumer_ids.get("test_consumer").unwrap();
        assert_eq!(ss.financial_system.get_cash_assets(consumer_id), 5000.0);
    }

    #[test]
    fn test_deposit_action_flow() {
        let mut harness = setup();
        let consumer_id = harness.consumer_ids.get("test_consumer").unwrap().clone();
        let bank_id = harness.bank_ids.get("test_bank").unwrap().clone();
        let deposit_amount = 1000.0;

        let action = SimAction::Deposit { agent_id: consumer_id.clone(), bank: bank_id.clone(), amount: deposit_amount };

        let er = TransactionExecutor::execute(&action, &mut harness.ss);
        assert!(er.success, "Deposit execution should succeed");
        TransactionExecutor::apply(&er.effects, &mut harness.ss).unwrap();

        let fs = &harness.ss.financial_system;
        let consumer_cash = fs.get_cash_assets(&consumer_id);
        let consumer_deposits = fs.get_deposits_at_bank(&consumer_id, &bank_id);

        assert_eq!(consumer_cash, 4000.0, "Consumer cash should be reduced");
        assert_eq!(consumer_deposits, deposit_amount, "Consumer should have a new deposit");
    }
}