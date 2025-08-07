//! # Production Domain Crate
//!
//! This crate encapsulates all logic related to producer agents (firms). It governs
//! how firms make decisions about production levels, hiring, and resource management,
//! and it handles the execution of these production-related actions.
//!
//! ## Crate Structure and Purpose
//!
//! The crate is divided into action execution and agent behavior modules:
//!
//! - **`domain.rs`**: Implements the `ProductionDomain` struct. This service validates
//!   and executes `ProductionAction`s, such as `Produce` and `Hire`. For a `Produce`
//!   action, it validates that the firm has the necessary input goods and labor, as
//!   defined by its `ProductionRecipe`. If valid, it generates `StateEffect`s to consume
//!   the inputs and add the finished product to the firm's inventory.
//!
//! - **`behavior.rs`**: Implements the `BasicFirmDecisionModel`. This is the "AI" for
//!   firm agents. It analyzes market conditions and its own inventory levels to decide
//!   whether to increase production, hire more employees, or purchase more raw materials.
//!
//! ## Key Components
//!
//! - **`ProductionDomain`**: The handler for executing firm-specific actions.
//! - **`BasicFirmDecisionModel`**: The default logic controller for firm agents.
//! - **`ProductionAction`**: The set of actions available to firms (defined in `sim_actions`).
//! - **`ProductionRecipe`**: A data structure from `sim_types` that defines the inputs,
//!   outputs, and labor required for a production process.
pub mod behavior;
pub mod domain;
pub use domain::*;
pub use behavior::*;
