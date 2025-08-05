use sim_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionValidator;

impl ProductionValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, action: &ProductionAction, state: &SimState) -> Result<(), String> {
        match action {
            ProductionAction::Hire { agent_id, count } => self.validate_hire(*agent_id, *count, state),
            ProductionAction::Produce { agent_id, recipe_id, batches } => {
                self.validate_produce(*agent_id, *recipe_id, *batches, state)
            }
            ProductionAction::PayWages { agent_id, amount, .. } => {
                Validator::positive_amount(*amount)?;
                if state.financial_system.get_liquid_assets(agent_id) < *amount {
                    return Err("Insufficient funds for wages".to_string());
                }
                Ok(())
            }
        }
    }

    fn validate_hire(&self, firm_id: AgentId, count: u32, state: &SimState) -> Result<(), String> {
        Validator::positive_integer(count, "hire count")?;
        if !state.agents.firms.contains_key(&firm_id) {
            return Err(format!("Firm {:?} not found", firm_id));
        }
        // Could add a check for liquidity to pay wages
        Ok(())
    }

    fn validate_produce(
        &self,
        firm_id: AgentId,
        recipe_id: RecipeId,
        batches: u32,
        state: &SimState,
    ) -> Result<(), String> {
        Validator::positive_integer(batches, "production batches")?;
        let firm = state.agents.firms.get(&firm_id).ok_or(format!("Firm {:?} not found", firm_id))?;
        let recipe = state.financial_system.goods.recipes.get(&recipe_id).ok_or(format!("Recipe {:?} not found", recipe_id))?;

        let bs = state.financial_system.balance_sheets.get(&firm_id).ok_or("Firm has no balance sheet")?;
        let inventory = bs.get_inventory().ok_or("Firm has no inventory")?;

        for (input_good, required_qty) in &recipe.inputs {
            let available = inventory.get(input_good).map_or(0.0, |item| item.quantity);
            let total_needed = *required_qty * batches as f64;
            if available < total_needed {
                return Err(format!("Insufficient input {:?}: have {:.2}, need {:.2}", input_good, available, total_needed));
            }
        }

        if firm.employees.is_empty() {
            return Err("Firm has no employees to produce".to_string());
        }
        Ok(())
    }
}