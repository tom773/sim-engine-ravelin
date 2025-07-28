use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CentralBank {
    pub id: AgentId,
    pub policy_rate: f64,         // The interest rate set by monetary policy
    pub reserve_requirement: f64, // Required reserve ratio for commercial banks
}

impl CentralBank {
    pub fn new(policy_rate: f64, reserve_requirement: f64) -> Self {
        let id = AgentId(Uuid::new_v4());
        Self {
            id,
            policy_rate,
            reserve_requirement,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bank {
    pub id: AgentId,
    pub name: String,
    pub lending_spread: f64, // Basis points above policy rate for loans
    pub deposit_spread: f64, // Basis points below policy rate for deposits
}

impl Bank {
    pub fn new(name: String, lending_spread: f64, deposit_spread: f64) -> Self {
        let id = AgentId(Uuid::new_v4());
        Self {
            id,
            name,
            lending_spread,
            deposit_spread,
        }
    }

    pub fn total_liabilities(&self, fs: &FinancialSystem) -> f64 {
        fs.get_total_liabilities(&self.id)
    }

    pub fn total_assets(&self, fs: &FinancialSystem) -> f64 {
        fs.get_total_assets(&self.id)
    }

    pub fn liquidity(&self, fs: &FinancialSystem) -> f64 {
        fs.liquidity(&self.id)
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
                    .filter(|inst| {
                        matches!(inst.instrument_type, InstrumentType::CentralBankReserves)
                    })
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
    fn decide(
        &self,
        fs: &FinancialSystem,
        _rng: &mut rand::prelude::StdRng,
    ) -> Vec<Self::DecisionType> {
        let mut decisions = Vec::new();
        
        if !self.meets_reserve_requirement(fs) {
            let amount_needed = fs.central_bank.reserve_requirement * self.total_liabilities(fs) - self.get_reserves(fs);
            if amount_needed > 0.0 {
                decisions.push(BankDecision::BorrowOvernight {
                    amount_dollars: amount_needed,
                    max_annual_rate_bps: 100.0, // Example rate
                });
            }
        }

        if self.liquidity(fs) > 1000.0 { // Example threshold
            decisions.push(BankDecision::LendOvernight {
                amount_dollars: 1000.0, // Example amount
                min_annual_rate_bps: 50.0, // Example rate
            });
        }

        decisions
    }
    fn act(&self, decisions: &[BankDecision]) -> Vec<SimAction> {
        let mut actions = Vec::new();
        let market_id = FinancialMarketId::SecuredOvernightFinancing;

        for decision in decisions {
            match *decision {
                BankDecision::BorrowOvernight {
                    amount_dollars,
                    max_annual_rate_bps,
                } => {

                    let daily_rate = market_id.annual_bps_to_daily_rate(max_annual_rate_bps);
                    let min_price = 1.0 / (1.0 + daily_rate);

                    let face_value_to_sell = amount_dollars / min_price;

                    actions.push(SimAction::PostAsk {
                        agent_id: self.id.clone(),
                        market_id: FinancialMarketId::SecuredOvernightFinancing,
                        quantity: face_value_to_sell, // Quantity is the face value
                        price: min_price,             // Price is per $1 of face value
                    });
                }

                BankDecision::LendOvernight {
                    amount_dollars,
                    min_annual_rate_bps,
                } => {

                    let daily_rate = market_id.annual_bps_to_daily_rate(min_annual_rate_bps);
                    let max_price = 1.0 / (1.0 + daily_rate);

                    let face_value_to_buy = amount_dollars / max_price;

                    actions.push(SimAction::PostBid {
                        agent_id: self.id.clone(),
                        market_id: FinancialMarketId::SecuredOvernightFinancing,
                        quantity: face_value_to_buy, // Quantity is the face value
                        price: max_price,            // Price is per $1 of face value
                    });
                }
            }
        }
        actions
    }
}
