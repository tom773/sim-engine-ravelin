//! # Simulation Actions Crate (`sim_actions`)
//!
//! This crate defines the fundamental "verbs" of the simulation. It establishes a unified
//! system for representing all possible actions that agents can decide to take within the
//! economic model.
//!
//! ## Crate Structure and Purpose
//!
//! The primary purpose of this crate is to aggregate all domain-specific actions into a single,
//! top-level enum, `SimAction`. This design allows the simulation engine to handle actions
//! generically during the decision-making phase, before dispatching them to the appropriate
//! domain for execution.
//!
//! The crate is structured into several modules, each corresponding to a specific economic domain:
//!
//! - **`banking.rs`**: Defines `BankingAction` for deposits, withdrawals, and transfers.
//! - **`consumption.rs`**: Defines `ConsumptionAction` for purchasing and consuming goods.
//! - **`fiscal.rs`**: Defines `FiscalAction` for government-related activities like taxation.
//! - **`production.rs`**: Defines `ProductionAction` for firm activities like producing goods and hiring.
//! - **`settlement.rs`**: Defines `SettlementAction` for financial processes like paying interest.
//! - **`trading.rs`**: Defines `TradingAction` for market activities like placing bids and asks.
//! - **`validation.rs`**: Provides helper functions for validating action parameters (e.g., ensuring amounts are positive).
//!
//! ## Key Components
//!
//! - **`SimAction` (enum)**: This is the core enum of the crate. It wraps all the domain-specific action enums.
//!   An instance of `SimAction` represents a single, concrete intention from an agent.
//!
//! - **`agent_id()` (method)**: A common method on all action types that returns the `AgentId` of the
//!   agent who initiated the action. This is crucial for logging, validation, and execution routing.

pub mod banking;
pub mod consumption;
pub mod fiscal;
pub mod production;
pub mod settlement;
pub mod trading;
pub mod validation;

pub use banking::*;
pub use consumption::*;
pub use fiscal::*;
pub use production::*;
pub use settlement::*;
pub use trading::*;
pub use validation::*;

use serde::{Deserialize, Serialize};
use crate::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimAction {
    Banking(BankingAction),
    Consumption(ConsumptionAction),
    Fiscal(FiscalAction),
    Production(ProductionAction),
    Settlement(SettlementAction),
    Trading(TradingAction),
}

impl SimAction {
    /// Returns a string slice representing the name of the action variant.
    pub fn name(&self) -> String {
        match self {
            SimAction::Banking(action) => format!("Banking::{}", action.name()),
            SimAction::Consumption(action) => format!("Consumption::{}", action.name()),
            SimAction::Fiscal(action) => format!("Fiscal::{}", action.name()),
            SimAction::Production(action) => format!("Production::{}", action.name()),
            SimAction::Settlement(action) => format!("Settlement::{}", action.name()),
            SimAction::Trading(action) => format!("Trading::{}", action.name()),
        }
    }

    /// Returns the `AgentId` of the agent performing the action.
    pub fn agent_id(&self) -> AgentId {
        match self {
            SimAction::Banking(action) => action.agent_id(),
            SimAction::Consumption(action) => action.agent_id(),
            SimAction::Fiscal(action) => action.agent_id(),
            SimAction::Production(action) => action.agent_id(),
            SimAction::Settlement(action) => action.agent_id(),
            SimAction::Trading(action) => action.agent_id(),
        }
    }
}