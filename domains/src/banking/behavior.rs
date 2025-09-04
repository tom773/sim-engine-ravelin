use rand::RngCore;
use rand::prelude::*;
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::any::Any;

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct BasicBankDecisionModel;


#[typetag::serde]
impl DecisionModel for BasicBankDecisionModel {
    fn name (&self) -> &str { "Bank" }
    fn decide(&self, agent: &dyn Any, state: &SimState, rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let bank = match agent.downcast_ref::<Bank>() {
            Some(b) => b,
            None => return vec![],
        };
        
        let mut intentions = Vec::new();
        let fs = &state.financial_system;

        self.assess_liquidity_needs(bank, fs, &mut intentions);
        self.handle_debt_auctions(state, bank, fs.get_liquid_assets(&bank.id), &mut intentions, rng);
        self.consider_treasury_market_making(bank, state, &mut intentions, rng);
        self.evaluate_lending_opportunities(bank, fs, &mut intentions);
        intentions
    }
}

impl BasicBankDecisionModel {
    fn assess_liquidity_needs(&self, bank: &Bank, fs: &FinancialSystem, intentions: &mut Vec<SimIntention>) {
        let bank_bs = match fs.balance_sheets.get(&bank.id) {
            Some(bs) => bs,
            None => return,
        };

        let total_deposits = bank_bs.liabilities.iter().filter_map(|(id, pos)| {
            fs.instruments.get(id).and_then(|inst| {
                if let InstrumentType::Cash(d) = &inst.instrument_type {
                    if matches!(d.cash_type, CashType::DemandDeposit | CashType::SavingsDeposit) {
                        return Some(pos.quantity);
                    }
                }
                None
            })
        }).sum::<f64>();

        
        let required_reserves = total_deposits * fs.central_bank.reserve_requirement;
        let desired_buffer = total_deposits * 0.02; // 2% buffer
        let target_reserve_level = required_reserves + desired_buffer;

        let current_reserves = fs.get_bank_reserves(&bank.id).unwrap_or(0.0);
        let reserve_surplus_or_shortfall = current_reserves - target_reserve_level;

        let policy_rate_bps = fs.central_bank.policy_rate_bps;
        let acceptable_rate_range = 25.0;
        let target_rate_bps = policy_rate_bps; 

        if reserve_surplus_or_shortfall < -1.0 {
            let amount_needed = -reserve_surplus_or_shortfall;
            let borrow_rate = target_rate_bps + Decimal::from_f64(acceptable_rate_range / 2.0).unwrap_or_default();
            intentions.push(SimIntention::BorrowReserves {
                agent_id: bank.id,
                amount: amount_needed,
                target_rate_bps: borrow_rate,
            });
        } else if reserve_surplus_or_shortfall > 1.0 {
            let amount_to_lend = reserve_surplus_or_shortfall * 0.75;
            if amount_to_lend > 100.0 {
                let lend_rate = target_rate_bps - Decimal::from_f64(acceptable_rate_range / 2.0).unwrap_or_default();
                intentions.push(SimIntention::LendExcessReserves {
                    agent_id: bank.id,
                    amount: amount_to_lend,
                    target_rate_bps: lend_rate,
                });
            }
        }
    }
    fn handle_debt_auctions(
        &self,
        state: &SimState,
        bank: &Bank,
        liquid_assets: f64,
        intentions: &mut Vec<SimIntention>,
        rng: &mut dyn RngCore,
    ) {
        let fs = &state.financial_system;
        let auction_budget = liquid_assets * 0.15;
        if auction_budget < 1000.0 {
            return;
        }
        for auction in fs.exchange.open_auctions.values() {
            if auction.status != AuctionStatus::Open {
                continue;
            }
            if let Some(instrument) = fs.instruments.get(&auction.instrument_id) {
                if let InstrumentType::Bond(details) = &instrument.instrument_type {
                    let ytm = pricing::years_to_maturity(state.current_date, details.maturity_date);
                    let (bid_yield_bps, _ask_yield_bps) = self.calculate_yield_quotes(ytm, fs, rng);

                    let bid_price = pricing::bond_price(
                        details.face_value,
                        bps_to_decimal(details.coupon_rate_bps),
                        bps_to_decimal(bid_yield_bps),
                        ytm,
                        details.frequency as usize,
                    );

                    if bid_price <= Money::ZERO {
                        continue;
                    }

                    let quantity_to_bid = (auction_budget / bid_price.to_f64()).floor() as u32;
                    if quantity_to_bid > 0 {
                        intentions.push(SimIntention::BidInDebtAuction {
                            agent_id: bank.id,
                            auction_id: auction.auction_id,
                            quantity: quantity_to_bid,
                            bid_price,
                        });
                    }
                }
            }
        }
    }
    fn consider_treasury_market_making(&self, bank: &Bank, state: &SimState, intentions: &mut Vec<SimIntention>, rng: &mut dyn RngCore) {
        let fs = &state.financial_system;
        let liquidity = fs.get_liquid_assets(&bank.id);
        if liquidity < 10000.0 {
            return;
        }

        let quantity_per_issue = 5.0;
        let current_date = state.current_date;

        if let Some(treasury_ids) = fs.exchange.index.by_bond_type.get(&BondType::Government) {
            for inst_id in treasury_ids {
                if let Some(instrument) = fs.instruments.get(inst_id) {
                    if let InstrumentType::Bond(details) = &instrument.instrument_type {
                        
                        if details.maturity_date <= current_date {
                            continue;
                        }

                        let ytm = years_to_maturity(current_date, details.maturity_date);

                        if self.should_make_market_for_ytm(ytm, bank, fs) {
                            let (bid_yield, ask_yield) = self.calculate_yield_quotes(ytm, fs, rng);
                            
                            intentions.push(SimIntention::MarketMakeTreasuries {
                                agent_id: bank.id,
                                maturity_date: details.maturity_date,
                                quantity: quantity_per_issue,
                                bid_yield_bps: bid_yield,
                                ask_yield_bps: ask_yield,
                            });
                        }
                    }
                }
            }
        }
    }

