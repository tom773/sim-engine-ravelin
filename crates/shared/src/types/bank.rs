use crate::*;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CentralBank {
    pub id: AgentId,
    pub policy_rate: f64,           // The interest rate set by monetary policy
    pub reserve_requirement: f64,    // Required reserve ratio for commercial banks
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
    pub lending_spread: f64,    // Basis points above policy rate for loans
    pub deposit_spread: f64,    // Basis points below policy rate for deposits
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
            .map(|bs| bs.assets.values()
                .filter(|inst| matches!(inst.instrument_type, InstrumentType::CentralBankReserves))
                .map(|inst| inst.principal)
                .sum()
            )
            .unwrap_or(0.0)
    }
    
    pub fn meets_reserve_requirement(&self, fs: &FinancialSystem) -> bool {
        let deposits = self.total_liabilities(fs);
        let reserves = self.get_reserves(fs);
        let required = deposits * fs.central_bank.reserve_requirement;
        reserves >= required
    }
}