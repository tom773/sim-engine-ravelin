use rand::RngCore;
use rand::prelude::*;
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
        self.consider_treasury_market_making(bank, fs, &mut intentions, rng);
        self.evaluate_lending_opportunities(bank, fs, &mut intentions);

        intentions
    }
}

impl BasicBankDecisionModel {
    fn assess_liquidity_needs(&self, bank: &Bank, fs: &FinancialSystem, intentions: &mut Vec<SimIntention>) {
        let total_deposits = fs.get_total_liabilities(&bank.id);
        let required_reserves = total_deposits * fs.central_bank.reserve_requirement;
        let desired_buffer = total_deposits * 0.02;
        let target_reserve_level = required_reserves + desired_buffer;

        let current_reserves = fs.get_bank_reserves(&bank.id).unwrap_or(0.0);
        let reserve_surplus_or_shortfall = current_reserves - target_reserve_level;

        let policy_rate_bps = fs.central_bank.policy_rate_bps;
        let acceptable_rate_range = 25.0;
        let target_rate_bps = policy_rate_bps + (acceptable_rate_range / 2.0);

        if reserve_surplus_or_shortfall < -1.0 {
            let amount_needed = -reserve_surplus_or_shortfall;
            intentions.push(SimIntention::BorrowReserves {
                agent_id: bank.id,
                amount: amount_needed,
                target_rate_bps,
            });
        } else if reserve_surplus_or_shortfall > 1.0 {
            let amount_to_lend = reserve_surplus_or_shortfall * 0.75;
            if amount_to_lend > 100.0 {
                intentions.push(SimIntention::LendExcessReserves {
                    agent_id: bank.id,
                    amount: amount_to_lend,
                    target_rate_bps,
                });
            }
        }
    }

    fn consider_treasury_market_making(&self, bank: &Bank, fs: &FinancialSystem, intentions: &mut Vec<SimIntention>, rng: &mut dyn RngCore) {
        let liquidity = fs.get_liquid_assets(&bank.id);
        if liquidity < 10000.0 {
            return;
        }

        let quantity_per_tenor = 5.0;

        for (market_id, _market) in &fs.exchange.financial_markets {
            if let FinancialMarketId::Treasury { tenor } = market_id {
                if self.should_make_market_for_tenor(tenor, bank, fs) {
                    let (bid_yield, ask_yield) = self.calculate_yield_quotes(*tenor, fs, rng);
                    
                    intentions.push(SimIntention::MarketMakeTreasuries {
                        agent_id: bank.id,
                        tenor: *tenor,
                        quantity: quantity_per_tenor,
                        bid_yield_bps: bid_yield,
                        ask_yield_bps: ask_yield,
                    });
                }
            }
        }
    }

    fn should_make_market_for_tenor(&self, tenor: &Tenor, _bank: &Bank, _fs: &FinancialSystem) -> bool {
        match tenor {
            Tenor::T30Y => false,
            _ => true,
        }
    }

    fn calculate_yield_quotes(&self, tenor: Tenor, fs: &FinancialSystem, rng: &mut dyn RngCore) -> (BasisPoints, BasisPoints) {
        let policy_rate_bps = fs.central_bank.policy_rate_bps;
        
        let term_premium = match tenor {
            Tenor::T1M => 2.0,
            Tenor::T2M => 3.0,
            Tenor::T3M => 7.0,
            Tenor::T6M => 10.0,
            Tenor::T1Y => 12.0,
            Tenor::T2Y => 15.0,
            Tenor::T5Y => 35.0,
            Tenor::T10Y => 50.0,
            Tenor::T30Y => 65.0,
        };

        let bid_ask_spread_bps = rng.random_range(15.0..30.0);
        
        let base_yield = policy_rate_bps + (term_premium * rng.random_range(0.9..1.1));
        let bid_yield_bps = base_yield + (bid_ask_spread_bps / 2.0);
        let ask_yield_bps = base_yield - (bid_ask_spread_bps / 2.0);

        (bid_yield_bps, ask_yield_bps)
    }

    fn evaluate_lending_opportunities(&self, bank: &Bank, fs: &FinancialSystem, _intentions: &mut Vec<SimIntention>) {
        let available_capital = fs.get_liquid_assets(&bank.id) - 5000.0;
        
        if available_capital > 1000.0 {
        }
    }
}