use sim_prelude::*;
use std::any::Any;
use rand::RngCore;
use serde::{Deserialize, Serialize};

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

        let total_deposits = fs.get_total_liabilities(&bank.id);
        let required_reserves = total_deposits * fs.central_bank.reserve_requirement;
        let desired_buffer = total_deposits * 0.02;
        let target_reserve_level = required_reserves + desired_buffer;

        let current_reserves = fs.get_bank_reserves(&bank.id).unwrap_or(0.0);
        let reserve_surplus_or_shortfall = current_reserves - target_reserve_level;

        let overnight_market_id = FinancialMarketId::SecuredOvernightFinancing;

        if reserve_surplus_or_shortfall < -1.0 {
            let amount_needed = -reserve_surplus_or_shortfall;
            let max_annual_rate_bps = (fs.central_bank.policy_rate * 10000.0) + 50.0;
            let daily_rate = overnight_market_id.annual_bps_to_daily_rate(max_annual_rate_bps);
            let price = 1.0 / (1.0 + daily_rate);

            actions.push(SimAction::Trading(TradingAction::PostBid {
                agent_id: bank.id,
                market_id: MarketId::Financial(overnight_market_id),
                quantity: amount_needed,
                price,
            }));
        } else if reserve_surplus_or_shortfall > 1.0 {
            let amount_to_lend = reserve_surplus_or_shortfall * 0.75;
            if amount_to_lend > 100.0 {
                let min_annual_rate_bps = (fs.central_bank.policy_rate * 10000.0) - 25.0;
                let daily_rate = overnight_market_id.annual_bps_to_daily_rate(min_annual_rate_bps);
                let price = 1.0 / (1.0 + daily_rate);

                actions.push(SimAction::Trading(TradingAction::PostAsk {
                    agent_id: bank.id,
                    market_id: MarketId::Financial(overnight_market_id),
                    quantity: amount_to_lend,
                    price,
                }));
            }
        }
        actions
    }
}