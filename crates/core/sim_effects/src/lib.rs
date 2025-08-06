pub mod financial;
pub mod inventory;  
pub mod market;
pub mod agent;
pub mod application;

pub use financial::*;
pub use inventory::*;
pub use market::*;
pub use agent::*;
pub use application::*;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StateEffect {
    Financial(FinancialEffect),
    Inventory(InventoryEffect),
    Market(MarketEffect),
    Agent(AgentEffect),
}

impl StateEffect {
    pub fn name(&self) -> String {
        match self {
            StateEffect::Financial(effect) => format!("Financial::{}", effect.name()),
            StateEffect::Inventory(effect) => format!("Inventory::{}", effect.name()),
            StateEffect::Market(effect) => format!("Market::{}", effect.name()),
            StateEffect::Agent(effect) => format!("Agent::{}", effect.name()),
        }
    }
}