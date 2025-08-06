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