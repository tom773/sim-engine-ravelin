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
    pub income: f64, // Weekly income
}

impl Consumer {
    pub fn new(age: u32, id: AgentId, bank_id: AgentId, dm: Box<dyn DecisionModel>) -> Self {
        let mut rng = rand::rng();
        Self {
            id,
            bank_id,
            age,
            decision_model: dm,
            income: (rng.random_range(30000.0..80000.0))/52.0, // Convert annual to weekly
        }
    }
    
    pub fn snip_id(&self) -> String { 
        self.id.0.to_string().chars().take(4).collect() 
    }
    
    pub fn balance_sheet<'a>(&self, fs: &'a FinancialSystem) -> Option<&'a BalanceSheet> {
        fs.get_bs_by_id(&self.id)
    }
    
    pub fn get_cash_holdings(&self, fs: &FinancialSystem) -> f64 {
        fs.get_cash_assets(&self.id)
    }
    pub fn get_liabilities(&self, fs: &FinancialSystem) -> f64 {
        fs.get_total_liabilities(&self.id)
    } 
    pub fn get_deposits(&self, fs: &FinancialSystem) -> f64 {
        fs.get_deposits_at_bank(&self.id, &self.bank_id)
    }
}

impl Agent for Consumer {
    type DecisionType = ConsumerDecision;
    
    fn decide(&self, fs: &FinancialSystem, rng: &mut StdRng) -> Vec<ConsumerDecision> {
        self.decision_model.decide(self, fs, rng)
    }
    
    fn act(&self, decisions: &[ConsumerDecision]) -> Vec<SimAction> {
        let mut actions = Vec::new();
        /* 
        if self.income > 0.0 {
            actions.push(SimAction::IssueIncome {
                agent_id: self.id.clone(),
                amount: self.income,
            });
        }
        */
        for decision in decisions {
            match decision {
                ConsumerDecision::Save { agent_id, amount } => {
                    if *amount > 0.0 {
                        actions.push(SimAction::Deposit {
                            agent_id: agent_id.clone(),
                            bank: self.bank_id.clone(),
                            amount: amount.min(1000.0), // Cap at 1000 for now
                        });
                    }
                }
                ConsumerDecision::Spend { agent_id, seller_id, amount, good_id } => {
                    if *amount > 0.0 {
                        actions.push(SimAction::Purchase {
                            agent_id: agent_id.clone(),
                            seller: seller_id.clone(),
                            amount: *amount,
                            good_id: good_id.0.clone(),
                        });
                    }
                }
            }
        }
        
        actions
    }
}

impl FeatureSource for Consumer {
    fn get_age(&self) -> u32 {
        self.age
    }
    
    fn get_income(&self) -> f64 {
        self.income * 52.0 // Convert weekly back to annual for ML features
    }
    
    fn get_savings(&self) -> f64 {
        0.0
    }
    
    fn get_debt(&self) -> f64 {
        0.0
    }
}