use sim_prelude::*;
use std::any::Any;
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct BasicConsumerDecisionModel;

#[typetag::serde]
impl DecisionModel for BasicConsumerDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimAction> {
        let consumer = match agent.downcast_ref::<Consumer>() {
            Some(c) => c,
            None => return vec![],
        };

        let mut actions = Vec::new();
        let fs = &state.financial_system;

        let weekly_income = consumer.income / 52.0;
        let cash_holdings = fs.get_cash_assets(&consumer.id);
        let total_available = weekly_income + cash_holdings;

        let prop_to_consume = match consumer.personality {
            PersonalityArchetype::Balanced => 0.7,
            PersonalityArchetype::Spender => 0.8,
            PersonalityArchetype::Saver => 0.6,
        };
        let spend_amount = total_available * prop_to_consume;
        let save_amount = total_available - spend_amount;

        let good_to_buy = good_id!("petrol");
        if let Some(seller) = fs.exchange.goods_market(&good_to_buy).and_then(|m| m.best_ask()) {
            if spend_amount > 1.0 {
                actions.push(SimAction::Consumption(ConsumptionAction::Purchase {
                    agent_id: consumer.id,
                    seller: seller.agent_id,
                    amount: spend_amount,
                    good_id: good_to_buy,
                }));
            }
        }

        if save_amount > 1.0 {
            actions.push(SimAction::Banking(BankingAction::Deposit {
                agent_id: consumer.id,
                bank: consumer.bank_id,
                amount: save_amount
            }));
        }
        actions
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParametricMPC {
    pub mpc_min: f64,
    pub mpc_max: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

#[typetag::serde]
impl DecisionModel for ParametricMPC {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimAction> {
        let consumer = match agent.downcast_ref::<Consumer>() {
            Some(c) => c,
            None => return vec![],
        };

        let fs = &state.financial_system;
        let weekly_income = consumer.income / 52.0;
        let cash = fs.get_liquid_assets(&consumer.id);
        let total = weekly_income + cash;
        let wealth_ratio = fs.get_total_assets(&consumer.id) / consumer.income.max(1.0);

        let mpc = self.mpc_min
            + (self.mpc_max - self.mpc_min)
                / (1.0 + (self.a + self.b * consumer.income.ln() + self.c * wealth_ratio).exp());

        let spend_amount = mpc * total;
        let save_amount = total - spend_amount;

        let mut actions = Vec::new();
        let good_to_buy = good_id!("petrol");

        if let Some(seller) = fs.exchange.goods_market(&good_to_buy).and_then(|m| m.best_ask()) {
            if spend_amount > 0.0 {
                actions.push(SimAction::Consumption(ConsumptionAction::Purchase {
                    agent_id: consumer.id,
                    seller: seller.agent_id,
                    amount: spend_amount,
                    good_id: good_to_buy,
                }));
            }
        }

        if save_amount > 0.0 {
            actions.push(SimAction::Banking(BankingAction::Deposit {
                agent_id: consumer.id,
                bank: consumer.bank_id,
                amount: save_amount
            }));
        }
        actions
    }
}