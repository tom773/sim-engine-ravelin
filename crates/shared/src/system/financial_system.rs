
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::{*, types::*};
use uuid::Uuid;
use chrono::Utc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialSystem {
    pub instruments: HashMap<InstrumentId, FinancialInstrument>,
    pub balance_sheets: HashMap<AgentId, BalanceSheet>,
    pub central_bank: CentralBank,
    pub commercial_banks: HashMap<AgentId, Bank>,
    pub exchange: Exchange, // Markets for goods
}

impl FinancialSystem {
    pub fn create_instrument(&mut self, instrument: FinancialInstrument) -> Result<(), String> {
        let id = instrument.id.clone();

        if let Some(creditor_bs) = self.balance_sheets.get_mut(&instrument.creditor) {
            creditor_bs.assets.insert(id.clone(), instrument.clone());
        } else {
            return Err("Creditor not found".to_string());
        }
        if let Some(debtor_bs) = self.balance_sheets.get_mut(&instrument.debtor) {
            debtor_bs.liabilities.insert(id.clone(), instrument.clone());
        } else {
            if let Some(creditor_bs) = self.balance_sheets.get_mut(&instrument.creditor) {
                creditor_bs.assets.remove(&id);
            }
            return Err("Debtor not found".to_string());
        }
        self.instruments.insert(id, instrument);
        Ok(())
    }

    pub fn transfer_instrument(
        &mut self, 
        instrument_id: &InstrumentId, 
        new_creditor: AgentId
    ) -> Result<(), String> {
        let instrument = self.instruments.get_mut(instrument_id)
            .ok_or("Instrument not found")?;
        
        let old_creditor = instrument.creditor.clone();
        
        if let Some(old_bs) = self.balance_sheets.get_mut(&old_creditor) {
            old_bs.assets.remove(instrument_id);
        }
        
        instrument.creditor = new_creditor.clone();
        if let Some(new_bs) = self.balance_sheets.get_mut(&new_creditor) {
            new_bs.assets.insert(instrument_id.clone(), instrument.clone());
        }
        
        Ok(())
    }
    pub fn m0(&self) -> f64 {
        self.balance_sheets
            .get(&self.central_bank.id)
            .map(|cb_bs| cb_bs.liabilities.values()
                .map(|inst| inst.principal)
                .sum()
            )
            .unwrap_or(0.0)
    }
    
    pub fn m1(&self) -> f64 {
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
    
    pub fn m2(&self) -> f64 {
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
    pub fn monetary_aggregates_breakdown(&self) -> MonetaryAggregates {
        let mut public_cash = 0.0;
        let mut bank_cash = 0.0;
        let mut bank_reserves = 0.0;
        let mut demand_deposits = 0.0;
        let mut savings_deposits = 0.0;
        
        for (agent_id, bs) in &self.balance_sheets {
            let is_bank = self.commercial_banks.contains_key(agent_id);
            let is_central_bank = *agent_id == self.central_bank.id;
            
            if is_central_bank {
                continue; // Central bank doesn't hold these assets
            }
            
            for inst in bs.assets.values() {
                match &inst.instrument_type {
                    InstrumentType::Cash => {
                        if is_bank {
                            bank_cash += inst.principal;
                        } else {
                            public_cash += inst.principal;
                        }
                    }
                    InstrumentType::CentralBankReserves => {
                        if is_bank {
                            bank_reserves += inst.principal;
                        }
                    }
                    InstrumentType::DemandDeposit => {
                        if !is_bank {
                            demand_deposits += inst.principal;
                        }
                    }
                    InstrumentType::SavingsDeposit { .. } => {
                        if !is_bank {
                            savings_deposits += inst.principal;
                        }
                    }
                    _ => {}
                }
            }
        }
        
        MonetaryAggregates {
            public_cash,
            bank_cash,
            bank_reserves,
            demand_deposits,
            savings_deposits,
            m0: bank_cash + bank_reserves + public_cash,
            m1: public_cash + demand_deposits,
            m2: public_cash + demand_deposits + savings_deposits,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonetaryAggregates {
    pub public_cash: f64,           // Cash held by non-banks
    pub bank_cash: f64,             // Cash held by banks (part of reserves)
    pub bank_reserves: f64,         // Reserves at central bank
    pub demand_deposits: f64,       // Checking accounts
    pub savings_deposits: f64,      // Savings accounts
    pub m0: f64,                    // Monetary base
    pub m1: f64,                    // M0 cash in public + demand deposits
    pub m2: f64,                    // M1 + savings deposits
} 

impl Default for FinancialSystem {
    fn default() -> Self {
        let central_bank = CentralBank::new(430.0, 0.1);
        let cb_id = central_bank.id.clone();
        
        let mut balance_sheets = HashMap::new();
        balance_sheets.insert(cb_id.clone(), BalanceSheet::new(cb_id.clone()));
        
        Self {
            exchange: Exchange::new(),
            instruments: HashMap::new(),
            balance_sheets,
            central_bank,
            commercial_banks: HashMap::new(),
        }
    }
}