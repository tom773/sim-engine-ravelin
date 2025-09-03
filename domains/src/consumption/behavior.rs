use sim_core::*;
use std::any::Any;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use rust_decimal::prelude::*;

// Helper for labour market identification.
fn find_general_labour_market(state: &SimState) -> Option<LabourMarketId> {
    // In a simple simulation, we might just take the first available labour market.
    state.financial_system.exchange.labour_markets.keys().next().cloned()
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimpleConsumerDecisionModel {
    pub mpc: f64,
    // Configured with GoodIds instead of relying on static slug lookup which is removed.
    pub consumption_basket: HashMap<GoodId, f64>, // GoodId -> Budget Share
}

impl Default for SimpleConsumerDecisionModel {
    fn default() -> Self {
        // Default is empty; must be configured by the scenario setup as we cannot access GoodsRegistry here.
        Self { mpc: 0.7, consumption_basket: HashMap::new() }
    }
}

#[typetag::serde]
impl DecisionModel for SimpleConsumerDecisionModel {
    fn name(&self) -> &str { "Simple Consumer" }
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let consumer = match agent.downcast_ref::<Consumer>() {
            Some(c) => c,
            None => return vec![],
        };

        let mut intentions = Vec::new();

        self.handle_employment(consumer, state, &mut intentions);

        let fs = &state.financial_system;
        // Simplified income calculation (assuming income is annual).
        let weekly_income = consumer.income / 52.0;
        // Liquid assets are primarily Demand Deposits.
        let liquid_assets = fs.get_liquid_assets(&consumer.id);
        let total_resources = weekly_income + liquid_assets;
        
        if total_resources < 1.0 {
            return intentions;
        }

        let budget = total_resources * self.mpc;
        let save_amount = total_resources - budget;

        self.make_purchases(consumer, budget, &mut intentions);

        // Deposit savings if the consumer holds physical cash they wish to deposit.
        // This action is likely to fail execution due to sim_core limitations (Partial Transfers).
        if save_amount > 1.0 {
            // We only attempt deposit if the consumer actually has physical cash (Currency) to deposit.
            // In many models, income is received directly as deposits, so explicit deposit action might be unnecessary unless modeling physical cash economy.
            // Assuming income is received as deposits, we skip explicit deposit intention.
        }

        intentions
    }
}

impl SimpleConsumerDecisionModel {
    fn handle_employment(&self, consumer: &Consumer, state: &SimState, intentions: &mut Vec<SimIntention>) {
        if consumer.employed_by.is_none() {
            
            // Find the labour market to apply to.
            let market_id = match find_general_labour_market(state) {
                Some(id) => id,
                None => return, // Cannot apply if no labour market exists.
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

            intentions.push(SimIntention::ApplyForJob {
                agent_id: consumer.id,
                market_id,
                application,
            });
        }
    }

    fn make_purchases(&self, consumer: &Consumer, budget: f64, intentions: &mut Vec<SimIntention>) {
        // Use the configured consumption basket.
        for (good_id, budget_share) in &self.consumption_basket {
            let allocation = budget * budget_share;
            
            if allocation > 0.01 {
                intentions.push(SimIntention::SpendOnGood {
                    agent_id: consumer.id,
                    good_id: *good_id,
                    max_notional: allocation,
                });
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
        // Default is empty; must be configured by the scenario setup.
        Self {
            sigma: 1.5,
            weights: HashMap::new(),
            mpc_base: 0.8,
        }
    }
}

#[typetag::serde]
impl DecisionModel for CESConsumerDecisionModel {
    fn name (&self) -> &str { "CES Consumer" }
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let consumer = match agent.downcast_ref::<Consumer>() {
            Some(c) => c,
            None => return vec![],
        };

        let mut intentions = Vec::new();

        self.handle_employment(consumer, state, &mut intentions);

        // Intertemporal consumption choice based on real interest rate.
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

        // Intra-period consumption choice (CES optimization).
        let market_data = self.collect_market_data(state);
        if !market_data.is_empty() {
            self.optimize_ces_consumption(consumer, budget, &market_data, &mut intentions);
        }

        self.handle_savings(consumer, save_amount, &mut intentions);

        intentions
    }
}

impl CESConsumerDecisionModel {
    // Uses the shared helper for labour market ID.
    fn handle_employment(&self, consumer: &Consumer, state: &SimState, intentions: &mut Vec<SimIntention>) {
        if consumer.employed_by.is_none() {
            
            // Find the labour market.
            let market_id = match find_general_labour_market(state) {
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

            intentions.push(SimIntention::ApplyForJob {
                agent_id: consumer.id,
                market_id,
                application,
            });
        }
    }

    fn collect_market_data(&self, state: &SimState) -> Vec<(GoodId, f64, f64)> {
        let mut market_data = Vec::new();

        for (good_id, weight) in &self.weights {
            // Access goods market view via the state analytics trait.
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

    fn handle_savings(&self, _consumer: &Consumer, save_amount: f64, _intentions: &mut Vec<SimIntention>) {
        // Similar to SimpleConsumerDecisionModel, explicit deposit might be unnecessary or fail.
        if save_amount > 1.0 {
            // If we wanted to model investment decisions (e.g., buying bonds instead of saving), it would happen here.
        }
    }
}