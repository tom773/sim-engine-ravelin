use rand::prelude::*;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::any::Any;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CentralBankDecisionModel {
    pub inflation_target: f64,
    pub employment_target: f64,
    pub reaction_function: ReactionFunction,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReactionFunction {
    TaylorRule { alpha: f64, beta: f64 },
    InflationTargeting { band_width: f64 },
    DualMandate { inflation_weight: f64, employment_weight: f64 },
}

impl Default for CentralBankDecisionModel {
    fn default() -> Self {
        Self {
            inflation_target: 0.02,
            employment_target: 0.95,
            reaction_function: ReactionFunction::TaylorRule { alpha: 1.5, beta: 0.5 },
        }
    }
}

#[typetag::serde]
impl DecisionModel for CentralBankDecisionModel {
    fn name(&self) -> &str {
        "Central Bank"
    }

    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let _cb = match agent.downcast_ref::<CentralBank>() {
            Some(cb) => cb,
            None => return vec![],
        };

        let mut intentions = Vec::new();
        let macro_stats = state.macro_stats();
        let cb_id = state.financial_system.central_bank.id;

        match &self.reaction_function {
            ReactionFunction::TaylorRule { alpha, beta } => {
                let inflation_gap = macro_stats.inflation_rate - self.inflation_target;
                let employment_gap = macro_stats.unemployment_rate - (1.0 - self.employment_target);

                let target_rate_change = alpha * inflation_gap - beta * employment_gap;

                if target_rate_change.abs() > 0.0025 {
                    // 25bps threshold
                    let current_rate = state.financial_system.central_bank.policy_rate_bps;
                    let new_rate = current_rate
                        + decimal_to_bps(Decimal::from_f64(target_rate_change).unwrap()).max(dec!(0)).min(dec!(2000)); // Cap at 20%

                    intentions.push(SimIntention::Monetary(MonetaryIntention::SetPolicyRate {
                        cb_id,
                        new_rate_bps: new_rate,
                    }));
                }

                if inflation_gap > 0.01 {
                    // Inflation too high
                    intentions.push(SimIntention::Monetary(MonetaryIntention::ConductOMO {
                        cb_id,
                        operation_type: OMOType::QuantitativeEasing,
                        amount: 1_000_000.0,
                    }));
                } else if inflation_gap < -0.01 {
                    // Inflation too low
                    intentions.push(SimIntention::Monetary(MonetaryIntention::ConductOMO {
                        cb_id,
                        operation_type: OMOType::QuantitativeTightening,
                        amount: 1_000_000.0,
                    }));
                }
            }
            ReactionFunction::InflationTargeting { band_width } => {
                let inflation_gap = macro_stats.inflation_rate - self.inflation_target;

                if inflation_gap.abs() > *band_width {
                    let current_rate = state.financial_system.central_bank.policy_rate_bps;
                    let adjustment = if inflation_gap > 0.0 { dec!(25) } else { dec!(-25) };
                    let new_rate = (current_rate + adjustment).max(dec!(0));

                    intentions.push(SimIntention::Monetary(MonetaryIntention::SetPolicyRate {
                        cb_id,
                        new_rate_bps: new_rate,
                    }));
                }
            }
            ReactionFunction::DualMandate { inflation_weight, employment_weight } => {
                let inflation_gap = macro_stats.inflation_rate - self.inflation_target;
                let employment_gap = macro_stats.unemployment_rate - (1.0 - self.employment_target);

                let combined_gap = inflation_weight * inflation_gap + employment_weight * employment_gap;

                if combined_gap.abs() > 0.005 {
                    if combined_gap > 0.0 {
                        intentions.push(SimIntention::Monetary(MonetaryIntention::ConductOMO {
                            cb_id,
                            operation_type: OMOType::QuantitativeEasing,
                            amount: 500_000.0,
                        }));
                    } else {
                        intentions.push(SimIntention::Monetary(MonetaryIntention::ConductOMO {
                            cb_id,
                            operation_type: OMOType::QuantitativeTightening,
                            amount: 500_000.0,
                        }));
                    }
                }
            }
        }

        intentions
    }
}
