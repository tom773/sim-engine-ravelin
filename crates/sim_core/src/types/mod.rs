//! # Simulation Types Crate (`sim_types`)
//!
//! This is the foundational data model crate for the entire simulation. It defines all the
//! core "nouns"—the data structures, enums, and IDs that represent the state of the
//! economic world.
//!
//! ## Crate Structure and Purpose
//!
//! This crate provides the building blocks for the simulation's state. It is deliberately
//! designed to contain only data definitions and associated helper methods, with no complex
//! business logic. All decision-making and state-transition logic is handled in the `sim_decisions`,
//! `sim_actions`, `sim_effects`, and `domains` crates.
//!
//! The crate is organized into modules representing distinct concepts:
//!
//! - **`state.rs`**: Defines `SimState`, the top-level container for all simulation data.
//! - **`system.rs`**: Defines `FinancialSystem`, a major component of the state that holds all
//!   financial instruments, balance sheets, and market data.
//! - **`agents.rs`**: Defines the primary agent types: `Bank`, `Consumer`, `Firm`, `Government`,
//!   and `CentralBank`.
//! - **`balance_sheet.rs`**: Defines the `BalanceSheet` structure, which tracks an agent's assets
//!   and liabilities.
//! - **`instruments.rs`**: Defines `FinancialInstrument` and its various concrete types (e.g.,
//!   `CashDetails`, `BondDetails`, `LoanDetails`).
//! - **`goods.rs`**: Defines goods, inventories, and production recipes (`ProductionRecipe`), loading
//!   them from a TOML configuration.
//! - **`markets.rs`**: Defines market structures like `Exchange`, `OrderBook`, `Trade`, `Bid`, and `Ask`.
//! - **`ids.rs`**: Defines strongly-typed unique identifiers used throughout the simulation (e.g.,
//!   `AgentId`, `InstrumentId`).
//! - **`macros.rs`**: Contains convenience macros for creating financial instruments (e.g., `cash!`, `deposit!`).
//! - **`traits.rs`**: Defines core traits for interacting with the financial system, like `InstrumentManager`.
//! - **`policy.rs`**: Defines structures related to fiscal and monetary policy.
//! - **`time.rs`**: Provides time and date-related utility functions.
//!
pub mod agents;
pub mod balance_sheet;
pub mod goods;
pub mod ids;
pub mod instruments;
pub mod macros;
pub mod markets;
pub mod policy;
pub mod state;
pub mod system;
pub mod time;
pub mod traits;

pub use agents::*;
pub use balance_sheet::*;
pub use goods::*;
pub use ids::*;
pub use instruments::*;
pub use markets::*;
pub use policy::*;
pub use state::*;
pub use system::*;
pub use time::*;
pub use traits::*;