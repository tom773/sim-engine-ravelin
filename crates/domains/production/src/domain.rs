use sim_prelude::*;
use crate::{ProductionOperations, ProductionValidator};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionDomain {
    validator: ProductionValidator,
    operations: ProductionOperations,
}

#[derive(Debug, Clone)]
pub struct ProductionResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl ProductionDomain {
    pub fn new() -> Self {
        Self {
            validator: ProductionValidator::new(),
            operations: ProductionOperations::new(),
        }
    }

    pub fn can_handle(&self, action: &ProductionAction) -> bool {
        matches!(action, ProductionAction::Hire { .. } | ProductionAction::Produce { .. } | ProductionAction::PayWages { .. })
    }

    pub fn validate(&self, action: &ProductionAction, state: &SimState) -> Result<(), String> {
        self.validator.validate(action, state)
    }

    pub fn execute(&self, action: &ProductionAction, state: &SimState) -> ProductionResult {
        if let Err(error) = self.validate(action, state) {
            return ProductionResult { success: false, effects: vec![], errors: vec![error] };
        }

        match action {
            ProductionAction::Hire { agent_id, count } => self.operations.execute_hire(*agent_id, *count),
            ProductionAction::Produce { agent_id, recipe_id, batches } => {
                self.operations.execute_produce(*agent_id, *recipe_id, *batches, state)
            }
            ProductionAction::PayWages { .. } => {
                // Wage payments are financial transfers, handled by the BankingDomain.
                // This action is converted into a BankingAction by the engine.
                // Therefore, this domain produces no effects for it directly.
                ProductionResult { success: true, effects: vec![], errors: vec![] }
            }
        }
    }
}

impl Default for ProductionDomain {
    fn default() -> Self {
        Self::new()
    }
}