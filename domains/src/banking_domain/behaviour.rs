use rand::RngCore;
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::any::Any;

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct BasicBankDecisionModel;

#[typetag::serde]
impl DecisionModel for BasicBankDecisionModel {
    fn name(&self) -> &str {
        "Bank"
    }
    fn decide(&self, agent: &dyn Any, state: &SimState, rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let bank = match agent.downcast_ref::<Bank>() {
            Some(b) => b,
            None => return vec![],
        };

        let mut intentions = Vec::new();
        let fs = &state.financial_system;

        self.assess_liquidity_needs(bank, fs, &mut intentions);
        self.handle_debt_auctions(bank, state, fs.get_liquid_assets(&bank.id), &mut intentions, rng);
        self.consider_treasury_market_making(bank, state, &mut intentions, rng);
        self.process_loan_applications(bank, state, &mut intentions);
        intentions
    }
}

impl BasicBankDecisionModel {
    fn process_loan_applications(&self, bank: &Bank, state: &SimState, intentions: &mut Vec<SimIntention>) {
        let fs = &state.financial_system;
        let app_ids = fs.credit_registry.applications_by_bank.get(&bank.id).cloned().unwrap_or_default();

        use std::collections::HashSet;
        let mut seen_borrowers: HashSet<AgentId> = HashSet::new();

        for app_id in app_ids {
            let Some(app) = fs.credit_registry.applications.get(&app_id) else {
                continue;
            };
            if !matches!(app.status, ApplicationStatus::Pending) {
                continue;
            }
            if !seen_borrowers.insert(app.borrower_id) {
                continue;
            }


            let decision = if let Some(borrower) = state.agents.consumers.get(&app.borrower_id) {
                let dti =
                    if borrower.income > 0.0 { fs.get_total_liabilities(&borrower.id) / borrower.income } else { 1.0 };
                if dti < 0.4 && borrower.income > 1000.0 {
                    LoanDecision::Approve {
                        terms: LoanTerms {
                            principal: app.requested_amount,
                            annual_rate_bps: rust_decimal_macros::dec!(850),
                            term_months: 60,
                            payment_frequency: PaymentFrequency::Monthly,
                            collateral_required: false,
                            loan_to_value_ratio: None,
                        },
                    }
                } else {
                    LoanDecision::Reject { reason: format!("DTI too high ({dti:.2}) or income too low") }
                }
            } else {
                LoanDecision::Reject { reason: "Borrower not found".to_string() }
            };

            match decision {
                LoanDecision::Approve { terms } => {
                    intentions.push(SimIntention::Banking(BankingIntention::ApproveLoan {
                        bank_id: bank.id,
                        borrower_id: app.borrower_id,
                        amount: terms.principal,
                        terms,
                    }));
                }
                LoanDecision::Reject { reason } => {
                    intentions.push(SimIntention::Banking(BankingIntention::RejectLoan {
                        bank_id: bank.id,
                        borrower_id: app.borrower_id,
                        application_id: app.application_id,
                        reason,
                    }));
                }
                LoanDecision::CounterOffer { .. } => { /* implement if desired */ }
            }
        }
    }

