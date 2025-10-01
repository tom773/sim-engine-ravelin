use engine_v3::{Scenario, SimulationEngine};
use rand::{rngs::StdRng, SeedableRng};
use rust_decimal_macros::dec;
use sim_core::actions::banking::TransactionContext;
use sim_core::prelude::*;
use sim_core::types::core_utils::time::decimal_to_bps;
use sim_core::types::instrument::credit::OvernightCreditType;
use sim_core::types::instrument::inst_core::CreditState;
use sim_core::types::markets::overnight::{ONQuote, ONQuoteSide, OvernightVenue};

fn create_banking_engine() -> SimulationEngine {
    let scenario_toml = include_str!("fixtures/interbank_test.toml");
    let scenario = Scenario::from_toml_str(scenario_toml).expect("failed to parse test scenario");
    scenario.initialize_engine()
}

fn get_balance_sheet(engine: &SimulationEngine, agent_id: AgentId) -> Option<&BalanceSheet> {
    engine.state.financial_system.balance_sheets.get(&agent_id)
}

fn get_interbank_loans(engine: &SimulationEngine) -> Vec<(InstrumentId, OvernightCreditState)> {
    engine.state
        .financial_system
        .instruments
        .instruments
        .iter()
        .filter_map(|(inst_id, inst)| {
            if let InstrumentRuntime::Credit(CreditState::OvernightCredit(credit)) = inst.state() {
                if credit.credit_type == OvernightCreditType::InterbankLoan {
                    return Some((*inst_id, credit.clone()));
                }
            }
            None
        })
        .collect()
}

fn bank_id_by_name(engine: &SimulationEngine, name: &str) -> AgentId {
    engine
        .state
        .agents
        .banks
        .iter()
        .find_map(|(id, bank)| (bank.name == name).then_some(*id))
        .expect("expected bank to exist in scenario")
}

