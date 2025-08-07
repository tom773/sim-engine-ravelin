use rand::RngCore;
use serde::{Deserialize, Serialize};
use sim_prelude::*;
use std::any::Any;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct BasicBankDecisionModel;

#[typetag::serde]
impl DecisionModel for BasicBankDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimAction> {
        let bank = match agent.downcast_ref::<Bank>() {
            Some(b) => b,
            None => return vec![],
        };
        let mut actions = Vec::new();
        let fs = &state.financial_system;

        self.manage_reserves(bank, fs, &mut actions);
        self.market_make_treasuries(bank, fs, &mut actions);

        actions
    }
}

impl BasicBankDecisionModel {
    fn manage_reserves(&self, bank: &Bank, fs: &FinancialSystem, actions: &mut Vec<SimAction>) {
        let total_deposits = fs.get_total_liabilities(&bank.id);
        let required_reserves = total_deposits * fs.central_bank.reserve_requirement;
        let desired_buffer = total_deposits * 0.02;
        let target_reserve_level = required_reserves + desired_buffer;

        let current_reserves = fs.get_bank_reserves(&bank.id).unwrap_or(0.0);
        let reserve_surplus_or_shortfall = current_reserves - target_reserve_level;

        let overnight_market_id = FinancialMarketId::SecuredOvernightFinancing;

        let floor_rate_bps = fs.central_bank.policy_rate * 10000.0;
        let ceiling_rate_bps = floor_rate_bps + 25.0;
        let target_rate_bps = (floor_rate_bps + ceiling_rate_bps) / 2.0;

        let daily_rate = overnight_market_id.annual_bps_to_daily_rate(target_rate_bps);
        let price = 1.0 / (1.0 + daily_rate);

        if reserve_surplus_or_shortfall < -1.0 {
            let amount_needed = -reserve_surplus_or_shortfall;

            actions.push(SimAction::Trading(TradingAction::PostBid {
                agent_id: bank.id,
                market_id: MarketId::Financial(overnight_market_id.clone()),
                quantity: amount_needed,
                price,
            }));
        } else if reserve_surplus_or_shortfall > 1.0 {
            let amount_to_lend = reserve_surplus_or_shortfall * 0.75;
            if amount_to_lend > 100.0 {
                actions.push(SimAction::Trading(TradingAction::PostAsk {
                    agent_id: bank.id,
                    market_id: MarketId::Financial(overnight_market_id.clone()),
                    quantity: amount_to_lend,
                    price,
                }));
            }
        }
    }

    fn market_make_treasuries(&self, bank: &Bank, fs: &FinancialSystem, actions: &mut Vec<SimAction>) {
        let bs = fs.get_bs_by_id(&bank.id).expect("Bank must have BS");

        let mut holdings_by_tenor: HashMap<Tenor, u64> = HashMap::new();
        for inst in bs.assets.values() {
            if let Some(bond_details) = inst.details.as_any().downcast_ref::<BondDetails>() {
                if bond_details.bond_type == BondType::Government {
                    *holdings_by_tenor.entry(bond_details.tenor).or_insert(0) += bond_details.quantity;
                }
            }
        }

        let quantity_to_quote = 5.0;
        const FACE_VALUE: f64 = 1000.0;
        let frequency = 2;


        for (market_id, _) in &fs.exchange.financial_markets {
            if let FinancialMarketId::Treasury { tenor } = market_id {
                let bid_ask_spread_bps = 10.0; 
                let target_yield_bps = fs.central_bank.policy_rate * 10000.0; // 0.043 -> 430

                let bid_yield_bps = target_yield_bps + (bid_ask_spread_bps / 2.0); // 430 + 5 = 435
                let ask_yield_bps = target_yield_bps - (bid_ask_spread_bps / 2.0); // 430 - 5 = 425

                let bid_yield = bid_yield_bps / 10000.0; // 435 -> 0.0435
                let ask_yield = ask_yield_bps / 10000.0; // 425 -> 0.0425

                let coupon_rate = 0.04;
                let years_to_maturity = tenor.to_days() as f64 / 365.25;

                let bid_price =
                    self.calculate_bond_price(FACE_VALUE, coupon_rate, bid_yield, years_to_maturity, frequency);

                let ask_price =
                    self.calculate_bond_price(FACE_VALUE, coupon_rate, ask_yield, years_to_maturity, frequency);

                actions.push(SimAction::Trading(TradingAction::PostBid {
                    agent_id: bank.id,
                    market_id: MarketId::Financial(market_id.clone()),
                    quantity: quantity_to_quote,
                    price: bid_price,
                }));

                let holdings = holdings_by_tenor.get(tenor).cloned().unwrap_or(0) as f64;
                if holdings >= quantity_to_quote {
                    actions.push(SimAction::Trading(TradingAction::PostAsk {
                        agent_id: bank.id,
                        market_id: MarketId::Financial(market_id.clone()),
                        quantity: quantity_to_quote,
                        price: ask_price,
                    }));
                }
            }
        }
    }

    fn calculate_bond_price(
        &self, face_value: f64, coupon_rate: f64, ytm: f64, years_to_maturity: f64, frequency: usize,
    ) -> f64 {
        let k = frequency as f64;
        let n = (years_to_maturity * k).round() as usize;
        let c = coupon_rate * face_value / k;
        let y = ytm / k;

        if n == 0 {
            return face_value;
        }

        let mut price = 0.0;

        if (y).abs() > 1e-9 {
            price += c * (1.0 - (1.0 + y).powf(-(n as f64))) / y;
        } else {
            price += c * n as f64;
        }

        price += face_value / (1.0 + y).powf(n as f64);

        price
    }
}
