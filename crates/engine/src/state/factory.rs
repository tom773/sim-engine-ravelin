use shared::*;
use uuid::Uuid;
use rand::prelude::*;
use fake::Fake;
use fake::faker::company::en::*;

use crate::SimState;

pub struct AgentFactory<'a> {
    pub ss: &'a mut SimState,
    pub rng: &'a mut StdRng,
}

impl<'a> AgentFactory<'a> {
    pub fn new(ss: &'a mut SimState, rng: &'a mut StdRng) -> Self {
        Self { ss, rng }
    }
    
    pub fn create_consumer(&mut self, bank_id: AgentId) -> Consumer {
        let agent_id = AgentId(Uuid::new_v4());
        
        self.ss.financial_system.balance_sheets.insert(
            agent_id.clone(),
            BalanceSheet::new(agent_id.clone())
        );
        
        let age = 35;
        let annual_income = 60_000.0;
        let propensity_to_consume = 0.7;
        
        let decision_model = Box::new(BasicDecisionModel { propensity_to_consume });
        
        let mut c = Consumer::new(
            age,
            agent_id.clone(),
            bank_id.clone(),
            decision_model,
        );
        c.income = annual_income / 52.0;
        
        self.ss.consumers.push(c.clone());
        c
    }
    
    pub fn create_bank(&mut self) -> Bank {
        let bank_names = ["Bank", "Financial", "Savings & Loan", "Credit Union"];
        let name = format!("{} {}", 
            CompanyName().fake::<String>(), 
            bank_names[self.rng.random_range(0..bank_names.len())]
        );
        let lending_spread = 250.0;
        let deposit_spread = 50.0;
        
        let bank = Bank::new(name, lending_spread, deposit_spread);
        
        self.ss.financial_system.balance_sheets.insert(
            bank.id.clone(),
            BalanceSheet::new(bank.id.clone())
        );
        
        self.ss.financial_system.commercial_banks.insert(bank.id.clone(), bank.clone());
        bank
    }
    
    pub fn create_firm(&mut self, bank_id: AgentId) -> Firm {
        let firm_id = AgentId(Uuid::new_v4());
        let firm_name = CompanyName().fake::<String>();
        
        self.ss.financial_system.balance_sheets.insert(
            firm_id.clone(),
            BalanceSheet::new(firm_id.clone())
        );
        
        self.ss.financial_system.exchange.goods_market_mut(&GoodId::generic())
            .unwrap()
            .post_ask(firm_id.clone(), GoodId::generic(), 100.0, 25.0);
        
        let mut f = Firm::new(firm_id, bank_id, firm_name);
        f.employees = 5;
        f.wage_rate = 20.0;
        f.productivity = 2.0;
        
        self.ss.firms.push(f.clone());
        f
    }
}