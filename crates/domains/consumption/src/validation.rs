use sim_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumptionValidator;

impl ConsumptionValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, action: &ConsumptionAction, state: &SimState) -> Result<(), String> {
        match action {
            ConsumptionAction::Purchase { agent_id, seller, good_id, amount } => {
                self.validate_purchase(*agent_id, *seller, *good_id, *amount, state)
            }
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                self.validate_consume(*agent_id, *good_id, *amount, state)
            }
        }
    }

    fn validate_purchase(
        &self,
        buyer: AgentId,
        seller: AgentId,
        good_id: GoodId,
        amount: f64,
        state: &SimState,
    ) -> Result<(), String> {
        Validator::positive_amount(amount)?;

        // Check agents exist
        if !state.financial_system.balance_sheets.contains_key(&buyer) {
            return Err(format!("Buyer {:?} not found", buyer));
        }
        if !state.financial_system.balance_sheets.contains_key(&seller) {
            return Err(format!("Seller {:?} not found", seller));
        }

        // Check seller has inventory
        let seller_bs = state.financial_system.balance_sheets.get(&seller).unwrap();
        let available_inventory = seller_bs.get_inventory().and_then(|inv| inv.get(&good_id)).map_or(0.0, |item| item.quantity);
        if available_inventory < amount {
            return Err(format!("Seller has insufficient inventory: needs {:.2}, has {:.2}", amount, available_inventory));
        }

        // Check buyer has funds
        let price = state.financial_system.exchange.goods_market(&good_id)
            .and_then(|m| m.best_ask()).map_or(1.0, |ask| ask.price);
        let total_cost = amount * price;
        let available_funds = state.financial_system.get_liquid_assets(&buyer);
        if available_funds < total_cost {
            return Err(format!("Buyer has insufficient funds: needs ${:.2}, has ${:.2}", total_cost, available_funds));
        }

        Ok(())
    }

    fn validate_consume(&self, agent_id: AgentId, good_id: GoodId, amount: f64, state: &SimState) -> Result<(), String> {
        Validator::positive_amount(amount)?;

        let bs = state.financial_system.balance_sheets.get(&agent_id).ok_or(format!("Agent {:?} not found", agent_id))?;
        let available = bs.get_inventory().and_then(|inv| inv.get(&good_id)).map_or(0.0, |item| item.quantity);

        if available < amount {
            return Err(format!("Agent has insufficient goods to consume: needs {:.2}, has {:.2}", amount, available));
        }

        Ok(())
    }
}