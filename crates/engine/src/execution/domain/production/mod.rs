use crate::{
    domain::{ExecutionDomain, SerializableExecutionDomain},
    effects::{EffectError, ExecutionResult, StateEffect},
    state::SimState,
};
use serde::{Deserialize, Serialize};
use shared::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct ProductionDomain {}

impl ProductionDomain {
    pub fn new() -> Self {
        Self {}
    }

    fn execute_hire(&self, firm_id: &AgentId, count: u32, state: &SimState) -> ExecutionResult {
        if state.firms.iter().any(|f| f.id == *firm_id) {
            ExecutionResult {
                success: true,
                effects: vec![StateEffect::Hire { firm: firm_id.clone(), count }],
                errors: vec![],
            }
        } else {
            ExecutionResult {
                success: false,
                effects: vec![],
                errors: vec![EffectError::FirmNotFound { id: firm_id.clone() }],
            }
        }
    }
    fn execute_produce(
        &self, firm_id: &AgentId, recipe_id: &RecipeId, batches: u32, state: &SimState,
    ) -> ExecutionResult {
        if let Some(recipe) = state.financial_system.goods.recipes.get(recipe_id) {
            let mut effects = vec![];
            for _ in 0..batches {
                for (good_id, qty) in &recipe.inputs {
                    effects.push(StateEffect::RemoveInventory {
                        owner: firm_id.clone(),
                        good_id: *good_id,
                        quantity: *qty,
                    });
                }
                let (output_good, output_qty) = &recipe.output;
                effects.push(StateEffect::AddInventory {
                    owner: firm_id.clone(),
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
                errors: vec![EffectError::RecipieError { id: *recipe_id }],
            }
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
        let validator = FirmValidator::new(&state.financial_system);
        match action {
            SimAction::Produce { agent_id, recipe_id, batches } => {
                validator.validate_production(agent_id, recipe_id, *batches).is_ok()
            }
            SimAction::Hire { agent_id, count } => {
                // Get wage rate from firm or use default
                let wage_rate = state.firms.iter()
                    .find(|f| f.id == *agent_id)
                    .map(|f| f.wage_rate)
                    .unwrap_or(20.0);
                
                validator.validate_hire(agent_id, *count, wage_rate).is_ok()
            }
            _ => true,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        match action {
            SimAction::Hire { agent_id, count } => self.execute_hire(agent_id, *count, state),
            SimAction::Produce { agent_id, recipe_id, batches } => {
                self.execute_produce(agent_id, recipe_id, *batches, state)
            }
            _ => ExecutionResult::unhandled(self.name()),
        }
    }

    fn clone_box(&self) -> Box<dyn ExecutionDomain> {
        Box::new(self.clone())
    }
}

#[typetag::serde]
impl SerializableExecutionDomain for ProductionDomain {
    fn clone_box_serializable(&self) -> Box<dyn SerializableExecutionDomain> {
        Box::new(self.clone())
    }
}
