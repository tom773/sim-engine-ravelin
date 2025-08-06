use crate::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bank {
    pub id: AgentId,
    pub name: String,
    pub lending_spread: f64,
    pub deposit_spread: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Consumer {
    pub id: AgentId,
    pub age: u32,
    pub bank_id: AgentId,
    pub income: f64,
    pub personality: PersonalityArchetype,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Firm {
    pub id: AgentId,
    pub bank_id: AgentId,
    pub name: String,
    pub employees: Vec<AgentId>,
    pub wage_rate: f64,
    pub productivity: f64,
    pub recipe: Option<RecipeId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Government {
    pub id: AgentId,
    pub tax_rates: TaxRates,
    pub spending_targets: SpendingTargets,
    pub debt_ceiling: Option<f64>,
    pub fiscal_policy: FiscalPolicy,
}

impl Bank {
    pub fn new(name: String, lending_spread: f64, deposit_spread: f64) -> Self {
        Self { 
            id: AgentId(uuid::Uuid::new_v4()), 
            name, 
            lending_spread, 
            deposit_spread, 
        }
    }
}

impl Consumer {
    pub fn new(age: u32, bank_id: AgentId, personality: PersonalityArchetype) -> Self {
        Self { 
            id: AgentId(uuid::Uuid::new_v4()), 
            age, 
            bank_id, 
            income: 0.0, 
            personality, 
        }
    }
}

impl Firm {
    pub fn new(bank_id: AgentId, name: String, recipe: Option<RecipeId>) -> Self {
        Self {
            id: AgentId(uuid::Uuid::new_v4()),
            bank_id,
            name,
            employees: Vec::new(),
            wage_rate: 25.0,
            productivity: 1.0,
            recipe,
        }
    }
    pub fn get_employees(&self) -> &Vec<AgentId> {
        &self.employees
    }
    // TODO: Market needs to store transaction details 
    pub fn calculate_profits(&self, revenues: f64, costs: f64) -> FirmProfits {
        let gross_profit = revenues - costs;
        let tax_liability = gross_profit * 0.21;
        let net_profit = gross_profit - tax_liability;
        
        FirmProfits {
            gross: gross_profit,
            tax: tax_liability,
            net: net_profit,
            retained_earnings_ratio: 0.6,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FirmProfits {
    pub gross: f64,
    pub tax: f64,
    pub net: f64,
    pub retained_earnings_ratio: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CentralBank {
    pub id: AgentId,
    pub policy_rate: f64,
    pub reserve_requirement: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PersonalityArchetype {
    Balanced,
    Spender,
    Saver,
}

impl Government {
    pub fn new(tax_rates: TaxRates, spending_targets: SpendingTargets, fiscal_policy: FiscalPolicy) -> Self {
        Self {
            id: AgentId(uuid::Uuid::new_v4()),
            tax_rates,
            spending_targets,
            debt_ceiling: None,
            fiscal_policy,
        }
    }
    pub fn get_id(&self) -> &AgentId {
        &self.id
    }
}
impl Default for Government {
    fn default() -> Self {
        Self {
            id: AgentId(uuid::Uuid::new_v4()),
            tax_rates: TaxRates::default(),
            spending_targets: SpendingTargets::default(),
            debt_ceiling: None,
            fiscal_policy: FiscalPolicy::default(),
        }
    }
}
