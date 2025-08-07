//! # Fiscal Domain Crate
//!
//! This crate manages all logic related to government agents and fiscal policy. It
//! is responsible for executing fiscal actions and defining the decision-making
//! processes of the government entity.
//!
//! ## Crate Structure and Purpose
//!
//! The crate is organized into the standard domain/behavior modules:
//!
//! - **`domain.rs`**: Contains the `FiscalDomain` struct. This module handles the
//!   execution of `FiscalAction`s, such as collecting taxes or distributing
//!   transfer payments. It translates these high-level policy actions into concrete
//!   `StateEffect`s that modify agent balance sheets.
//!
//! - **`behaviour.rs`**: Contains the `BasicGovernmentDecisionModel`. This model implements
//!   the `DecisionModel` trait and defines how the government agent behaves. For example,
//!   it may decide to issue new bonds to fund a deficit based on its defined `FiscalPolicy`
//!   (e.g., `Expansionary`, `Contractionary`).
//!
//! ## Key Components
//!
//! - **`FiscalDomain`**: The service for executing government-level actions.
//! - **`BasicGovernmentDecisionModel`**: The logic controller for the government agent.
//! - **`FiscalAction`**: The set of actions available to the government (defined in `sim_actions`).
//! - **`FiscalPolicy`**: An enum from `sim_types` that guides the government's decision-making.
pub mod behaviour;
pub mod domain;

pub use behaviour::*;
pub use domain::*;