    fn should_make_market_for_ytm(&self, ytm: f64, _bank: &Bank, _fs: &FinancialSystem) -> bool {
        if ytm > 25.0 {
            false
        } else {
            true
        }
    }

    fn calculate_yield_quotes(&self, ytm: f64, fs: &FinancialSystem, rng: &mut dyn RngCore) -> (BasisPoints, BasisPoints) {
        let policy_rate_bps = fs.central_bank.policy_rate_bps;
        
        let term_premium = if ytm <= 0.083 { // ~1 month
            2.0
        } else if ytm <= 0.25 { // ~3 months
            7.0
        } else if ytm <= 1.0 {
            12.0
        } else if ytm <= 5.0 {
            35.0
        } else if ytm <= 10.0 {
            50.0
        } else {
            65.0
        };

        let bid_ask_spread_bps = rng.random_range(15.0..30.0);
        
        let base_yield = policy_rate_bps + Decimal::from_f64(term_premium * rng.random_range(0.9..1.1)).unwrap_or_default();
        let bid_yield_bps = base_yield + Decimal::from_f64(bid_ask_spread_bps / 2.0).unwrap_or_default();
        let ask_yield_bps = base_yield - Decimal::from_f64(bid_ask_spread_bps / 2.0).unwrap_or_default();

        (bid_yield_bps, ask_yield_bps)
    }

    fn evaluate_lending_opportunities(&self, bank: &Bank, fs: &FinancialSystem, _intentions: &mut Vec<SimIntention>) {
        let available_capital = fs.get_liquid_assets(&bank.id) - 5000.0;
        
        if available_capital > 1000.0 {
        }
    }
}