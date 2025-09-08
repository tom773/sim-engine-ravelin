use rand::prelude::*;
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::any::Any;
use std::collections::HashMap;
use uuid::Uuid;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimpleConsumerDecisionModel {
    pub mpc: f64,
    pub consumption_basket: HashMap<GoodId, f64>,
}

impl Default for SimpleConsumerDecisionModel {
    fn default() -> Self {
        Self { mpc: 0.7, consumption_basket: HashMap::new() }
    }
}

#[typetag::serde]
impl DecisionModel for SimpleConsumerDecisionModel {
    fn name(&self) -> &str {
        "Simple Consumer"
    }
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let consumer = match agent.downcast_ref::<Consumer>() {
            Some(c) => c,
            None => return vec![],
        };

        let mut intentions = Vec::new();

        self.handle_employment(consumer, state, &mut intentions);

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
        self.apply_for_loan(consumer, &state.financial_system, &mut intentions);
        if save_amount > 1.0 {}

        intentions
    }
}

impl SimpleConsumerDecisionModel {
    fn handle_employment(&self, consumer: &Consumer, state: &SimState, intentions: &mut Vec<SimIntention>) {
        if consumer.employed_by.is_none() {
            let market_id = match state.financial_system.find_general_labour_market() {
                Some(id) => id,
                None => return,
            };

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

            intentions.push(SimIntention::Production(ProductionIntention::ApplyForJob {
                agent_id: consumer.id,
                market_id,
                application,
            }));
        }
    }
    fn apply_for_loan(
        &self,
        consumer: &Consumer,
        fs: &FinancialSystem,
        intentions: &mut Vec<SimIntention>,
    ) {
        if let Some(_apps) = fs.credit_registry.applications.get(&consumer.id.0) {
            return;
        }
        let liquid_assets = fs.get_liquid_assets(&consumer.id);
        if liquid_assets < 10_000.0 && consumer.income > 1000.0 {
            let existing_debt = fs.get_total_liabilities(&consumer.id);
            if existing_debt > consumer.income * 0.5 {
                return; 
            }

            let desired_amount = 5_000.0;

            intentions.push(SimIntention::Banking(BankingIntention::RequestLoan {
                agent_id: consumer.id,
                bank_id: consumer.bank_id,
                amount: desired_amount,
                purpose: LoanPurpose::PersonalConsumption,
                collateral: None,
            }));
        }
    }
    fn make_purchases(&self, consumer: &Consumer, budget: f64, intentions: &mut Vec<SimIntention>) {
        for (good_id, budget_share) in &self.consumption_basket {
            let allocation = budget * budget_share;

            if allocation > 0.01 {
                intentions.push(SimIntention::Consumption(ConsumptionIntention::SpendOnGood {
                    agent_id: consumer.id,
                    good_id: *good_id,
                    max_notional: allocation,
                }));
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
        Self { sigma: 1.5, weights: HashMap::new(), mpc_base: 0.8 }
    }
}

#[typetag::serde]
impl DecisionModel for CESConsumerDecisionModel {
    fn name(&self) -> &str {
        "CES Consumer"
    }
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let consumer = match agent.downcast_ref::<Consumer>() {
            Some(c) => c,
            None => return vec![],
        };

        let mut intentions = Vec::new();

        self.handle_employment(consumer, state, &mut intentions);

        let nominal_rate_bps = state.financial_system.central_bank.policy_rate_bps;
        let nominal_rate = bps_to_decimal(nominal_rate_bps);
        let expected_inflation = consumer.expectations.expected_inflation;
        let real_rate = nominal_rate - Decimal::from_f64(expected_inflation).unwrap_or_default();

        let mpc_adjustment = (real_rate.to_f64().unwrap_or_default() - 0.02).max(0.0) * 5.0;
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
    fn handle_employment(&self, consumer: &Consumer, state: &SimState, intentions: &mut Vec<SimIntention>) {
        if consumer.employed_by.is_none() {
            let market_id = match state.financial_system.find_general_labour_market() {
                Some(id) => id,
                None => return,
            };

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

            intentions.push(SimIntention::Production(ProductionIntention::ApplyForJob {
                agent_id: consumer.id,
                market_id,
                application,
            }));
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
        &self, consumer: &Consumer, budget: f64, market_data: &[(GoodId, f64, f64)], intentions: &mut Vec<SimIntention>,
    ) {
        let denominator: f64 = market_data.iter().map(|(_, price, weight)| weight * price.powf(1.0 - self.sigma)).sum();

        if denominator <= 1e-9 {
            return;
        }

        for (good_id, price, weight) in market_data {
            let share = (weight * price.powf(1.0 - self.sigma)) / denominator;
            let notional = share * budget;

            if notional > 0.01 {
                intentions.push(SimIntention::Consumption(ConsumptionIntention::SpendOnGood {
                    agent_id: consumer.id,
                    good_id: *good_id,
                    max_notional: notional,
                }));
            }
        }
    }

    fn handle_savings(&self, _consumer: &Consumer, save_amount: f64, _intentions: &mut Vec<SimIntention>) {
        if save_amount > 1.0 {}
    }
}
