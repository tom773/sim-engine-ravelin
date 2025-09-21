//! Instrument types combine immutable archetypes (contract templates) with runtime state.
//! Archetypes describe the long-lived terms, while builders and `inst_core` construct
//! `InstrumentCore<InstrumentRuntime>` values that flow through registries and catalogs.
pub mod archetypes;
pub mod builder;
pub mod credit;
pub mod inst_core;
pub mod inst_registry;
pub mod instrument;
pub mod issuance;
pub mod money;

pub use archetypes::*;
pub use builder::*;
pub use credit::*;
pub use inst_core::*;
pub use inst_registry::*;
pub use instrument::*;
pub use issuance::*;
pub use money::*;
