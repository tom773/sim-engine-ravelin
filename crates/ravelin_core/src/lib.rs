#[macro_export]
macro_rules! prep_serde_as {
    ($outer:ty, $inner:ty) => {
        impl std::fmt::Display for $outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        
        impl std::str::FromStr for $outer {
            type Err = <$inner as std::str::FromStr>::Err;
            
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.parse::<$inner>()?))
            }
        }
    };
}
pub mod action;
pub mod agent;
pub mod behaviour;
pub mod execution;
pub mod model;
pub mod state;
pub mod validation;

pub use action::*;
pub use agent::*;
pub use execution::domain::*;
pub use execution::effects::{EffectError, StateEffect};
pub use model::*;
pub use state::{Scenario, initialize_economy_from_scenario, SimState};

use rand::{RngCore, SeedableRng};
use ravelin_traits::*;
use std::error::Error;

define_core!(
    RavelinCore,
    State = SimState,
    Action = SimAction,
    Effect = StateEffect,
    Scenario = Scenario,
    DomainRegistry = DomainRegistry
);

impl_named_trait_for_core!(ravelin_traits::SimulationAction, SimAction, RavelinCore, String);
impl_named_trait_for_core!(ravelin_traits::StateEffect, StateEffect, RavelinCore, String);

impl_domain_registry_for_core!(DomainRegistry, RavelinCore);

define_agent_adapter!(AgentAdapter, RavelinCore, crate::agent::Agent, SimState, SimAction);

impl ravelin_traits::SimulationScenario for Scenario {
    type Core = RavelinCore;

    fn from_file(path: &str) -> Result<Self, Box<dyn Error>> {
        Self::from_file(path)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl ravelin_traits::SimulationState for SimState {
    type Core = RavelinCore;

    fn advance_time(&mut self) {
        self.ticknum += 1;
        self.current_date += chrono::Duration::days(1);
    }

    fn get_agents(&self) -> Vec<Box<dyn AbstractAgent<Core = Self::Core>>> {
        self.collect_all_agents()
    }

    fn get_domain_registry(&self) -> &DomainRegistry {
        &self.domain_registry
    }

    fn apply_effects(&mut self, effects: &[StateEffect]) -> Result<(), Box<dyn Error + Send + Sync>> {
        execution::executor::TransactionExecutor::apply(effects, self)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }

    fn clear_markets_and_settle(&mut self) -> Vec<StateEffect> {
        let trades = self.financial_system.exchange.clear_markets();
        let mut settlement_effects = Vec::new();

        for trade in &trades {
            let total_value = trade.quantity * trade.price;
            let payment_action = SimAction::Transfer {
                agent_id: trade.buyer,
                from: trade.buyer,
                to: trade.seller,
                amount: total_value,
            };

            let payment_result = self.domain_registry.execute(&payment_action, self);

            if payment_result.success {
                settlement_effects.extend(payment_result.effects);

                if let MarketId::Goods(good_id) = &trade.market_id {
                    settlement_effects.push(StateEffect::RemoveInventory {
                        owner: trade.seller,
                        good_id: *good_id,
                        quantity: trade.quantity,
                    });
                    settlement_effects.push(StateEffect::AddInventory {
                        owner: trade.buyer,
                        good_id: *good_id,
                        quantity: trade.quantity,
                        unit_cost: trade.price,
                    });
                }
            } else {
                for error in payment_result.errors {
                    println!("Trade settlement failed for trade {:?}: {}", trade, error);
                }
            }
        }
        settlement_effects
    }

    fn initialize_from_scenario(scenario: &Scenario, rng: &mut dyn RngCore) -> Self {
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);
        let mut std_rng = rand::rngs::StdRng::from_seed(seed);
        initialize_economy_from_scenario(scenario, &mut std_rng)
    }

    fn get_stats_json(&self) -> serde_json::Value {
        #[derive(serde::Serialize)]
        struct Stats {
            m0: f64,
            m1: f64,
            m2: f64,
        }
        let stats = Stats {
            m0: self.financial_system.m0(),
            m1: self.financial_system.m1(),
            m2: self.financial_system.m2(),
        };
        serde_json::to_value(stats).unwrap_or_default()
    }
}