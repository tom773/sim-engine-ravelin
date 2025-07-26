use shared::*;
use uuid::Uuid;
use rand::prelude::*;
use fake::{Fake, Faker};
use fake::faker::company::en::*;
use fake::faker::name::en::*;

use crate::SimState;

pub struct AgentFactory<'a> {
    pub ss: &'a mut SimState,
    pub rng: &'a mut StdRng,
}

impl<'a> AgentFactory<'a> {
    pub fn new(ss: &'a mut SimState, rng: &'a mut StdRng) -> Self {
        Self { ss, rng }
    }
    
    pub fn create_consumer_with_income(&mut self, bank_id: AgentId, income_percentile: f64) -> Consumer {
        let agent_id = AgentId(Uuid::new_v4());
        
        self.ss.financial_system.balance_sheets.insert(
            agent_id.clone(),
            BalanceSheet::new(agent_id.clone())
        );
        
        let annual_income = if income_percentile < 0.2 {
            self.rng.random_range(20_000.0..40_000.0)
        } else if income_percentile < 0.5 {
            self.rng.random_range(40_000.0..70_000.0)
        } else if income_percentile < 0.8 {
            self.rng.random_range(70_000.0..120_000.0)
        } else if income_percentile < 0.95 {
            self.rng.random_range(120_000.0..250_000.0)
        } else {
            self.rng.random_range(250_000.0..500_000.0)
        };
        
        let propensity_to_consume = if income_percentile < 0.3 {
            self.rng.random_range(0.85..0.95)
        } else if income_percentile < 0.7 {
            self.rng.random_range(0.70..0.85)
        } else {
            self.rng.random_range(0.50..0.70)
        };
        
        let decision_model = Box::new(BasicDecisionModel { propensity_to_consume });
        
        let mut c = Consumer::new(
            self.rng.random_range(18..65),
            agent_id.clone(),
            bank_id.clone(),
            decision_model,
        );
        c.income = annual_income / 52.0; // Weekly income
        
        let cash = cash!(
            agent_id.clone(),
            1000.0,
            self.ss.financial_system.central_bank.id.clone(),
            self.ss.ticknum
        );
        self.ss.financial_system.create_instrument(cash).unwrap();
        
        let initial_savings = c.income * self.rng.random_range(4.0..12.0);
        let deposit = deposit!(
            agent_id.clone(),
            bank_id.clone(),
            initial_savings,
            self.ss.financial_system.central_bank.policy_rate - 100.0,
            self.ss.ticknum
        );
        self.ss.financial_system.create_instrument(deposit).unwrap();
        
        self.ss.consumers.push(c.clone());
        c
    }
    
    pub fn create_consumer(&mut self, bank_id: AgentId, _model: Box<dyn DecisionModel>) -> Consumer {
        let income_percentile = self.rng.random::<f64>();
        self.create_consumer_with_income(bank_id, income_percentile)
    }
    
    pub fn create_bank_with_name(&mut self) -> Bank {
        let bank_names = ["Bank", "Financial", "Savings & Loan", "Credit Union"];
        let name = format!("{} {}", 
            CompanyName().fake::<String>(), 
            bank_names[self.rng.random_range(0..bank_names.len())]
        );
        
        let lending_spread = self.rng.random_range(200.0..300.0);
        let deposit_spread = self.rng.random_range(30.0..80.0);
        
        self.create_bank(name, lending_spread, deposit_spread)
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
    
    pub fn create_firm_with_capital(&mut self, bank_id: AgentId) -> Firm {
        let firm_name = CompanyName().fake::<String>();
        let firm = self.create_firm(firm_name, bank_id.clone());
        
        let loan_amount = self.rng.random_range(50_000.0..500_000.0);
        let loan = loan!(
            bank_id.clone(),
            firm.id.clone(),
            loan_amount,
            self.ss.financial_system.central_bank.policy_rate + 250.0,
            60, // 5 year term
            LoanType::Personal, // TODO: Add Business loan type
            self.ss.ticknum
        );
        self.ss.financial_system.create_instrument(loan).unwrap();
        
        let deposit = deposit!(
            firm.id.clone(),
            bank_id.clone(),
            loan_amount,
            self.ss.financial_system.central_bank.policy_rate - 50.0,
            self.ss.ticknum
        );
        self.ss.financial_system.create_instrument(deposit).unwrap();
        
        firm
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