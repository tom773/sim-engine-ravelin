use shared::*; 
use crate::{state::SimState, effects::ExecutionResult, domain::ExecutionDomain};

pub struct ProductionDomain {
    // Add any necessary fields here
}

impl ProductionDomain {
    pub fn new() -> Self {
        ProductionDomain {
        }
    }
    pub fn execute_hire(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        //TODO
        ExecutionResult {
            success: true,
            effects: vec![],
            errors: vec![],
        }
    }
    pub fn execute_produce(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        //TODO
        ExecutionResult {
            success: true,
            effects: vec![],
            errors: vec![],
        }
    }
}

impl ExecutionDomain for ProductionDomain {
    fn name(&self) -> &'static str {
        "ProductionDomain"
    }
    
    fn can_handle(&self, action: &SimAction) -> bool {
        matches!(
            action,
            SimAction::Produce { .. }
                | SimAction::Hire { .. }
        )
    }
    fn validate(&self, action: &SimAction, state: &SimState) -> bool {
        matches!(
            action,
            SimAction::Produce { .. }
                | SimAction::Hire { .. }
        )
    }
    fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        match action {
            SimAction::Produce { .. } => { self.execute_produce(action, state) },
            SimAction::Hire { .. } => { self.execute_hire(action, state) },
            _ => {
                ExecutionResult {
                    success: true,
                    effects: vec![],
                    errors: vec![],
                }
            }
        }
    }
}