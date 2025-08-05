use sim_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradingValidator;

impl TradingValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, action: &TradingAction, state: &SimState) -> Result<(), String> {
        match action {
            TradingAction::PostBid { agent_id, quantity, price, .. } => {
                self.validate_post_bid(*agent_id, *quantity, *price, state)
            }
            TradingAction::PostAsk { agent_id, market_id, quantity, .. } => {
                self.validate_post_ask(*agent_id, market_id, *quantity, state)
            }
        }
    }

    fn validate_post_bid(&self, agent_id: AgentId, quantity: f64, price: f64, state: &SimState) -> Result<(), String> {
        Validator::positive_amount(quantity)?;
        Validator::positive_amount(price)?;

        if !state.financial_system.balance_sheets.contains_key(&agent_id) {
            return Err(format!("Bidding agent {:?} not found", agent_id));
        }

        let required_cash = quantity * price;
        let available_cash = state.financial_system.get_liquid_assets(&agent_id);

        if available_cash < required_cash {
            return Err(format!(
                "Insufficient funds for bid: agent {:?} needs ${:.2}, has ${:.2}",
                agent_id, required_cash, available_cash
            ));
        }

        Ok(())
    }

    fn validate_post_ask(
        &self,
        agent_id: AgentId,
        market_id: &MarketId,
        quantity: f64,
        state: &SimState,
    ) -> Result<(), String> {
        Validator::positive_amount(quantity)?;

        if !state.financial_system.balance_sheets.contains_key(&agent_id) {
            return Err(format!("Asking agent {:?} not found", agent_id));
        }

        if let MarketId::Goods(good_id) = market_id {
            let bs = state.financial_system.balance_sheets.get(&agent_id).unwrap();
            let available_inventory = bs.get_inventory().and_then(|inv| inv.get(good_id)).map_or(0.0, |item| item.quantity);

            if available_inventory < quantity {
                return Err(format!(
                    "Insufficient inventory for ask: agent {:?} needs {:.2}, has {:.2}",
                    agent_id, quantity, available_inventory
                ));
            }
        }
        // Note: Validation for financial instruments would go here

        Ok(())
    }
}