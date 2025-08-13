use sim_core::*;
use std::any::Any;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

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
        let liquid_assets = fs.get_liquid_assets(&consumer.id);
        let total_available = weekly_income + liquid_assets;

        let prop_to_consume = match consumer.personality {
            PersonalityArchetype::Balanced => 0.7,
            PersonalityArchetype::Spender => 0.8,
            PersonalityArchetype::Saver => 0.6,
        };
        let spend_amount = total_available * prop_to_consume;
        let save_amount = total_available - spend_amount;

        let good_to_buy = good_id!("petrol");

        if spend_amount > 1.0 {
            actions.push(SimAction::Consumption(ConsumptionAction::PurchaseAtBest {
                agent_id: consumer.id,
                good_id: good_to_buy,
                max_notional: spend_amount,
            }));
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
pub struct CESConsumerDecisionModel {
    pub sigma: f64,
    pub weights: HashMap<GoodId, f64>,
    pub mpc_base: f64,
}

impl Default for CESConsumerDecisionModel {
    fn default() -> Self {
        let mut weights = HashMap::new();
        if let Some(petrol_id) = goods::CATALOGUE.get_good_id_by_slug("petrol") {
            weights.insert(petrol_id, 1.0);
        }

        Self {
            sigma: 1.5, // Goods are substitutes
            weights,
            mpc_base: 0.8,
        }
    }
}

#[typetag::serde]
impl DecisionModel for CESConsumerDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimAction> {
        let consumer = match agent.downcast_ref::<Consumer>() {
            Some(c) => c,
            None => return vec![],
        };

        let mut actions = Vec::new();

        self.handle_employment(consumer, state, &mut actions);

        let nominal_rate = state.financial_system.central_bank.policy_rate;
        let expected_inflation = consumer.expectations.expected_inflation;
        let real_rate = nominal_rate - expected_inflation;

        let mpc_adjustment = (real_rate - 0.02).max(0.0) * 5.0; // Sensitivity factor
        let mpc = (self.mpc_base - mpc_adjustment).max(0.1).min(0.95);


        let fs = &state.financial_system;
        let weekly_income = consumer.income / 52.0;
        let liquid_assets = fs.get_liquid_assets(&consumer.id);
        let total_resources = weekly_income + liquid_assets; // Taxes handled by FiscalDomain

        let budget = total_resources * mpc;
        let save_amount = total_resources - budget;

        if budget < 1.0 {
            self.handle_savings(consumer, save_amount, &mut actions);
            return actions;
        }

        let mut market_data = Vec::new();

        for (good_id, weight) in &self.weights {
            if let Some(view) = state.market_view(&MarketId::Goods(*good_id)) {
                if let Some(price) = view.last_or_mid() {
                     if price > 1e-6 {
                        market_data.push((*good_id, price, *weight));
                     }
                }
            }
        }

        if market_data.is_empty() {
            self.handle_savings(consumer, save_amount, &mut actions);
            return actions;
        }

        let denominator: f64 = market_data.iter().map(|(_, price, weight)| {
            weight * price.powf(1.0 - self.sigma)
        }).sum();

        if denominator <= 1e-9 {
            self.handle_savings(consumer, save_amount, &mut actions);
            return actions;
        }

        for (good_id, price, weight) in market_data {
            let share = (weight * price.powf(1.0 - self.sigma)) / denominator;

            let notional = share * budget;

            if notional > 0.01 {
                actions.push(SimAction::Consumption(ConsumptionAction::PurchaseAtBest {
                    agent_id: consumer.id,
                    good_id,
                    max_notional: notional,
                }));
            }
        }

        self.handle_savings(consumer, save_amount, &mut actions);

        actions
    }
}

impl CESConsumerDecisionModel {
    fn handle_savings(&self, consumer: &Consumer, save_amount: f64, actions: &mut Vec<SimAction>) {
        if save_amount > 1.0 {
            actions.push(SimAction::Banking(BankingAction::Deposit {
                agent_id: consumer.id,
                bank: consumer.bank_id,
                amount: save_amount
            }));
        }
    }

    fn handle_employment(&self, consumer: &Consumer, _state: &SimState, actions: &mut Vec<SimAction>) {
        if consumer.employed_by.is_none() {
            let expected_hourly_wage = match consumer.personality {
                PersonalityArchetype::Balanced => 25.0,
                PersonalityArchetype::Spender => 30.0,
                PersonalityArchetype::Saver => 20.0,
            };

            let application = JobApplication {
                application_id: Uuid::new_v4(),
                consumer_id: consumer.id,
                reservation_wage: expected_hourly_wage * 0.9, // Willing to accept 10% less than ideal
                hours_desired: 40.0,
            };

            actions.push(SimAction::Labour(LabourAction::ApplyForJob {
                market_id: LabourMarketId::GeneralLabour,
                application,
            }));
        }
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
        let liquid_assets = fs.get_liquid_assets(&consumer.id);
        let total = weekly_income + liquid_assets;
        let wealth_ratio = fs.get_total_assets(&consumer.id) / consumer.income.max(1.0);

        let mpc = self.mpc_min
            + (self.mpc_max - self.mpc_min)
                / (1.0 + (self.a + self.b * consumer.income.ln() + self.c * wealth_ratio).exp());

        let spend_amount = mpc * total;
        let save_amount = total - spend_amount;

        let mut actions = Vec::new();
        let good_to_buy = good_id!("petrol");

        if spend_amount > 0.0 {
            actions.push(SimAction::Consumption(ConsumptionAction::PurchaseAtBest {
                agent_id: consumer.id,
                good_id: good_to_buy,
                max_notional: spend_amount,
            }));
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