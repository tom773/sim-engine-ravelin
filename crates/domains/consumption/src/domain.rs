use sim_prelude::*;
use crate::{ConsumptionOperations, ConsumptionValidator};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumptionDomain {
    validator: ConsumptionValidator,
    operations: ConsumptionOperations,
}

#[derive(Debug, Clone)]
pub struct ConsumptionResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl ConsumptionDomain {
    pub fn new() -> Self {
        Self {
            validator: ConsumptionValidator::new(),
            operations: ConsumptionOperations::new(),
        }
    }

    pub fn can_handle(&self, action: &ConsumptionAction) -> bool {
        matches!(action, ConsumptionAction::Purchase { .. } | ConsumptionAction::Consume { .. })
    }

    pub fn validate(&self, action: &ConsumptionAction, state: &SimState) -> Result<(), String> {
        self.validator.validate(action, state)
    }

    pub fn execute(&self, action: &ConsumptionAction, state: &SimState) -> ConsumptionResult {
        if let Err(error) = self.validate(action, state) {
            return ConsumptionResult { success: false, effects: vec![], errors: vec![error] };
        }

        match action {
            ConsumptionAction::Purchase { agent_id, seller, good_id, amount } => {
                self.operations.execute_purchase(*agent_id, *seller, *good_id, *amount, state)
            }
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                self.operations.execute_consume(*agent_id, *good_id, *amount)
            }
        }
    }
}

impl Default for ConsumptionDomain {
    fn default() -> Self {
        Self::new()
    }
}