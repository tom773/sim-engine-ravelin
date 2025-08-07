//! # Consumption Domain Crate
//!
//! This crate contains all logic related to consumer agents. It governs how consumers
//! make decisions about what to buy and handles the mechanics of purchasing and
//! consuming goods.
//!
//! ## Crate Structure and Purpose
//!
//! Following the standard domain pattern, this crate separates action execution from
//! agent behavior.
//!
//! - **`domain.rs`**: Implements the `ConsumptionDomain` struct. This service validates
//!   and executes `ConsumptionAction`s, such as `Purchase` or `Consume`. It ensures
//!   a consumer has sufficient funds for a purchase and enough inventory to consume.
//!   Executing these actions generates the appropriate `StateEffect`s, like removing
//!   inventory or creating payment transfers.
//!
//! - **`behavior.rs`**: Implements decision models for consumer agents. This includes
//!   `BasicConsumerDecisionModel`, which uses simple heuristics to decide on purchases,
//!   and `ParametricMPC`, which uses a marginal propensity to consume (MPC) to determine
//    spending levels based on income.
//!
//! ## Key Components
//!
//! - **`ConsumptionDomain`**: The handler for executing consumer actions.
//! - **`BasicConsumerDecisionModel`**: The default "AI" for consumer agents.
//! - **`ConsumptionAction`**: The set of actions available to consumers (defined in `sim_actions`).
pub mod behavior;
pub mod domain;

pub use behavior::*;
pub use domain::*;