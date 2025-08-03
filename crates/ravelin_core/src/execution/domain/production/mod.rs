use super::SerializableExecutionDomain;
use crate::validation::FirmValidator;
use crate::{EffectError, SimState, StateEffect, *};
use ravelin_traits::ExecutionResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct ProductionDomain {}

impl ProductionDomain {
    pub fn new() -> Self {
        Self {}
    }

    fn execute_hire(&self, firm_id: &AgentId, count: u32, state: &SimState) -> ExecutionResult<StateEffect> {
        if state.firms.iter().any(|f| f.id == *firm_id) {
            ExecutionResult {
                success: true,
                effects: vec![StateEffect::Hire { firm: *firm_id, count }],
                errors: vec![],
            }
        } else {
            ExecutionResult {
                success: false,
                effects: vec![],
                errors: vec![Box::new(EffectError::FirmNotFound { id: *firm_id })],
            }
        }
    }
    fn execute_produce(
        &self, firm_id: &AgentId, recipe_id: &RecipeId, batches: u32, state: &SimState,
    ) -> ExecutionResult<StateEffect> {
        if let Some(recipe) = state.financial_system.goods.recipes.get(recipe_id) {
            let mut effects = vec![];
            for _ in 0..batches {
                for (good_id, qty) in &recipe.inputs {
                    effects.push(StateEffect::RemoveInventory {
                        owner: *firm_id,
                        good_id: *good_id,
                        quantity: *qty,
                    });
                }
                let (output_good, output_qty) = &recipe.output;
                effects.push(StateEffect::AddInventory {
                    owner: *firm_id,
                    good_id: *output_good,
                    quantity: *output_qty * recipe.efficiency,
                    unit_cost: (recipe.labour_hours * 40.0 * 10.0) / recipe.efficiency,
                });
            }
            ExecutionResult { success: true, effects, errors: vec![] }
        } else {
            ExecutionResult {
                success: false,
                effects: vec![],
                errors: vec![Box::new(EffectError::RecipeError { id: *recipe_id })],
            }
        }
    }
}

impl_execution_domain! {
    ProductionDomain,
    "ProductionDomain",
    validate = |action, state| {
        let validator = FirmValidator::new(&state.financial_system);
        match action {
            SimAction::Produce { agent_id, recipe_id, batches } => {
                validator.validate_production(agent_id, recipe_id, *batches).is_ok()
            }
            SimAction::Hire { agent_id, count } => {
                let wage_rate = state.firms.iter().find(|f| f.id == *agent_id).map(|f| f.wage_rate).unwrap_or(20.0);
                validator.validate_hire(agent_id, *count, wage_rate).is_ok()
            }
            _ => true,
        }
    },
    execute = |self_domain, _action, state| {
        SimAction::Hire { agent_id: _agent_id, count: _count } => self_domain.execute_hire(_agent_id, *_count, state),
        SimAction::Produce { agent_id: _agent_id, recipe_id: _recipe_id, batches: _batches } => self_domain.execute_produce(_agent_id, _recipe_id, *_batches, state)
    }
}

#[typetag::serde]
impl SerializableExecutionDomain for ProductionDomain {
    fn clone_box_serializable(&self) -> Box<dyn SerializableExecutionDomain> {
        Box::new(self.clone())
    }
}