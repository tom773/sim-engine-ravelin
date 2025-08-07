//! # Domains Prelude Crate (`prelude`)
//!
//! A convenience crate for the simulation engine and other high-level components.
//!
//! ## Crate Structure and Purpose
//!
//! This crate follows the "prelude" pattern. Its purpose is to re-export the primary
//! public types from all of the individual `domains` crates.
//!
//! Specifically, it exports the main "domain handler" structs (e.g., `BankingDomain`,
//! `ProductionDomain`, `ConsumptionDomain`, etc.). This allows the `DomainRegistry` in the
//! `engine` crate to easily access all domain handlers with a single `use` statement.
//!
//! This crate contains no original logic and consists only of `pub use` statements.
pub use crate::banking::{
    BankingDomain, BasicBankDecisionModel, BankingResult,
};
pub use crate::consumption::{
    ConsumptionDomain, BasicConsumerDecisionModel,
};
pub use crate::fiscal::{
    FiscalDomain, BasicGovernmentDecisionModel,
};
pub use crate::production::{
    ProductionDomain, BasicFirmDecisionModel,
};
pub use crate::settlement::{
    SettlementDomain
};
pub use crate::trading::{
    TradingDomain
};