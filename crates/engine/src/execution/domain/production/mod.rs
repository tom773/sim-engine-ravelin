use crate::{
    domain::{SerializableExecutionDomain, ExecutionDomain},
    effects::{ExecutionResult, EffectError, StateEffect},
    state::SimState,
};
use shared::*;
use serde::{Deserialize, Serialize};

pub struct ProductionDomainImpl {}

impl ProductionDomainImpl {
    pub fn new() -> Self {
        ProductionDomainImpl {}
    }
    
    // Main execute method that dispatches
    pub fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        match action {
            SimAction::Produce { .. } => self.execute_produce(action, state),
            SimAction::Hire { .. } => self.execute_hire(action, state),
            _ => ExecutionResult {
                success: false,
                effects: vec![],
                errors: vec![EffectError::InvalidState("Action not handled by ProductionDomain".to_string())],
            },
        }
    }
    
    pub fn execute_hire(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        if let SimAction::Hire { agent_id, count } = action {
            if let Some(f) = state.firms.iter().find(|f| f.id == *agent_id) {
                return ExecutionResult {
                    success: true,
                    effects: vec![StateEffect::Hire { firm: f.id.clone(), count: *count }],
                    errors: vec![],
                };
            } else {
                return ExecutionResult {
                    success: false,
                    effects: vec![],
                    errors: vec![EffectError::FirmNotFound { id: agent_id.clone() }],
                };
            }
        }
        ExecutionResult { success: true, effects: vec![], errors: vec![] }
    }
    
    pub fn execute_produce(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        if let SimAction::Produce { agent_id, recipe_id, batches } = action {
            if let Some(recipe) = state.financial_system.goods.recipes.get(recipe_id) {
                let mut effects = vec![];
                for _ in 0..*batches {
                    for (good_id, qty) in &recipe.inputs {
                        effects.push(StateEffect::RemoveInventory {
                            owner: agent_id.clone(),
                            good_id: *good_id,
                            quantity: *qty,
                        });
                    }
                    let (output_good, output_qty) = &recipe.output;
                    effects.push(StateEffect::AddInventory {
                        owner: agent_id.clone(),
                        good_id: *output_good,
                        quantity: *output_qty * recipe.efficiency,
                        unit_cost: (recipe.labour_hours * 40.0 * 10.0) / recipe.efficiency,
                    });
                }
                return ExecutionResult { success: true, effects, errors: vec![] };
            } else {
                return ExecutionResult {
                    success: false,
                    effects: vec![],
                    errors: vec![EffectError::RecipieError { id: *recipe_id }],
                };
            }
        }
        ExecutionResult { success: false, effects: vec![], errors: vec![EffectError::InvalidState("Produce Failed".to_string())] }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProductionDomain {
}

impl ProductionDomain {
    pub fn new() -> Self {
        Self {
        }
    }
}

impl ExecutionDomain for ProductionDomain {
    fn name(&self) -> &'static str {
        "ProductionDomain"
    }
    
    fn can_handle(&self, action: &SimAction) -> bool {
        matches!(action, SimAction::Produce { .. } | SimAction::Hire { .. })
    }
    
    fn validate(&self, action: &SimAction, state: &SimState) -> bool {
        match action {
            SimAction::Produce { recipe_id, .. } => {
                state.financial_system.goods.recipes.contains_key(recipe_id)
            }
            SimAction::Hire { .. } => true,
            _ => false
        }
    }
    
    fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        let impl_domain = ProductionDomainImpl::new();
        impl_domain.execute(action, state)
    }
    
    fn clone_box(&self) -> Box<dyn SerializableExecutionDomain> {
        Box::new(self.clone())
    }
}

#[typetag::serde]
impl SerializableExecutionDomain for ProductionDomain {}