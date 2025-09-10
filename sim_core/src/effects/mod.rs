pub mod agent_effects;
pub mod application;
pub mod financial;
pub mod inventory;
pub mod market_effects;
pub mod rtgs;
pub mod credit_effects;

pub use credit_effects::*;
pub use rtgs::*;
pub use agent_effects::*;
pub use application::*;
pub use financial::*;
pub use inventory::*;
pub use market_effects::*;
pub mod monetary_effects;
pub use monetary_effects::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StateEffect {
    Financial(FinancialEffect),
    Inventory(InventoryEffect),
    Market(MarketEffect),
    Agent(AgentEffect),
    Monetary(MonetaryEffect),
    Credit(CreditEffect),
}

impl StateEffect {
    pub fn name(&self) -> String {
        match self {
            StateEffect::Financial(effect) => format!("Financial::{}", effect.name()),
            StateEffect::Inventory(effect) => format!("Inventory::{}", effect.name()),
            StateEffect::Market(effect) => format!("Market::{}", effect.name()),
            StateEffect::Agent(effect) => format!("Agent::{}", effect.name()),
            StateEffect::Monetary(effect) => format!("CentralBank::{}", effect.name()),
            StateEffect::Credit(effect) => format!("Credit::{}", effect.name()),
        }
    }
}