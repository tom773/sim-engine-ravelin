use sim_core::*;
use std::any::Any;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimpleConsumerDecisionModel {
    pub mpc: f64,
}

impl Default for SimpleConsumerDecisionModel {
    fn default() -> Self {
        Self { mpc: 0.7 }
    }
}

#[typetag::serde]
impl DecisionModel for SimpleConsumerDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let consumer = match agent.downcast_ref::<Consumer>() {
            Some(c) => c,
            None => return vec![],
        };

        let mut intentions = Vec::new();

        self.handle_employment(consumer, &mut intentions);

        let fs = &state.financial_system;
        let weekly_income = consumer.income / 52.0;
        let liquid_assets = fs.get_liquid_assets(&consumer.id);
        let total_resources = weekly_income + liquid_assets;
        
        if total_resources < 1.0 {
            return intentions;
        }

        let budget = total_resources * self.mpc;
        let save_amount = total_resources - budget;

        self.make_purchases(consumer, budget, &mut intentions);

        if save_amount > 1.0 {
            intentions.push(SimIntention::DepositFunds {
                agent_id: consumer.id,
                bank: consumer.bank_id,
                amount: save_amount,
            });
        }

        intentions
    }
}

impl SimpleConsumerDecisionModel {
    fn handle_employment(&self, consumer: &Consumer, intentions: &mut Vec<SimIntention>) {
        if consumer.employed_by.is_none() {
            let expected_hourly_wage = match consumer.personality {
                PersonalityArchetype::Balanced => 25.0,
                PersonalityArchetype::Spender => 30.0,
                PersonalityArchetype::Saver => 20.0,
            };

            let application = JobApplication {
                application_id: Uuid::new_v4(),
                consumer_id: consumer.id,
                reservation_wage: expected_hourly_wage * 0.9,
                hours_desired: 40.0,
            };

            intentions.push(SimIntention::ApplyForJob {
                agent_id: consumer.id,
                market_id: LabourMarketId::GeneralLabour,
                application,
            });
        }
    }

    fn make_purchases(&self, consumer: &Consumer, budget: f64, intentions: &mut Vec<SimIntention>) {
        let consumption_basket = [
            ("bread", 0.4, 3.0),
            ("petrol", 0.6, 4.0),
        ];

        for (good_slug, budget_share, _fallback_price) in consumption_basket {
            if let Some(good_id) = goods::CATALOGUE.get_good_id_by_slug(good_slug) {
                let allocation = budget * budget_share;
                
                if allocation > 0.01 {
                    intentions.push(SimIntention::SpendOnGood {
                        agent_id: consumer.id,
                        good_id,
                        max_notional: allocation,
                    });
                }
            }
        }
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
            weights.insert(petrol_id, 0.2);
        }
        if let Some(bread_id) = goods::CATALOGUE.get_good_id_by_slug("bread") {
            weights.insert(bread_id, 0.5);
        }

        Self {
            sigma: 1.5,
            weights,
            mpc_base: 0.8,
        }
    }
}

#[typetag::serde]
impl DecisionModel for CESConsumerDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let consumer = match agent.downcast_ref::<Consumer>() {
            Some(c) => c,
            None => return vec![],
        };

        let mut intentions = Vec::new();

        self.handle_employment(consumer, &mut intentions);

        let nominal_rate_bps = state.financial_system.central_bank.policy_rate_bps;
        let nominal_rate = bps_to_decimal(nominal_rate_bps);
        let expected_inflation = consumer.expectations.expected_inflation;
        let real_rate = nominal_rate - expected_inflation;

        let mpc_adjustment = (real_rate - 0.02).max(0.0) * 5.0;
        let mpc = (self.mpc_base - mpc_adjustment).max(0.1).min(0.95);

        let fs = &state.financial_system;
        let weekly_income = consumer.income / 52.0;
        let liquid_assets = fs.get_liquid_assets(&consumer.id);
        let total_resources = weekly_income + liquid_assets;
        let budget = total_resources * mpc;
        let save_amount = total_resources - budget;

        if budget < 1.0 {
            self.handle_savings(consumer, save_amount, &mut intentions);
            return intentions;
        }

        let market_data = self.collect_market_data(state);
        if !market_data.is_empty() {
            self.optimize_ces_consumption(consumer, budget, &market_data, &mut intentions);
        }

        self.handle_savings(consumer, save_amount, &mut intentions);

        intentions
    }
}

impl CESConsumerDecisionModel {
    fn handle_employment(&self, consumer: &Consumer, intentions: &mut Vec<SimIntention>) {
        if consumer.employed_by.is_none() {
            let expected_hourly_wage = match consumer.personality {
                PersonalityArchetype::Balanced => 25.0,
                PersonalityArchetype::Spender => 30.0,
                PersonalityArchetype::Saver => 20.0,
            };

            let application = JobApplication {
                application_id: Uuid::new_v4(),
                consumer_id: consumer.id,
                reservation_wage: expected_hourly_wage * 0.9,
                hours_desired: 40.0,
            };

            intentions.push(SimIntention::ApplyForJob {
                agent_id: consumer.id,
                market_id: LabourMarketId::GeneralLabour,
                application,
            });
        }
    }

    fn collect_market_data(&self, state: &SimState) -> Vec<(GoodId, f64, f64)> {
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

        market_data
    }

    fn optimize_ces_consumption(
        &self,
        consumer: &Consumer,
        budget: f64,
        market_data: &[(GoodId, f64, f64)],
        intentions: &mut Vec<SimIntention>,
    ) {
        let denominator: f64 = market_data.iter()
            .map(|(_, price, weight)| weight * price.powf(1.0 - self.sigma))
            .sum();

        if denominator <= 1e-9 {
            return;
        }

        for (good_id, price, weight) in market_data {
            let share = (weight * price.powf(1.0 - self.sigma)) / denominator;
            let notional = share * budget;

            if notional > 0.01 {
                intentions.push(SimIntention::SpendOnGood {
                    agent_id: consumer.id,
                    good_id: *good_id,
                    max_notional: notional,
                });
            }
        }
    }

    fn handle_savings(&self, consumer: &Consumer, save_amount: f64, intentions: &mut Vec<SimIntention>) {
        if save_amount > 1.0 {
            intentions.push(SimIntention::DepositFunds {
                agent_id: consumer.id,
                bank: consumer.bank_id,
                amount: save_amount,
            });
        }
    }
}