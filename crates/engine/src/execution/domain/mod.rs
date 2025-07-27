pub mod banking;
pub mod production;
pub mod trading;

use crate::execution::effects::ExecutionResult;
use crate::state::SimState;
use shared::*;
use std::collections::HashMap;

pub trait ExecutionDomain: Send + Sync {
    fn name(&self) -> &'static str;
    fn can_handle(&self, action: &SimAction) -> bool;
    fn validate(&self, action: &SimAction, state: &SimState) -> bool;
    fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult;
}

pub struct DomainRegistry {
    domains: Vec<Box<dyn ExecutionDomain>>,
}

impl DomainRegistry {
    pub fn new() -> Self {
        Self {
            domains: vec![
                Box::new(banking::BankingDomain::new()),
                Box::new(trading::TradingDomain::new()),
                Box::new(production::ProductionDomain::new()),
            ],
        }
    }

    pub fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        for domain in &self.domains {
            if domain.can_handle(action) {
                if domain.validate(action, state) {
                    return domain.execute(action, state);
                } else {
                    return ExecutionResult {
                        success: false,
                        effects: vec![],
                        errors: vec![format!("Validation failed for action: {}", action.name())],
                    };
                }
                return domain.execute(action, state);
            }
        }

        ExecutionResult {
            success: false,
            effects: vec![],
            errors: vec![format!("No domain can handle action: {}", action.name())],
        }
    }
}
