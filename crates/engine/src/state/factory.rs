// crates/engine/src/state/factory.rs
use shared::*;
use uuid::Uuid;
use rand::prelude::*;

use crate::SimState;

pub struct AgentFactory<'a> {
    pub ss: &'a mut SimState,
    pub rng: &'a mut StdRng,
}

impl<'a> AgentFactory<'a> {
    pub fn new(ss: &'a mut SimState, rng: &'a mut StdRng) -> Self {
        Self { ss, rng }
    }
    
    pub fn create_consumer(&mut self, bank_id: AgentId, model: Box<dyn DecisionModel>) -> Consumer {
        let agent_id = AgentId(Uuid::new_v4());
        
        self.ss.financial_system.balance_sheets.insert(
            agent_id.clone(),
            BalanceSheet::new(agent_id.clone())
        );
        println!("Creating consumer with ID: {}", agent_id.0);
        println!("{:?}", self.ss.financial_system.central_bank.id.clone());
        let cash = cash!(
            agent_id.clone(),
            1000.0,
            self.ss.financial_system.central_bank.id.clone(),
            self.ss.ticknum
        );
        self.ss.financial_system.create_instrument(cash).unwrap();
        let c = Consumer::new(
            self.rng.random_range(18..65),
            agent_id,
            bank_id,
            model,
        );
        self.ss.consumers.push(c.clone());
        c
    }
    
    pub fn create_bank(&mut self, name: String, lending_spread: f64, deposit_spread: f64) -> Bank {
        let bank = Bank::new(name, lending_spread, deposit_spread);
        
        self.ss.financial_system.balance_sheets.insert(
            bank.id.clone(),
            BalanceSheet::new(bank.id.clone())
        );
        self.ss.financial_system.commercial_banks.insert(bank.id.clone(), bank.clone()); 
        bank
    }
    
    pub fn create_firm(&mut self, name: String, bank_id: AgentId) -> Firm {
        let firm_id = AgentId(Uuid::new_v4());
        
        self.ss.financial_system.balance_sheets.insert(
            firm_id.clone(),
            BalanceSheet::new(firm_id.clone())
        );

        let f = Firm::new(firm_id, bank_id, name);
        self.ss.firms.push(f.clone());
        f
    }
}