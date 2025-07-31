use crate::*;
pub struct FirmValidator<'a> {
    fs: &'a FinancialSystem,
}

impl<'a> FirmValidator<'a> {
    pub fn new(fs: &'a FinancialSystem) -> Self {
        Self { fs }
    }
    
    pub fn validate_production(&self, firm_id: &AgentId, recipe_id: &RecipeId, batches: u32) -> Result<(), String> {
        Validator::positive_integer(batches, "batches")?;
        
        self.ensure_is_firm(firm_id)?;
        
        let recipe = self.fs.goods.recipes.get(recipe_id)
            .ok_or_else(|| format!("Recipe {:?} not found", recipe_id))?;
        
        if let Some(bs) = self.fs.get_bs_by_id(firm_id) {
            if let Some(inventory) = bs.get_inventory() {
                for (input_good, required_qty) in &recipe.inputs {
                    let available = inventory.get(input_good)
                        .map(|item| item.quantity)
                        .unwrap_or(0.0);
                    
                    let total_needed = required_qty * batches as f64;
                    if available < total_needed {
                        return Err(format!(
                            "Insufficient input {:?}: have {:.2}, need {:.2}", 
                            input_good, available, total_needed
                        ));
                    }
                }
            } else {
                return Err("Firm has no inventory".to_string());
            }
        }
        
        Ok(())
    }
    
    pub fn validate_hire(&self, firm_id: &AgentId, count: u32, wage_rate: f64) -> Result<(), String> {
        Validator::positive_integer(count, "hire count")?;
        Validator::positive_amount(wage_rate)?;
        
        self.ensure_is_firm(firm_id)?;
        
        let wage_cost = wage_rate * count as f64 * 40.0; // Weekly wage
        let liquidity = self.fs.get_liquid_assets(firm_id);
        
        if liquidity < wage_cost * 4.0 { // Ensure 4 weeks of wages
            return Err(format!(
                "Insufficient liquidity for wages: ${:.2} < ${:.2}", 
                liquidity, wage_cost * 4.0
            ));
        }
        
        Ok(())
    }
    
    pub fn validate_sell_inventory(&self, firm_id: &AgentId, good_id: &GoodId, quantity: f64, price: f64) -> Result<(), String> {
        Validator::positive_amount(quantity)?;
        Validator::positive_amount(price)?;
        
        self.ensure_is_firm(firm_id)?;
        
        let available = self.get_available_inventory(firm_id, good_id)?;
        if available < quantity {
            return Err(format!(
                "Insufficient inventory of {:?}: have {:.2}, trying to sell {:.2}",
                good_id, available, quantity
            ));
        }
        
        
        Ok(())
    }
    
    pub fn validate_pay_wages(&self, firm_id: &AgentId, employee_id: &AgentId, amount: f64) -> Result<(), String> {
        Validator::positive_amount(amount)?;
        
        self.ensure_is_firm(firm_id)?;
        self.ensure_has_balance_sheet(employee_id)?;
        
        let liquidity = self.fs.get_liquid_assets(firm_id);
        if liquidity < amount {
            return Err(format!(
                "Insufficient liquidity for wage payment: ${:.2} < ${:.2}",
                liquidity, amount
            ));
        }
        
        
        Ok(())
    }
    
    fn ensure_is_firm(&self, firm_id: &AgentId) -> Result<(), String> {
        if self.fs.balance_sheets.contains_key(firm_id) {
            Ok(())
        } else {
            Err(format!("Firm {} not found", &firm_id.0.to_string()[..8]))
        }
    }
    
    fn ensure_has_balance_sheet(&self, agent_id: &AgentId) -> Result<(), String> {
        if self.fs.balance_sheets.contains_key(agent_id) {
            Ok(())
        } else {
            Err(format!("Agent {} does not have a balance sheet", &agent_id.0.to_string()[..8]))
        }
    }
    
    fn get_available_inventory(&self, firm_id: &AgentId, good_id: &GoodId) -> Result<f64, String> {
        self.fs.get_bs_by_id(firm_id)
            .and_then(|bs| bs.get_inventory())
            .and_then(|inv| inv.get(good_id))
            .map(|item| item.quantity)
            .ok_or_else(|| format!("No inventory of good {:?}", good_id))
    }
}