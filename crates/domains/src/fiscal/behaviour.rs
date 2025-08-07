use sim_core::*;
use std::any::Any;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use chrono::Datelike;

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct BasicGovernmentDecisionModel;

#[typetag::serde]
impl DecisionModel for BasicGovernmentDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimAction> {
        let government = match agent.downcast_ref::<Government>() {
            Some(g) => g,
            None => return vec![],
        };

        let mut actions = Vec::new();
        if state.current_date.ordinal() % 30 == 0{ // Every 30 days, collect taxes 
            let tax_rate = government.tax_rates.income_tax;

            for consumer in state.agents.consumers.values() {
                let tax_liability = (consumer.income/12.0) * tax_rate; // Monthly income tax
                if tax_liability > 0.0 {
                    actions.push(SimAction::Banking(BankingAction::Transfer {
                        from: consumer.id,
                        to: government.id,
                        amount: tax_liability,
                    }));
                }
            }
        }
        actions
    }
}