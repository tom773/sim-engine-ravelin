use sim_prelude::*;
use crate::ProductionResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionOperations;

impl ProductionOperations {
    pub fn new() -> Self {
        Self
    }

    pub fn execute_hire(&self, firm_id: AgentId, count: u32) -> ProductionResult {
        // In a more complex model, this would find unemployed agents.
        // For now, it's a simple effect that a firm behavior model can react to.
        let effects = vec![StateEffect::Agent(AgentEffect::Hire { firm: firm_id, count })];
        ProductionResult { success: true, effects, errors: vec![] }
    }

    pub fn execute_produce(
        &self,
        firm_id: AgentId,
        recipe_id: RecipeId,
        batches: u32,
        state: &SimState,
    ) -> ProductionResult {
        let recipe = match state.financial_system.goods.recipes.get(&recipe_id) {
            Some(r) => r,
            None => {
                return ProductionResult {
                    success: false,
                    effects: vec![],
                    errors: vec![format!("Recipe {:?} not found", recipe_id)],
                }
            }
        };

        let mut effects = vec![];
        let total_batches = batches as f64;

        // 1. Consume inputs
        for (input_good, required_qty) in &recipe.inputs {
            effects.push(StateEffect::Inventory(InventoryEffect::RemoveInventory {
                owner: firm_id,
                good_id: *input_good,
                quantity: *required_qty * total_batches,
            }));
        }

        // 2. Add output
        let (output_good, output_qty) = &recipe.output;
        let total_output = output_qty * total_batches * recipe.efficiency;
        // A simple cost model: for now, unit cost is 0 as we don't model input costs perfectly yet.
        effects.push(StateEffect::Inventory(InventoryEffect::AddInventory {
            owner: firm_id,
            good_id: *output_good,
            quantity: total_output,
            unit_cost: 0.0,
        }));

        ProductionResult { success: true, effects, errors: vec![] }
    }
}