#[test]
fn test_full_interbank_pipeline_with_intentions() {
    let mut engine = create_banking_engine();
    let mut rng = StdRng::seed_from_u64(42);

    let lender_id = bank_id_by_name(&engine, "Bank 01");
    let borrower_id = bank_id_by_name(&engine, "Bank 02");
    assert_ne!(lender_id, borrower_id, "lender and borrower should be distinct banks");

    let loan_amount = 50_000.0;
    let lend_rate_bps = decimal_to_bps(dec!(0.004));
    let borrow_rate_bps = decimal_to_bps(dec!(0.005));

    let lend_quote = ONQuote {
        venue: OvernightVenue::FedFundsON,
        agent: lender_id,
        side: ONQuoteSide::Lend,
        notional: loan_amount,
        limit_rate_bps: lend_rate_bps,
        haircut: None,
        preferred_collateral: None,
        min_fill: 0.0,
        ts: 0,
    };

    let borrow_quote = ONQuote {
        venue: OvernightVenue::FedFundsON,
        agent: borrower_id,
        side: ONQuoteSide::Borrow,
        notional: loan_amount,
        limit_rate_bps: borrow_rate_bps,
        haircut: None,
        preferred_collateral: None,
        min_fill: 0.0,
        ts: 0,
    };

    engine.state.financial_system.funding_markets.post_quote(lend_quote);
    engine.state.financial_system.funding_markets.post_quote(borrow_quote);

    // Check for quotes
    let quote_count = engine.state.financial_system.funding_markets.fedfunds_on.len();
    assert_eq!(quote_count, 2, "should have 2 quotes in the market");

    let date_before_run = engine.state.current_date;

    // Check for successful day
    let (day_results, _events) = engine.run_day(&mut rng);
    assert!(day_results.iter().all(|r| r.success), "all sessions should execute successfully");

    let date_after_run = engine.state.current_date;
    assert!(date_after_run > date_before_run, "simulation date should advance after run_day");

    // Check for loan creation
    let interbank_loans = get_interbank_loans(&engine);
    assert!(!interbank_loans.is_empty(), "should have created interbank loan");

    // Identify banks loans and ensure they exist
    let tracked_loans: Vec<_> = interbank_loans
        .iter()
        .filter(|(_, credit)| credit.lender == lender_id && credit.borrower == borrower_id)
        .map(|(loan_id, credit)| (*loan_id, credit.clone()))
        .collect();
    assert!(!tracked_loans.is_empty(), "expected at least one loan between selected lender and borrower");
    let created_loan_ids: Vec<_> = tracked_loans.iter().map(|(id, _)| *id).collect();

    // Validate loan details
    for (_, credit) in &tracked_loans {
        assert_eq!(credit.lender, lender_id, "loan should record expected lender"); 
        assert_eq!(credit.borrower, borrower_id, "loan should record expected borrower");
        let amount = credit.amount.to_f64();
        assert!(
            (amount - loan_amount).abs() < 1.0,
            "loan amount should be close to requested amount (expected {}, got {})",
            loan_amount,
            amount
        );
        assert!(
            credit.maturity_date > credit.issue_date,
            "overnight credit should mature after it is issued"
        );
    }

    let lender_bs_post = get_balance_sheet(&engine, lender_id).expect("lender should have balance sheet");
    let borrower_bs_post = get_balance_sheet(&engine, borrower_id).expect("borrower should have balance sheet");

    // Assert lender has loan asset and borrower has loan liability
    for loan_id in &created_loan_ids {
        assert!(
            lender_bs_post.assets.contains_key(loan_id),
            "lender should have loan asset {loan_id} on balance sheet"
        );
        assert!(
            borrower_bs_post.liabilities.contains_key(loan_id),
            "borrower should have loan liability {loan_id}"
        );
    }

    let initial_loan_funding_payments = &engine.state.financial_system.rtgs.settled;
    for loan_id in &created_loan_ids {
        // Asset there is a settled payment for the loan disbursement
        assert!(
            initial_loan_funding_payments.iter().any(|pi| {
                matches!(
                    &pi.context,
                    TransactionContext::InterbankLoan { loan_id: settled_loan_id, lender, borrower }
                        if settled_loan_id == loan_id && *lender == lender_id && *borrower == borrower_id
                ) && pi.payer == lender_id && pi.payee == borrower_id
            }),
            "expected interbank loan disbursement payment for loan {}",
            loan_id
        );
        // Assert no pending payment for the loan disbursement remains
        assert!(
            !engine
                .state
                .financial_system
                .rtgs
                .pending
                .iter()
                .any(|pi| {
                    matches!(
                        &pi.context,
                        TransactionContext::InterbankLoan { loan_id: queued_loan_id, .. }
                            if queued_loan_id == loan_id
                    )
                }),
            "interbank loan disbursement for loan {} should not remain pending after RTGS run",
            loan_id
        );
    }
    // Advance to next day to process maturity
    let (maturity_results, _) = engine.run_day(&mut rng);
    assert!(
        maturity_results.iter().all(|r| r.success),
        "maturity day should execute successfully: {:?}",
        maturity_results
    );

    // Assert loans are marked as matured
    let loans_after_maturity = get_interbank_loans(&engine);
    assert!(
        tracked_loans
            .iter()
            .all(|(_, credit)| engine.state.current_date >= credit.maturity_date),
        "tracked loans should have reached maturity"
    );

    let lender_bs_final = get_balance_sheet(&engine, lender_id).expect("lender should have balance sheet");
    let borrower_bs_final = get_balance_sheet(&engine, borrower_id).expect("borrower should have balance sheet");

    // Assert loans have been removed from balance sheets and instrument catalog
    for loan_id in &created_loan_ids {
        assert!(
            !loans_after_maturity.iter().any(|(id, _)| id == loan_id),
            "loan {} should be removed from instrument catalog after maturity",
            loan_id
        );
        assert!(
            !lender_bs_final.assets.contains_key(loan_id),
            "lender should no longer hold loan asset {} after redemption",
            loan_id
        );
        assert!(
            !borrower_bs_final.liabilities.contains_key(loan_id),
            "borrower should no longer owe loan liability {} after redemption",
            loan_id
        );
    }
    // Assert principal repayment has occurred
    let settled_payments_since_maturity = &engine.state.financial_system.rtgs.settled;
    for loan_id in &created_loan_ids {
        assert!(
            settled_payments_since_maturity.iter().any(|pi| {
                matches!(
                    &pi.context,
                    TransactionContext::PrincipalRepayment { instrument_id }
                        if instrument_id == loan_id
                ) && pi.payer == borrower_id && pi.payee == lender_id
            }),
            "expected principal repayment settlement for loan {}. Settled payments: {:?}",
            loan_id,
            settled_payments_since_maturity
        );
        assert!(
            !engine
                .state
                .financial_system
                .rtgs
                .pending
                .iter()
                .any(|pi| matches!(&pi.context, TransactionContext::PrincipalRepayment { instrument_id } if instrument_id == loan_id)),
            "no principal repayments for loan {} should remain pending after RTGS run",
            loan_id
        );
    }
}
