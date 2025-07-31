use std::fmt::Debug;
use serde::{Deserialize, Serialize};
use crate::*; 
use dyn_clone::{DynClone, clone_trait_object};
use rand::prelude::*;

#[typetag::serde(tag = "type")]
pub trait BankDecisionModel: DynClone + Send + Sync {
    fn decide(&self, bank: &Bank, fs: &FinancialSystem, rng: &mut dyn RngCore) -> Vec<BankDecision>;
}
clone_trait_object!(BankDecisionModel);

impl Debug for dyn BankDecisionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BankDecisionModel")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasicBankDecisionModel;

#[typetag::serde]
impl BankDecisionModel for BasicBankDecisionModel {
    fn decide(&self, bank: &Bank, fs: &FinancialSystem, _rng: &mut dyn RngCore) -> Vec<BankDecision> {
        let mut decisions = Vec::new();

        let total_deposits = bank.total_liabilities(fs);
        let required_reserves = total_deposits * fs.central_bank.reserve_requirement;

        let desired_buffer = total_deposits * 0.02;
        let target_reserve_level = required_reserves + desired_buffer;

        let current_reserves = bank.get_reserves(fs);
        let reserve_surplus_or_shortfall = current_reserves - target_reserve_level;

        if reserve_surplus_or_shortfall < 0.0 {
            let amount_needed = -reserve_surplus_or_shortfall;
            decisions.push(BankDecision::BorrowOvernight {
                amount_dollars: amount_needed,
                max_annual_rate_bps: fs.central_bank.policy_rate + 50.0,
            });
        } else if reserve_surplus_or_shortfall > 0.0 {
            let lendable_surplus = reserve_surplus_or_shortfall;
            let portion_to_lend = 0.75;

            let amount_to_lend = lendable_surplus * portion_to_lend;

            if amount_to_lend > 100.0 {

                decisions.push(BankDecision::LendOvernight {
                    amount_dollars: amount_to_lend,
                    min_annual_rate_bps: fs.central_bank.policy_rate - 25.0,
                });
            }
        }

        decisions
    }
}