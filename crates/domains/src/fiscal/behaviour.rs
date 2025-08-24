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
        self.handle_funding(government, state, &mut actions);
        actions
    }
}

impl BasicGovernmentDecisionModel {
    fn handle_funding(&self, government: &Government, state: &SimState, actions: &mut Vec<SimAction>) {
        let fs = &state.financial_system;
        let gbs = fs.balance_sheets.get(&government.id);
        if gbs.is_none() {
            return;
        }
        let gbs = gbs.unwrap();
        let current_balance = gbs.liquid_assets(); 
        let spending_target = 1_000_000.0 / 12.0; // 1m / 12
        if current_balance < spending_target {
            let deficit = spending_target - current_balance;
            // Issuance distribution across different tenors
            let issue_distribution = [
                (Tenor::T2Y, 0.15), 
                (Tenor::T5Y, 0.25), 
                (Tenor::T10Y, 0.40), 
                (Tenor::T30Y, 0.20)
            ];
            let face_value = 1000.0;
            // Use the central bank policy rate as a proxy for the new coupon rate
            let coupon_rate = fs.central_bank.policy_rate_bps;
            for (tenor, percentage) in issue_distribution {
                let amount_to_issue = deficit * percentage;
                let quantity = (amount_to_issue / face_value).ceil() as u32;
                if quantity > 0 {
                    actions.push(SimAction::Fiscal(FiscalAction::IssueDebt {
                        government_id: government.id,
                        tenor,
                        quantity,
                        face_value,
                        coupon_rate,
                    }));
                }
            }
        }
    }
}