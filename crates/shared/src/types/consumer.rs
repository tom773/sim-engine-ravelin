use crate::*;
use serde::{Serialize, Deserialize};
use rand::prelude::*;
use crate::FeatureSource;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Consumer {
    pub id: AgentId,
    pub age: u32,
    pub bank_id: AgentId,
    pub decision_model: Box<dyn DecisionModel>,
    pub income: f64, // Annual income
}

impl Consumer {
    pub fn new(age: u32, id: AgentId, bank_id: AgentId, dm: Box<dyn DecisionModel>) -> Self {
        let mut rng = rand::rng();
        Self {
            id,
            bank_id,
            age,
            decision_model: dm,
            income: (rng.random_range(30000.0..80000.0))/52.0,
        }
    }
    pub fn snip_id(&self) -> String { self.id.0.to_string().chars().take(4).collect() }
    pub fn balance_sheet<'a>(&self, fs: &'a FinancialSystem) -> Option<&'a BalanceSheet> {
        fs.get_bs_by_id(&self.id)
    }
    
    pub fn get_cash_holdings(&self, fs: &FinancialSystem) -> f64 {
        fs.get_cash_assets(&self.id)
    }
    
    pub fn get_deposits(&self, fs: &FinancialSystem) -> f64 {
        fs.get_deposits_at_bank(&self.id, &self.bank_id)
    }
}

impl Agent for Consumer {
    fn act(&self, decision: &Decision) -> Vec<Action> {
        let mut actions = Vec::new();

        // Save $1000 
        if decision.save_amount > 0.0 {
            actions.push(Action::DepositCash { amount: 1000.0 });
        }
        
        actions
    }
    
    fn decide(&self, _fs: &FinancialSystem, _rng: &mut StdRng) -> Decision {
        //self.decision_model.decide(self, fs, rng)
        Decision { spend_amount: 0.0, save_amount: 1000.0, total_available: 1000.0 }
    }
}

impl FeatureSource for Consumer {
    fn get_age(&self) -> u32 {
        self.age
    }
    
    fn get_income(&self) -> f64 {
        self.income
    }
    
    fn get_savings(&self) -> f64 {
        // TODO: Implement properly from balance sheet
        0.0
    }
    
    fn get_debt(&self) -> f64 {
        // TODO: Implement properly from balance sheet
        0.0
    }
}