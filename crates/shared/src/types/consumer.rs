use crate::*;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use rand::prelude::*;
use crate::FeatureSource;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Consumer {
    pub id: AgentId,
    pub age: u32,
    pub bank_id: AgentId,
    pub decision_model: Box<dyn DecisionModel>,
    pub propensity_to_consume: f64,
    pub income: f64,
}

impl Consumer {
    pub fn new(age: u32, id: AgentId, bank_id: AgentId, dm: Box<dyn DecisionModel>) -> Self {
        let mut rng = rand::rng();
        Self {
            id,
            bank_id,
            age,
            decision_model: dm,
            propensity_to_consume: rng.random_range(0.3..0.7),
            income: rng.random_range(30000.0..80000.0),
        }
    }
    
    pub fn save(&mut self, amount: f64, fs: &mut FinancialSystem) -> Result<(), String> {
        if amount > self.income {
            return Err("Insufficient income to save".to_string());
        }
        
        let deposit = FinancialInstrument {
            id: InstrumentId(Uuid::new_v4()),
            creditor: self.id.clone(),
            debtor: self.bank_id.clone(),
            principal: amount,
            maturity: None,
            interest_rate: fs.central_bank.policy_rate + fs.commercial_banks[&self.bank_id].deposit_spread,
            instrument_type: InstrumentType::DemandDeposit,
            originated_date: 0,
        };
        
        fs.create_instrument(deposit)?;
        Ok(())
    }
    
    pub fn balance_sheet<'a>(&self, fs: &'a FinancialSystem) -> Option<&'a BalanceSheet> {
        fs.balance_sheets.get(&self.id)
    }
}

impl Agent for Consumer {
    fn act(&self, decision: &Decision) -> Action {
        match decision {
            Decision::Spend { amount } => Action::Buy { 
                good_id: "goods".to_string(), 
                quantity: (*amount / 10.0) as u32 
            },
            Decision::Save => Action::Save,
        }
    }
    
    fn decide(&self, fs: &FinancialSystem, rng: &mut StdRng) -> Decision {
        self.decision_model.decide(self, fs, rng)
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
        // TODO
        0.0
    }
    
    fn get_debt(&self) -> f64 {
        // TODO
        0.0
    }
}