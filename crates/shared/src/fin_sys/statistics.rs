use crate::*;
use serde::{Serialize, Deserialize};

impl FinancialStatistics for FinancialSystem { 
   fn m0(&self) -> f64 {
        self.balance_sheets
            .get(&self.central_bank.id)
            .map(|cb_bs| cb_bs.liabilities.values()
                .map(|inst| inst.principal)
                .sum()
            )
            .unwrap_or(0.0)
    }
    fn m1(&self) -> f64 {
        let mut m1 = 0.0;
        
        for (agent_id, bs) in &self.balance_sheets {
            if *agent_id == self.central_bank.id || self.commercial_banks.contains_key(agent_id) {
                continue;
            }
            
            m1 += bs.assets.values()
                .filter(|inst| matches!(inst.instrument_type, InstrumentType::Cash))
                .map(|inst| inst.principal)
                .sum::<f64>();
                
            m1 += bs.assets.values()
                .filter(|inst| matches!(inst.instrument_type, InstrumentType::DemandDeposit))
                .map(|inst| inst.principal)
                .sum::<f64>();
        }
        
        m1
    }
    fn m2(&self) -> f64 {
        let mut m2 = self.m1();
        
        for (agent_id, bs) in &self.balance_sheets {
            if *agent_id == self.central_bank.id || self.commercial_banks.contains_key(agent_id) {
                continue;
            }
            
            m2 += bs.assets.values()
                .filter(|inst| matches!(inst.instrument_type, InstrumentType::SavingsDeposit { .. }))
                .map(|inst| inst.principal)
                .sum::<f64>();
        }
        
        m2
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonetaryAggregates {
    pub public_cash: f64,
    pub bank_cash: f64,
    pub bank_reserves: f64,
    pub demand_deposits: f64,
    pub savings_deposits: f64,
    pub m0: f64,
    pub m1: f64,
    pub m2: f64,
} 