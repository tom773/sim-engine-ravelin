use shared::*;
use crate::state::SimState;
use uuid::Uuid;
use std::collections::HashMap;

pub mod domain;
pub mod executor;
pub mod effects;

use domain::*;
pub use executor::*;
pub use effects::*;

// Lifecycle of a tick:
/* 
    1. Call <agent>.decide() to get their decisions.
    2. Collect all actions via <agent>.act().
    3. Convert each action to sim actions.
    4. Run TransactionExecutor::execute_action() to collect effects of actions.
    5. Run TransactionExecutor::apply_effects() to apply the effects of the actions. 
    6. Update the SimState with the new tick number.
*/
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentFactory, TransactionExecutor, initialize_economy, state::SimState};
    use rand::prelude::*;
    use shared::SimAction;
    use uuid::Uuid;

    fn t_econ() -> SimState {
        let mut rng = StdRng::from_os_rng();
        let mut ss = SimState::default();
        let mut factory = AgentFactory::new(&mut ss, &mut rng);

        let bank = factory.create_bank();
        for _ in 0..2 {
            factory.create_firm(bank.id.clone());
        }
        for _ in 0..2 {
            factory.create_consumer(bank.id.clone());
        }
        ss
    }
    fn t_econ_w_liquidity() -> SimState {
        let mut state = t_econ();
        let e = TransactionExecutor::execute_inject_liquidity(&state);
        TransactionExecutor::apply_effects(&e.effects, &mut state);
        state
    }
    #[test]
    fn test_new_econ() {
        let state = t_econ();

        let consumer_length = state.consumers.len();
        let firm_length = state.firms.len();
        let banks_length = state.financial_system.commercial_banks.len();

        assert!(consumer_length > 0, "No consumers created");
        assert!(firm_length > 0, "No firms created");
        assert!(banks_length > 0, "No banks created");

        let num_agents = consumer_length + firm_length + banks_length;
        assert!(
            state.financial_system.balance_sheets.len() == num_agents + 1,
            "Balance sheets do not match number of agents"
        );
    }

    #[test]
    fn test_li() {
        let mut state = t_econ();
        let e = TransactionExecutor::execute_inject_liquidity(&state);
        TransactionExecutor::apply_effects(&e.effects, &mut state);
        assert!(e.success, "Inject liquidity failed: {:?}", e.errors);
        let consumer_assets = state
            .consumers
            .iter()
            .map(|c| c.get_cash_holdings(&state.financial_system))
            .sum::<f64>();
        let bank_assets = state
            .financial_system
            .commercial_banks
            .values()
            .map(|b| state.financial_system.get_bank_reserves(&b.id).unwrap())
            .sum::<f64>();

        assert!(
            consumer_assets > 0.0,
            "Consumers should have cash holdings after liquidity injection"
        );
        assert!(
            bank_assets > 0.0,
            "Banks should have cash holdings after liquidity injection"
        );

        let m0 = state.financial_system.m0();
        let m1 = state.financial_system.m1();
        let m2 = state.financial_system.m2();

        assert!(
            m0 == 11000.0,
            "M0 should be equal to 11000 (10k intiial bank reserves + 1k consumer cash) after liquidity injection"
        );
        assert!(
            m1 == 1000.0,
            "M1 should be equal to 1000 (1k consumer cash) after liquidity injection"
        );
        assert!(
            m2 == 1000.0,
            "M2 should be equal to 1000 (1k consumer cash) after liquidity injection)"
        );
    }
    #[test]
    fn test_execute_deposit() {
        let mut state = t_econ_w_liquidity();

        let (consumer_opt, firm_opt, bank_opt) = state.get_first_agents();
        let consumer = consumer_opt.unwrap();
        let bank = bank_opt.unwrap();

        let amount = 100.0;
        let consumer_id = consumer.id.clone();
        let bank_id = bank.id.clone();

        let consumer_cash_before = consumer.get_cash_holdings(&state.financial_system);
        let bank_liabilities_before = bank.total_liabilities(&state.financial_system);

        let action = SimAction::Deposit {
            agent_id: consumer_id.clone(),
            bank: bank_id.clone(),
            amount,
        };

        let result_action = TransactionExecutor::execute_action(&action, &state);
        assert!(
            result_action.success,
            "Deposit action failed: {:?}",
            result_action.errors
        );

        let result = TransactionExecutor::apply_effects(&result_action.effects, &mut state);
        assert!(result.is_ok(), "Failed to apply effects: {:?}", result);

        let consumer_cash_after = state.financial_system.get_cash_assets(&consumer_id);
        let bank_liabilities_after = state.financial_system.get_total_liabilities(&bank_id);

        assert!(
            consumer_cash_before >= amount,
            "Consumer should have enough cash to deposit"
        );
        assert_eq!(
            consumer_cash_after,
            consumer_cash_before - amount,
            "Consumer cash holdings should be reduced by deposit amount"
        );
        assert_eq!(
            bank_liabilities_after,
            bank_liabilities_before + amount,
            "Bank liabilities should increase by the deposit amount"
        );
    }
}
