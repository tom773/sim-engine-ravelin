#![doc(hidden)]
pub use crate::{Domain, DomainResult, DomainValidator, DomainRegistration};

pub use crate::{ResolutionContext, ResolutionResult, ResolutionPhase};

pub use crate::banking::{BankingDomain, BasicBankDecisionModel};
pub use crate::consumption::{ConsumptionDomain, SimpleConsumerDecisionModel, CESConsumerDecisionModel};
pub use crate::fiscal::{FiscalDomain, BasicGovernmentDecisionModel};
pub use crate::labour::{LabourDomain};
pub use crate::production::{ProductionDomain, ProductionFirmDecisionModel, InvestmentFirmDecisionModel};
pub use crate::settlement::{SettlementDomain};
pub use crate::trading::{TradingDomain};

pub use sim_core::*;
pub use std::any::Any;

extern crate inventory;