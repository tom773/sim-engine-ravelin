use serde::{Deserialize, Serialize};
use sim_core::*;
use sim_macros::SimDomain;

#[derive(Clone, Debug, Serialize, Deserialize, SimDomain)]
pub struct ProductionDomain {}

#[derive(Debug, Clone)]
pub struct ProductionResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl ProductionDomain {
    pub fn new() -> Self {
        Self {}
    }

    pub fn can_handle(&self, action: &ProductionAction) -> bool {
        matches!(action, ProductionAction::Hire { .. } | ProductionAction::Produce { .. })
    }

    pub fn validate(&self, action: &ProductionAction, state: &SimState) -> Result<(), String> {
        match action {
            ProductionAction::Hire { agent_id, count } => self.validate_hire(*agent_id, *count, state),
            ProductionAction::Produce { agent_id, recipe_id, batches } => {
                self.validate_produce(*agent_id, *recipe_id, *batches, state)
            }
        }
    }

    fn validate_hire(&self, firm_id: AgentId, count: u32, state: &SimState) -> Result<(), String> {
        Validator::positive_integer(count, "hire count")?;
        if !state.agents.firms.contains_key(&firm_id) {
            return Err(format!("Firm {:?} not found", firm_id));
        }
        Ok(())
    }

    fn validate_produce(
        &self, firm_id: AgentId, recipe_id: RecipeId, batches: u32, state: &SimState,
    ) -> Result<(), String> {
        Validator::positive_integer(batches, "production batches")?;
        let firm = state.agents.firms.get(&firm_id).ok_or(format!("Firm {:?} not found", firm_id))?;
        let recipe =
            state.financial_system.goods.recipes.get(&recipe_id).ok_or(format!("Recipe {:?} not found", recipe_id))?;

        let bs = state.financial_system.balance_sheets.get(&firm_id).ok_or("Firm has no balance sheet")?;
        let inventory = bs.get_inventory().ok_or("Firm has no inventory")?;

        for (input_good, required_qty) in &recipe.inputs {
            let available = inventory.get(input_good).map_or(0.0, |item| item.quantity);
            let total_needed = *required_qty * batches as f64;
            if available < total_needed {
                return Err(format!(
                    "Insufficient input {:?}: have {:.2}, need {:.2}",
                    input_good, available, total_needed
                ));
            }
        }

        if firm.employees.is_empty() {
            return Err("Firm has no employees to produce".to_string());
        }
        Ok(())
    }

    pub fn execute(&self, action: &ProductionAction, state: &SimState) -> ProductionResult {
        if let Err(error) = self.validate(action, state) {
            return ProductionResult { success: false, effects: vec![], errors: vec![error] };
        }

        match action {
            ProductionAction::Hire { agent_id, count } => self.execute_hire(*agent_id, *count),
            ProductionAction::Produce { agent_id, recipe_id, batches } => {
                self.execute_produce(*agent_id, *recipe_id, *batches, state)
            }
        }
    }
    pub fn execute_hire(&self, firm_id: AgentId, count: u32) -> ProductionResult {
        let effects = vec![StateEffect::Agent(AgentEffect::Hire { firm: firm_id, count })];
        ProductionResult { success: true, effects, errors: vec![] }
    }

    pub fn execute_produce(
        &self, firm_id: AgentId, recipe_id: RecipeId, batches: u32, state: &SimState,
    ) -> ProductionResult {
        let recipe = match state.financial_system.goods.recipes.get(&recipe_id) {
            Some(r) => r,
            None => {
                return ProductionResult {
                    success: false,
                    effects: vec![],
                    errors: vec![format!("Recipe {:?} not found", recipe_id)],
                };
            }
        };

        let mut effects = vec![];
        let total_batches = batches as f64;

        for (input_good, required_qty) in &recipe.inputs {
            effects.push(StateEffect::Inventory(InventoryEffect::RemoveInventory {
                owner: firm_id,
                good_id: *input_good,
                quantity: *required_qty * total_batches,
            }));
        }

        let (output_good, output_qty) = &recipe.output;
        let total_output = output_qty * total_batches * recipe.efficiency;
        effects.push(StateEffect::Inventory(InventoryEffect::AddInventory {
            owner: firm_id,
            good_id: *output_good,
            quantity: total_output,
            unit_cost: 0.0,
        }));

        ProductionResult { success: true, effects, errors: vec![] }
    }
}

impl Default for ProductionDomain {
    fn default() -> Self {
        Self::new()
    }
}
