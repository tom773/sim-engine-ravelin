//! # Simulation Engine Crate (`engine`)
//!
//! This is the main executable crate that orchestrates the entire simulation. It brings together
//! the data types (`sim_types`), agent behaviors (`sim_decisions`), and domain logic (`domains`)
//! to run the economic model over time.
//!
//! ## Crate Structure and Purpose
//!
//! The engine is responsible for the main simulation loop. It loads a scenario, initializes the
//! state, and then repeatedly executes "ticks" until the simulation is complete.
//!
//! The crate is organized into several key modules:
//!
//! - **`executor.rs`**: Contains `SimulationEngine`, the core struct that holds the simulation state
//!   and manages the main `tick()` loop. This is where the magic happens.
//!
//! - **`tick()` method**: The heart of the simulation. In each tick, it performs the following steps:
//!   1.  **Process Financial Updates**: Handles time-based events like interest accrual.
//!   2.  **Collect Actions**: Queries the `DecisionModel` of every agent to gather their desired `SimAction`s.
//!   3.  **Execute Actions**: Passes the collected actions to the `DomainRegistry` to be validated and executed, producing a set of `StateEffect`s.
//!   4.  **Apply Effects**: Applies the generated effects to the `SimState`.
//!   5.  **Clear Markets**: Calls the `Exchange` to match bids and asks, generating `Trade`s.
//!   6.  **Settle Trades**: Passes the `Trade`s to the `TradingDomain` to generate settlement effects.
//!   7.  **Apply Settlement Effects**: Applies the final settlement effects.
//!   8.  **Advance Time**: Increments the simulation clock.
//!
//! - **`registry.rs`**: Defines `DomainRegistry`, a struct that holds an instance of every
//!   domain handler (e.g., `BankingDomain`, `ProductionDomain`). It acts as a router, dispatching
//!   each `SimAction` to the correct domain for execution.
//!
//! - **`scenario.rs`**: Provides the logic for parsing a TOML configuration file (`config.toml`)
//!   into a `Scenario` struct. This defines the initial conditions of the simulation.
//!
//! - **`factory.rs`**: Implements `AgentFactory`, a helper for populating the initial `SimState`
//!   based on a `Scenario`. It creates all the agents (banks, firms, consumers) and their initial
//!   balance sheets and assets.
//!
//! - **`cli/` (directory)**: Contains a simple binary that uses NATS messaging to provide a remote
//!   interface for controlling the simulation engine (e.g., initializing, ticking, querying state).
pub mod executor;
pub mod factory;
pub mod registry;
pub mod scenario;

pub use executor::*;
pub use factory::*;
pub use registry::*;
pub use scenario::*;