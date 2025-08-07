//! # Domains Prelude Crate (`domains_prelude`)
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
pub use domains_banking::{
    BankingDomain, BasicBankDecisionModel, BankingResult,
};
pub use domains_consumption::{
    ConsumptionDomain, BasicConsumerDecisionModel,
};
pub use domains_fiscal::{
    FiscalDomain, BasicGovernmentDecisionModel,
};
pub use domains_production::{
    ProductionDomain, BasicFirmDecisionModel,
};
pub use domains_settlement::{
    SettlementDomain
};
pub use domains_trading::{
    TradingDomain
};