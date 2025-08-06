use sim_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumptionDomain {
}

#[derive(Debug, Clone)]
pub struct ConsumptionResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl ConsumptionDomain {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn can_handle(&self, action: &ConsumptionAction) -> bool {
        matches!(action, ConsumptionAction::Purchase { .. } | ConsumptionAction::Consume { .. })
    }

    pub fn validate(&self, action: &ConsumptionAction, state: &SimState) -> Result<(), String> {
        match action {
            ConsumptionAction::Purchase { agent_id, seller, good_id, amount } => {
                self.validate_purchase(*agent_id, *seller, *good_id, *amount, state)
            }
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                self.validate_consume(*agent_id, *good_id, *amount, state)
            }
            ConsumptionAction::NoAction { agent_id: _agent_id } => {
                Ok(())
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

        if !state.financial_system.balance_sheets.contains_key(&buyer) {
            return Err(format!("Buyer {:?} not found", buyer));
        }
        if !state.financial_system.balance_sheets.contains_key(&seller) {
            return Err(format!("Seller {:?} not found", seller));
        }

        let seller_bs = state.financial_system.balance_sheets.get(&seller).unwrap();
        let available_inventory = seller_bs.get_inventory().and_then(|inv| inv.get(&good_id)).map_or(0.0, |item| item.quantity);
        if available_inventory < amount {
            return Err(format!("Seller has insufficient inventory: needs {:.2}, has {:.2}", amount, available_inventory));
        }

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

    pub fn execute(&self, action: &ConsumptionAction, state: &SimState) -> ConsumptionResult {
        if let Err(error) = self.validate(action, state) {
            return ConsumptionResult { success: false, effects: vec![], errors: vec![error] };
        }

        match action {
            ConsumptionAction::Purchase { agent_id, seller, good_id, amount } => {
                self.execute_purchase(*agent_id, *seller, *good_id, *amount, state)
            }
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                self.execute_consume(*agent_id, *good_id, *amount)
            }
            ConsumptionAction::NoAction { agent_id: _ } => {
                ConsumptionResult { success: true, effects: vec![], errors: vec![] }
            }
        }
    }
    pub fn execute_purchase(
        &self,
        buyer: AgentId,
        seller: AgentId,
        good_id: GoodId,
        amount: f64,
        state: &SimState,
    ) -> ConsumptionResult {
        let mut effects = vec![];

        let price = state
            .financial_system
            .exchange
            .goods_market(&good_id)
            .and_then(|m| m.best_ask())
            .map_or(1.0, |ask| ask.price);

        let total_cost = amount * price;

        if let Some((cash_id, cash)) = state.financial_system.get_bs_by_id(&buyer)
            .and_then(|bs| bs.assets.iter().find(|(_, inst)| inst.details.as_any().is::<CashDetails>()))
        {
            effects.push(StateEffect::Financial(FinancialEffect::UpdateInstrument {
                id: *cash_id,
                new_principal: cash.principal - total_cost,
            }));
            let seller_cash = cash!(seller, total_cost, state.financial_system.central_bank.id, state.current_date);
            effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(seller_cash)));
        } else {
            return ConsumptionResult { success: false, effects: vec![], errors: vec!["Buyer has no cash instrument".to_string()] };
        }

        effects.push(StateEffect::Inventory(InventoryEffect::RemoveInventory {
            owner: seller,
            good_id,
            quantity: amount,
        }));
        effects.push(StateEffect::Inventory(InventoryEffect::AddInventory {
            owner: buyer,
            good_id,
            quantity: amount,
            unit_cost: price,
        }));

        ConsumptionResult { success: true, effects, errors: vec![] }
    }

    pub fn execute_consume(&self, agent_id: AgentId, good_id: GoodId, amount: f64) -> ConsumptionResult {
        let effects = vec![StateEffect::Inventory(InventoryEffect::RemoveInventory {
            owner: agent_id,
            good_id,
            quantity: amount,
        })];

        ConsumptionResult { success: true, effects, errors: vec![] }
    }
}

impl Default for ConsumptionDomain {
    fn default() -> Self {
        Self::new()
    }
}