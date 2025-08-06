pub mod behavior;
pub mod domain;

pub use behavior::*;
pub use domain::*;

#[cfg(test)]
mod tests {
    use super::*;
    use sim_prelude::*;
    use uuid::Uuid;

    fn setup_banking_test_state() -> (SimState, AgentId, AgentId, AgentId, AgentId) {
        let mut state = SimState::default();
        let payer_id = AgentId(Uuid::new_v4());
        let recipient_id = AgentId(Uuid::new_v4());
        let bank_id = AgentId(Uuid::new_v4());
        let cb_id = state.financial_system.central_bank.id;

        state.agents.consumers.insert(payer_id, Consumer::new(30, bank_id, PersonalityArchetype::Balanced));
        state.agents.consumers.insert(recipient_id, Consumer::new(40, bank_id, PersonalityArchetype::Spender));
        state.agents.banks.insert(bank_id, Bank::new("Test Bank".to_string(), 0.0, 0.0));
        state.financial_system.balance_sheets.insert(payer_id, BalanceSheet::new(payer_id));
        state.financial_system.balance_sheets.insert(recipient_id, BalanceSheet::new(recipient_id));
        state.financial_system.balance_sheets.insert(bank_id, BalanceSheet::new(bank_id));

        state.financial_system.create_instrument(cash!(payer_id, 50.0, cb_id, state.current_date)).unwrap();
        state.financial_system.create_instrument(deposit!(payer_id, bank_id, 200.0, 0.01, state.current_date)).unwrap();
        state.financial_system.create_instrument(reserves!(bank_id, cb_id, 500.0, state.current_date)).unwrap();

        (state, payer_id, recipient_id, bank_id, cb_id)
    }

    #[test]
    fn test_transfer_uses_cash_when_sufficient() {
        let (mut state, payer_id, recipient_id, _, _) = setup_banking_test_state();
        let domain = BankingDomain::new();
        let transfer_amount = 40.0;

        let result = domain.execute_transfer(payer_id, recipient_id, transfer_amount, &state);
        assert!(result.success);
        state.apply_effects(&result.effects).unwrap();

        assert_eq!(state.financial_system.get_cash_assets(&payer_id), 10.0);
        assert_eq!(state.financial_system.get_cash_assets(&recipient_id), transfer_amount);
        let payer_bs = state.financial_system.get_bs_by_id(&payer_id).unwrap();
        let deposit = payer_bs.assets.values().find(|i| i.details.as_any().is::<DemandDepositDetails>()).unwrap();
        assert_eq!(deposit.principal, 200.0);
    }

    #[test]
    fn test_composite_transfer_uses_cash_then_deposits() {
        let (mut state, payer_id, recipient_id, bank_id, cb_id) = setup_banking_test_state();
        let domain = BankingDomain::new();
        let transfer_amount = 150.0;

        println!("Initial Consumer BS: {:?}", state.financial_system.get_bs_by_id(&payer_id));
        println!("Initial Bank BS: {:?}", state.financial_system.get_bs_by_id(&bank_id));
        println!("Initial Central Bank BS: {:?}", state.financial_system.get_bs_by_id(&cb_id));

        let result = domain.execute_transfer(payer_id, recipient_id, transfer_amount, &state);
        assert!(result.success, "Transfer should succeed using composite funds");

        state.apply_effects(&result.effects).unwrap();

        assert_eq!(state.financial_system.get_cash_assets(&payer_id), 0.0);
        let payer_bs = state.financial_system.get_bs_by_id(&payer_id).unwrap();
        let deposit = payer_bs.assets.values().find(|i| i.details.as_any().is::<DemandDepositDetails>()).unwrap();
        assert!((deposit.principal - 100.0).abs() < 1e-6, "Deposit should be 200 - 100");
        assert!(
            (state.financial_system.get_bank_reserves(&bank_id).unwrap() - 400.0).abs() < 1e-6,
            "Reserves should be 500 - 100"
        );
        assert!((state.financial_system.get_cash_assets(&recipient_id) - transfer_amount).abs() < 1e-6);
        assert!(state.financial_system.get_total_liabilities(&cb_id) == 550.0, "Central bank should have liabilities after transfer, has {}", state.financial_system.get_total_liabilities(&cb_id));
        
        println!("Final Consumer BS: {:?}", state.financial_system.get_bs_by_id(&payer_id));
        println!("Final Bank BS: {:?}", state.financial_system.get_bs_by_id(&bank_id));
        println!("Final Central Bank BS: {:?}", state.financial_system.get_bs_by_id(&cb_id));
    }

    #[test]
    fn test_transfer_fails_when_all_funds_insufficient() {
        let (state, payer_id, recipient_id, _, _) = setup_banking_test_state();
        let domain = BankingDomain::new();
        let transfer_amount = 300.0; // Payer has only $250 total liquid assets.

        // 1. Create the action enum instance
        let action = BankingAction::Transfer { from: payer_id, to: recipient_id, amount: transfer_amount };

        // 2. Call the main `execute` function, which runs validation first.
        let execution_result = domain.execute(&action, &state);

        // 3. Assert that the execution failed as expected.
        assert!(!execution_result.success, "Execution should fail due to validation");
        assert!(execution_result.effects.is_empty(), "No effects should be generated on failure");
        assert!(execution_result.errors.iter().any(|e| e.contains("Insufficient liquid assets")));
    }
}
