use chrono::{NaiveDate, Datelike};
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
    fn decide(
        &self,
        agent: &dyn Any,
        state: &SimState,
        _rng: &mut dyn RngCore,
    ) -> Vec<SimIntention> {
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
    fn collect_taxes(
        &self,
        government: &Government,
        state: &SimState,
        intentions: &mut Vec<SimIntention>,
    ) {
        if state.current_date.day() != 15 {
            return;
        }
        let tax_rate = government.tax_rates.income_tax;

        for consumer in state.agents.consumers.values() {
            let monthly_tax_liability = (consumer.income / 12.0) * tax_rate;
            if monthly_tax_liability > 0.0 {
                intentions.push(SimIntention::Fiscal(FiscalIntention::CollectTaxes {
                    government_id: government.id,
                    target: consumer.id,
                    amount: monthly_tax_liability,
                }));
            }
        }
    }

fn handle_funding(
        &self,
        government: &Government,
        state: &SimState,
        intentions: &mut Vec<SimIntention>,
    ) {
        let fs = &state.financial_system;
        let current_balance = fs.get_liquid_assets(&government.id);
        
        let tga_target = 5_000_000.0;
        
        if state.current_date.day() == 2 || current_balance > tga_target {
            return;
        }

        let deficit = (tga_target - current_balance).max(0.0) + 500_000.0;

        let current_date = state.current_date;

        let maturity_date = TimePeriod::Years(5).add_to_date(current_date);
        let coupon_rate = fs.central_bank.policy_rate_bps;
        const FACE_VALUE: f64 = 1000.0;
        
        let quantity_to_issue = (deficit / FACE_VALUE).ceil() as u32;

        if quantity_to_issue > 0 {
            intentions.push(SimIntention::Fiscal(FiscalIntention::AnnounceDebtAuction {
                government_id: government.id,
                maturity_date,
                coupon_rate,
                quantity_to_issue,
            }));
        }
    }
}

fn _standardize_maturity(date: NaiveDate) -> NaiveDate {
    let (year, quarter) = (date.year(), date.quarter());
    let (month, day) = match quarter {
        1 => (3, 15),
        2 => (6, 15),
        3 => (9, 15),
        4 => (12, 15),
        _ => unreachable!(),
    };
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}