    pub fn assess_liquidity_needs(&self, bank: &Bank, fs: &FinancialSystem, intentions: &mut Vec<SimIntention>) {
        let bank_bs = match fs.balance_sheets.get(&bank.id) {
            Some(bs) => bs,
            None => return,
        };

        let total_deposits = bank_bs
            .liabilities
            .iter()
            .filter_map(|(id, pos)| {
                fs.instruments.get(id).and_then(|inst| {
                    if let InstrumentType::Cash(d) = &inst.instrument_type {
                        if matches!(d.cash_type, CashType::DemandDeposit | CashType::SavingsDeposit) {
                            return Some(pos.quantity);
                        }
                    }
                    None
                })
            })
            .sum::<f64>();

        let required_reserves = total_deposits * fs.central_bank.reserve_requirement;
        let desired_buffer = total_deposits * 0.02;
        let target_reserve_level = required_reserves + desired_buffer;

        let current_reserves = fs.get_bank_reserves(&bank.id).unwrap_or(0.0);
        let reserve_surplus_or_shortfall = current_reserves - target_reserve_level;

        let policy_rate_bps = fs.central_bank.policy_rate_bps;
        let acceptable_rate_range = 25.0;
        let target_rate_bps = policy_rate_bps;

        if reserve_surplus_or_shortfall < -1.0 {
            let amount_needed = -reserve_surplus_or_shortfall;
            let borrow_rate = target_rate_bps + Decimal::from_f64(acceptable_rate_range / 2.0).unwrap_or_default();
            intentions.push(SimIntention::Banking(BankingIntention::PostOvernightFundingQuote {
                quote: ONQuote {
                    venue: OvernightVenue::FedFundsON,
                    agent: bank.id,
                    side: ONQuoteSide::Borrow,
                    notional: amount_needed,
                    limit_rate_bps: borrow_rate,
                    haircut: None,
                    preferred_collateral: None,
                    min_fill: 0.0,
                    ts: 0,
                },
            }));
        } else if reserve_surplus_or_shortfall > 1.0 {
            let amount_to_lend = reserve_surplus_or_shortfall * 0.75;
            if amount_to_lend > 100.0 {
                let lend_rate = target_rate_bps - Decimal::from_f64(acceptable_rate_range / 2.0).unwrap_or_default();
                intentions.push(SimIntention::Banking(BankingIntention::PostOvernightFundingQuote {
                    quote: ONQuote {
                        venue: OvernightVenue::FedFundsON,
                        agent: bank.id,
                        side: ONQuoteSide::Lend,
                        notional: amount_to_lend,
                        limit_rate_bps: lend_rate,
                        haircut: None,
                        preferred_collateral: None,
                        min_fill: 0.0,
                        ts: 0,
                    },
                }));
            }
        }
    }
    fn consider_treasury_market_making(
        &self, bank: &Bank, state: &SimState, intentions: &mut Vec<SimIntention>, rng: &mut dyn RngCore,
    ) {
        let fs = &state.financial_system;
        let liquidity = fs.get_liquid_assets(&bank.id);
        if liquidity < 10_000.0 {
            return;
        }

        let quantity_per_issue = 5.0;
        let current_date = state.current_date;

        if let Some(treasury_ids) = fs.exchange.index.by_bond_type.get(&BondType::Government) {
            for inst_id in treasury_ids {
                if let Some(instrument) = fs.instruments.get(inst_id) {
                    if !instrument.should_create_order_book() {
                        continue;
                    }
                    if let InstrumentType::Debt(DebtInstrument::Bond(details)) = &instrument.instrument_type {
                        if details.maturity_date <= current_date {
                            continue;
                        }

                        let tenor_years = details.remaining_tenor_years(current_date);
                        if self.should_make_market_for_ytm(tenor_years, bank, fs) {
                            let (bid_yield_bps, ask_yield_bps) =
                                quote_treasury_yields(tenor_years, fs.central_bank.policy_rate_bps, rng);

                            intentions.push(SimIntention::Banking(BankingIntention::MarketMakeTreasuries {
                                agent_id: bank.id,
                                maturity_date: details.maturity_date,
                                quantity: quantity_per_issue,
                                bid_yield_bps,
                                ask_yield_bps,
                            }));
                        }
                    }
                }
            }
        }
    }

    fn handle_debt_auctions(
        &self, bank: &Bank, state: &SimState, liquid_assets: f64, intentions: &mut Vec<SimIntention>,
        rng: &mut dyn RngCore,
    ) {
        let fs = &state.financial_system;
        let current_date = state.current_date;
        let auction_budget = liquid_assets * 0.05;

        for auction in fs.exchange.open_auctions.values() {
            if let Some(instrument) = fs.instruments.get(&auction.instrument_id) {
                if let Some(details) = instrument.instrument_type.as_bond() {
                    let tenor_years = details.remaining_tenor_years(current_date);
                    let (bid_yield_bps, _ask_yield_bps) =
                        quote_treasury_yields(tenor_years, fs.central_bank.policy_rate_bps, rng);

                    let bid_price = auction_bid_price(
                        details,
                        bid_yield_bps,
                        &auction.instrument_id,
                        &fs.pricing_feeds,
                        current_date,
                    );

                    if bid_price.to_f64() < 1.0 {
                        continue;
                    }
                    let quantity_to_bid = (auction_budget / bid_price.to_f64()).floor() as u32;

                    if quantity_to_bid > 0 {
                        intentions.push(SimIntention::Fiscal(FiscalIntention::BidInDebtAuction {
                            agent_id: bank.id,
                            auction_id: auction.auction_id,
                            quantity: quantity_to_bid,
                            bid_price,
                        }));
                    }
                }
            }
        }
    }

    fn should_make_market_for_ytm(&self, ytm: f64, _bank: &Bank, _fs: &FinancialSystem) -> bool {
        if ytm > 25.0 { false } else { true }
    }
}
