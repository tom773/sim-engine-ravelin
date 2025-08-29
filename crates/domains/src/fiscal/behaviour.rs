use chrono::Datelike;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::any::Any;

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct BasicGovernmentDecisionModel;

#[typetag::serde]
impl DecisionModel for BasicGovernmentDecisionModel {
    fn name(&self) -> &str {
        "Government"
    }
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let government = match agent.downcast_ref::<Government>() {
            Some(g) => g,
            None => return vec![],
        };

        let mut intentions = Vec::new();
        self.collect_taxes(government, state, &mut intentions);
        self.handle_funding(government, state, &mut intentions);

        intentions
    }
}

impl BasicGovernmentDecisionModel {
    fn collect_taxes(&self, government: &Government, state: &SimState, intentions: &mut Vec<SimIntention>) {
        if (state.current_date.day() == 15) == false {
            return;
        }
        let tax_rate = government.tax_rates.income_tax;

        for consumer in state.agents.consumers.values() {
            let monthly_tax_liability = (consumer.income / 12.0) * tax_rate;
            if monthly_tax_liability > 0.0 {
                intentions.push(SimIntention::CollectTaxes {
                    government_id: government.id,
                    target: consumer.id,
                    amount: monthly_tax_liability,
                });
            }
        }
    }

    fn handle_funding(&self, government: &Government, state: &SimState, intentions: &mut Vec<SimIntention>) {
        let fs = &state.financial_system;
        let current_balance = fs.get_liquid_assets(&government.id);
        let monthly_spending_target = 1_000_000.0 / 12.0;

        if current_balance < monthly_spending_target {
            let deficit = monthly_spending_target - current_balance;

            let issue_distribution = [
                (Tenor::T1M, 0.05), (Tenor::T2M, 0.05), (Tenor::T3M, 0.10), (Tenor::T6M, 0.10), (Tenor::T1Y, 0.10),
                (Tenor::T2Y, 0.15), (Tenor::T5Y, 0.25), (Tenor::T10Y, 0.40), (Tenor::T30Y, 0.20)];

            let coupon_rate = fs.central_bank.policy_rate_bps;

            for (tenor, percentage) in issue_distribution {
                let amount_to_raise = deficit * percentage;
                if amount_to_raise > 0.0 {
                    intentions.push(SimIntention::IssueDebtToRaise {
                        government_id: government.id,
                        tenor,
                        amount_to_raise,
                        coupon_rate,
                    });
                }
            }
        }
    }
}
