use crate::*;
use dyn_clone::{DynClone, clone_trait_object};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CentralBank {
    pub id: AgentId,
    pub policy_rate: f64,
    pub reserve_requirement: f64,
}

impl CentralBank {
    pub fn new(policy_rate: f64, reserve_requirement: f64) -> Self {
        let id = AgentId(Uuid::new_v4());
        Self { id, policy_rate, reserve_requirement }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BankDecision {
    BorrowOvernight { amount_dollars: f64, max_annual_rate_bps: f64 },
    LendOvernight { amount_dollars: f64, min_annual_rate_bps: f64 },
}

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bank {
    pub id: AgentId,
    pub name: String,
    pub lending_spread: f64,
    pub deposit_spread: f64,
    pub decision_model: Box<dyn BankDecisionModel>,
}

impl Bank {
    pub fn new(name: String, lending_spread: f64, deposit_spread: f64) -> Self {
        let id = AgentId(Uuid::new_v4());
        Self { id, name, lending_spread, deposit_spread, decision_model: Box::new(BasicBankDecisionModel) }
    }

    pub fn total_liabilities(&self, fs: &FinancialSystem) -> f64 {
        fs.get_total_liabilities(&self.id)
    }

    pub fn total_assets(&self, fs: &FinancialSystem) -> f64 {
        fs.get_total_assets(&self.id)
    }

    pub fn liquidity(&self, fs: &FinancialSystem) -> f64 {
        fs.get_bs_by_id(&self.id)
            .map(|bs| {
                bs.assets
                    .values()
                    .filter(|inst| {
                        inst.details.as_any().is::<CashDetails>() ||
                        inst.details.as_any().is::<BondDetails>() ||
                        inst.details.as_any().is::<CentralBankReservesDetails>()
                    })
                    .map(|inst| inst.principal)
                    .sum()
            })
            .unwrap_or(0.0)
    }

    pub fn get_deposit_rate(&self, fs: &FinancialSystem) -> f64 {
        fs.central_bank.policy_rate - self.deposit_spread
    }
    pub fn get_lending_rate(&self, fs: &FinancialSystem) -> f64 {
        fs.central_bank.policy_rate + self.lending_spread
    }
    pub fn get_reserves(&self, fs: &FinancialSystem) -> f64 {
        fs.balance_sheets
            .get(&self.id)
            .map(|bs| {
                bs.assets
                    .values()
                    .filter(|inst| inst.details.as_any().is::<CentralBankReservesDetails>())
                    .map(|inst| inst.principal)
                    .sum()
            })
            .unwrap_or(0.0)
    }

    pub fn meets_reserve_requirement(&self, fs: &FinancialSystem) -> bool {
        let deposits = self.total_liabilities(fs);
        let reserves = self.get_reserves(fs);
        let required = deposits * fs.central_bank.reserve_requirement;
        reserves >= required
    }
}
impl Agent for Bank {
    type DecisionType = BankDecision;

    fn decide(&self, fs: &FinancialSystem, rng: &mut rand::prelude::StdRng) -> Vec<Self::DecisionType> {
        self.decision_model.decide(self, fs, rng)
    }

    fn act(&self, decisions: &[BankDecision]) -> Vec<SimAction> {
        let mut actions = Vec::new();
        let market_id = FinancialMarketId::SecuredOvernightFinancing;

        for decision in decisions {
            match *decision {
                BankDecision::BorrowOvernight { amount_dollars, max_annual_rate_bps } => {
                    let daily_rate = market_id.annual_bps_to_daily_rate(max_annual_rate_bps);
                    let min_price = 1.0 / (1.0 + daily_rate);
                    let face_value_to_sell = amount_dollars / min_price;

                    actions.push(SimAction::PostBid {
                        agent_id: self.id.clone(),
                        market_id: MarketId::Financial(market_id.clone()),
                        quantity: face_value_to_sell,
                        price: min_price,
                    });
                }

                BankDecision::LendOvernight { amount_dollars, min_annual_rate_bps } => {
                    let daily_rate = market_id.annual_bps_to_daily_rate(min_annual_rate_bps);
                    let max_price = 1.0 / (1.0 + daily_rate);
                    let face_value_to_buy = amount_dollars / max_price;

                    actions.push(SimAction::PostAsk {
                        agent_id: self.id.clone(),
                        market_id: MarketId::Financial(market_id.clone()),
                        quantity: face_value_to_buy,
                        price: max_price,
                    });
                }
            }
        }
        actions
    }
}
