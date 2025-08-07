//! # Simulation Prelude Crate (`sim_prelude`)
//!
//! A convenience crate for the entire simulation workspace.
//!
//! ## Crate Structure and Purpose
//!
//! This crate follows the "prelude" pattern common in the Rust ecosystem. Its sole purpose is to
//! re-export the most commonly used public types, traits, and functions from all other `core`
//! and `domains` crates.
//!
//! By importing `sim_prelude::*`, other crates can gain easy access to essential components like
//! `SimAction`, `StateEffect`, `DecisionModel`, `SimState`, `AgentId`, all agent types, and all
//! domain handlers without needing a long list of `use` statements. This simplifies development
//! and makes the code cleaner.
//!
//! This crate has no logic of its own; it consists entirely of `pub use` statements.
pub use sim_actions::*;
pub use sim_decisions::*;
pub use sim_effects::*;
pub use sim_types::*;