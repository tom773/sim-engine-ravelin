//! # Simulation Effects Crate (`sim_effects`)
//!
//! This crate defines the outcomes of actions. Where `sim_actions` represents what agents *want*
//! to do, `sim_effects` represents the concrete, validated changes that *will* be applied to
//! the simulation's state. It forms the second half of the simulation's core action/effect pipeline.
//!
//! ## Crate Structure and Purpose
//!
//! The crate aggregates all possible state mutations into a single `StateEffect` enum. This ensures
//! that all changes to the `SimState` are explicit, typed, and auditable. The most critical
// //! part of this crate is the `EffectApplicator`, which provides the logic for safely applying
//! these effects.
//!
//! This decouples the *declaration* of a state change from its *implementation*, which is a key
//! architectural pattern for keeping the simulation state consistent.
//!
//! The crate is structured into domain-specific effect modules and a central application module:
//!
//! - **`agent.rs`**: Effects that directly modify agent properties (e.g., hiring, changing income).
//! - **`financial.rs`**: Effects on financial instruments (e.g., creating/updating instruments, accruing interest).
//! - **`inventory.rs`**: Effects that change an agent's inventory of goods.
//! - **`market.rs`**: Effects on market order books (e.g., placing orders, executing trades).
//! - **`application.rs`**: Contains the `EffectApplicator` trait and `StateEffectApplicator` struct,
//!   which hold the logic for applying effects to the `SimState`.
//!
//! ## Key Components
//!
//! - **`StateEffect` (enum)**: The top-level enum wrapping all possible state mutations. It's the
//!   output of a domain's `execute` method.
//!
//! - **`EffectApplicator` (trait)**: A trait implemented by `SimState` that defines the `apply_effect`
//!   method. This is the public API for modifying the state.
//!
//! - **`StateEffectApplicator` (struct)**: A helper struct that contains the "dirty" work of
//!   pattern matching on a `StateEffect` and mutating the `SimState` accordingly. This keeps the
//!   complex matching logic separate from the state struct itself.
pub mod agent;
pub mod application;
pub mod financial;
pub mod inventory;
pub mod market;

pub use agent::*;
pub use application::*;
pub use financial::*;
pub use inventory::*;
pub use market::*;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StateEffect {
    Financial(FinancialEffect),
    Inventory(InventoryEffect),
    Market(MarketEffect),
    Agent(AgentEffect),
}

impl StateEffect {
    /// Returns a string representing the fully qualified name of the effect variant.
    pub fn name(&self) -> String {
        match self {
            StateEffect::Financial(effect) => format!("Financial::{}", effect.name()),
            StateEffect::Inventory(effect) => format!("Inventory::{}", effect.name()),
            StateEffect::Market(effect) => format!("Market::{}", effect.name()),
            StateEffect::Agent(effect) => format!("Agent::{}", effect.name()),
        }
    }
}