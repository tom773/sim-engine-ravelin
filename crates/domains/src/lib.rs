use serde::{Deserialize, Serialize};
use sim_core::{SimAction, SimState, StateEffect};
use std::any::Any;

pub trait Domain: Send + Sync {
    fn name(&self) -> &'static str;

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult;

    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

pub struct DomainRegistration {
    pub name: &'static str,
    pub constructor: fn() -> Box<dyn Domain>,
}

inventory::collect!(DomainRegistration);

pub mod banking;
pub mod consumption;
pub mod fiscal;
pub mod prelude;
pub mod production;
pub mod settlement;
pub mod trading;