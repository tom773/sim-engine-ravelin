//! # Settlement Domain Crate
//!
//! This crate is responsible for the financial settlement processes that occur
//! periodically within the simulation. It handles "background" financial mechanics
//! that are not typically initiated by direct agent decisions, such as interest
//! accrual and coupon payments.
//!
//! ## Crate Structure and Purpose
//!
//! Unlike other domains, the settlement domain primarily consists of a `domain.rs` module.
//! There is no `behavior.rs` because settlement actions are usually triggered by the
//! simulation engine's clock rather than by an agent's `DecisionModel`.
//!
//! - **`domain.rs`**: Contains the `SettlementDomain` struct. This service validates
//!   and executes `SettlementAction`s.
//!   - **`AccrueInterest`**: Calculates and records the interest that has accrued on an
//!     instrument since the last calculation.
//!   - **`PayInterest`**: Creates the financial transaction to move accrued interest from
//!     the debtor to the creditor.
//!   - **`ProcessCouponPayment`**: Handles the fixed payments for bond instruments.
//!
//! The `SettlementDomain` translates these financial events into concrete `StateEffect`s,
//! ensuring that the simulation's financial plumbing works correctly over time.
//!
//! ## Key Components
//!
//! - **`SettlementDomain`**: The primary handler for executing all settlement-related tasks.
//! - **`SettlementAction`**: The set of financial settlement actions available, such as
//!   `AccrueInterest` (defined in `sim_actions`).
//! - **`SettlementResult`**: A struct wrapping the outcome, containing effects or errors.
pub mod domain;
pub use domain::*;