use crate::*;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CentralBank {
    pub id: AgentId,
    pub policy_rate: f64,
    pub reserve_requirement: f64, 
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bank {
    pub id: AgentId,
    pub name: String,
    pub lending_spread: f64,
    pub deposit_spread: f64,
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
        fs.balance_sheets.get(&self.id)
            .map(|bs| bs.total_liabilities())
            .unwrap_or(0.0)
    }
    
    pub fn total_assets(&self, fs: &FinancialSystem) -> f64 {
        fs.balance_sheets.get(&self.id)
            .map(|bs| bs.total_assets())
            .unwrap_or(0.0)
    }
    
    pub fn liquidity(&self, fs: &FinancialSystem) -> f64 {
        fs.balance_sheets.get(&self.id)
            .map(|bs| bs.assets.values()
                .filter(|inst| matches!(
                    inst.instrument_type, 
                    InstrumentType::Cash
                ))
                .map(|inst| inst.principal)
                .sum()
            )
            .unwrap_or(0.0)
    }
}