pub use std::any::Any;
use serde::Serialize;
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

#[derive(Debug, Clone, Serialize)]
pub struct ResolutionContext<'a> {
    pub state: &'a SimState,
    pub current_tick: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct ResolutionResult {
    pub actions: Vec<SimAction>,
    pub success: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
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

pub struct DomainRegistration {
    pub name: &'static str,
    pub constructor: fn() -> Box<dyn Domain>,
}

inventory::collect!(DomainRegistration);

pub mod banking_domain;
pub mod consumption_domain;
pub mod fiscal_domain;
pub mod production_domain;
pub mod monetary_domain;
pub mod transaction_domain;

pub mod prelude {
    pub use crate::{
        Domain, DomainResult,DomainRegistration,
        ResolutionContext, ResolutionResult, ResolutionPhase,
    };
    
    pub use crate::banking_domain::{behaviour::*, domain::*};
    pub use crate::consumption_domain::{behaviour::*, domain::*};
    pub use crate::fiscal_domain::{behaviour::*, domain::*};
    pub use crate::monetary_domain::{behaviour::*, domain::*};
    pub use crate::production_domain::{behaviour::*, domain::*};
    pub use crate::transaction_domain::{domain::*};

    pub use sim_core::*;
    pub use std::any::Any;
}