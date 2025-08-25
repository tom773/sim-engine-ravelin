// crates/domains/src/lib.rs - Updated Domain trait

pub use std::any::Any;
use sim_core::*;
extern crate inventory;

pub trait Domain: Send + Sync {
    fn name(&self) -> &'static str;
    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult;
    
    fn resolve_intention(&self, _intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        None
    }
    
    fn resolution_phase(&self, _intention: &SimIntention) -> Option<ResolutionPhase> {
        None
    }
    
    /// Optional method for settling trades. Only the TradingDomain implements this.
    fn settle_trade(&self, _trade: &Trade, _state: &SimState) -> DomainResult {
        DomainResult::failure(vec![format!("Trade settlement not supported by {}", self.name())])
    }
    
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug)]
pub struct DomainResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl DomainResult {
    pub fn success(effects: Vec<StateEffect>) -> Self {
        Self { success: true, effects, errors: vec![] }
    }
    
    pub fn failure(errors: Vec<String>) -> Self {
        Self { success: false, effects: vec![], errors }
    }
    
    pub fn empty() -> Self {
        Self { success: true, effects: vec![], errors: vec![] }
    }
}

#[derive(Debug, Clone)]
pub struct ResolutionContext<'a> {
    pub state: &'a SimState,
    pub current_tick: u32,
}

#[derive(Debug)]
pub struct ResolutionResult {
    pub actions: Vec<SimAction>,
    pub success: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResolutionPhase {
    Independent = 0,
    Market = 1,
    Dependent = 2,
}

impl ResolutionResult {
    pub fn success(actions: Vec<SimAction>) -> Self {
        Self { actions, success: true, errors: vec![] }
    }
    
    pub fn failure(errors: Vec<String>) -> Self {
        Self { actions: vec![], success: false, errors }
    }
    
    pub fn not_handled() -> Self {
        Self { actions: vec![], success: true, errors: vec![] }
    }
}

pub struct DomainValidator;

impl DomainValidator {
    pub fn positive_amount(amount: f64) -> Result<(), String> {
        if amount <= 0.0 {
            Err("Amount must be positive".to_string())
        } else {
            Ok(())
        }
    }
    
    pub fn non_negative_amount(amount: f64) -> Result<(), String> {
        if amount < 0.0 {
            Err("Amount cannot be negative".to_string())
        } else {
            Ok(())
        }
    }
    
    pub fn agent_exists(agent_id: AgentId, state: &SimState) -> Result<(), String> {
        if state.financial_system.balance_sheets.contains_key(&agent_id) {
            Ok(())
        } else {
            Err(format!("Agent {} does not exist", agent_id.0))
        }
    }
    
    pub fn bank_exists(bank_id: AgentId, state: &SimState) -> Result<(), String> {
        if state.agents.banks.contains_key(&bank_id) {
            Ok(())
        } else {
            Err("Target is not a valid commercial bank".to_string())
        }
    }
    
    pub fn firm_exists(firm_id: AgentId, state: &SimState) -> Result<(), String> {
        if state.agents.firms.contains_key(&firm_id) {
            Ok(())
        } else {
            Err("Target is not a valid firm".to_string())
        }
    }
    
    pub fn positive_integer(value: u32, field_name: &str) -> Result<(), String> {
        if value == 0 {
            Err(format!("{} must be positive", field_name))
        } else {
            Ok(())
        }
    }
    
    pub fn percentage(value: f64) -> Result<(), String> {
        if value < 0.0 || value > 1.0 {
            Err("Value must be between 0.0 and 1.0".to_string())
        } else {
            Ok(())
        }
    }
}

pub struct DomainRegistration {
    pub name: &'static str,
    pub constructor: fn() -> Box<dyn Domain>,
}

inventory::collect!(DomainRegistration);

pub mod banking;
pub mod consumption;
pub mod fiscal;
pub mod labour;
pub mod production;
pub mod settlement;
pub mod trading;

pub mod prelude {
    pub use crate::{
        Domain, DomainResult, DomainValidator, DomainRegistration,
        ResolutionContext, ResolutionResult, ResolutionPhase,
    };
    
    pub use crate::banking::{BankingDomain, BasicBankDecisionModel};
    pub use crate::consumption::{ConsumptionDomain, SimpleConsumerDecisionModel, CESConsumerDecisionModel};
    pub use crate::fiscal::{FiscalDomain, BasicGovernmentDecisionModel};
    pub use crate::labour::LabourDomain;
    pub use crate::production::{ProductionDomain, ProductionFirmDecisionModel, InvestmentFirmDecisionModel};
    pub use crate::settlement::SettlementDomain;
    pub use crate::trading::TradingDomain;
    
    pub use sim_core::*;
    pub use std::any::Any;
}