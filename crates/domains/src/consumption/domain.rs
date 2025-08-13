use serde::{Deserialize, Serialize};
use sim_core::*;
use sim_macros::SimDomain;
use crate::banking::BankingDomain;

#[derive(Clone, Debug, Serialize, Deserialize, SimDomain)]
pub struct ConsumptionDomain {
    payment_router: BankingDomain,
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
            payment_router: BankingDomain::new(),
        }
    }

    pub fn can_handle(&self, action: &ConsumptionAction) -> bool {
        matches!(action, ConsumptionAction::Purchase { .. } | ConsumptionAction::Consume { .. } | ConsumptionAction::PurchaseAtBest { .. })
    }

    pub fn validate(&self, action: &ConsumptionAction, state: &SimState) -> Result<(), String> {
        match action {
            ConsumptionAction::Purchase { agent_id, seller, good_id, amount } => {
                self.validate_purchase(*agent_id, *seller, *good_id, *amount, state)
            }
            ConsumptionAction::PurchaseAtBest { agent_id, good_id, max_notional } => {
                self.validate_purchase_at_best(*agent_id, *good_id, *max_notional, state)
           }
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                self.validate_consume(*agent_id, *good_id, *amount, state)
            }
            ConsumptionAction::NoAction { agent_id: _agent_id } => Ok(()),
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
        let available_inventory =
            seller_bs.get_inventory().and_then(|inv| inv.get(&good_id)).map_or(0.0, |item| item.quantity);
        if available_inventory < amount {
            return Err(format!(
                "Seller has insufficient inventory: needs {:.2}, has {:.2}",
                amount, available_inventory
            ));
        }

        let price =
            state.financial_system.exchange.goods_market(&good_id).and_then(|m| m.best_ask()).map_or(1.0, |ask| ask.price);
        let total_cost = amount * price;
        let available_funds = state.financial_system.get_liquid_assets(&buyer);
        if available_funds < total_cost {
            return Err(format!("Buyer has insufficient funds: needs ${:.2}, has ${:.2}", total_cost, available_funds));
        }

        Ok(())
    }

    fn validate_purchase_at_best(
        &self,
        buyer: AgentId,
        _good_id: GoodId,
        max_notional: f64,
        state: &SimState,
    ) -> Result<(), String> {
        Validator::positive_amount(max_notional)?;
        if !state.financial_system.balance_sheets.contains_key(&buyer) {
            return Err(format!("Buyer {:?} not found", buyer));
        }
        let available_funds = state.financial_system.get_liquid_assets(&buyer);
        if available_funds < max_notional {
             return Err(format!("Buyer has insufficient funds for max notional: needs ${:.2}, has ${:.2}", max_notional, available_funds));
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
            ConsumptionAction::PurchaseAtBest { agent_id, good_id, max_notional } => {
                self.execute_purchase_at_best(*agent_id, *good_id, *max_notional, state)
            }
            ConsumptionAction::Consume { agent_id, good_id, amount } => self.execute_consume(*agent_id, *good_id, *amount),
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

        let price =
            state.financial_system.exchange.goods_market(&good_id).and_then(|m| m.best_ask()).map_or(1.0, |ask| ask.price);

        let total_cost = amount * price;

        let payment_result = self.payment_router.execute_transfer(buyer, seller, total_cost, state);

        if !payment_result.success {
            return ConsumptionResult {
                success: false,
                effects: vec![],
                errors: vec![format!("Payment failed during direct purchase: {:?}", payment_result.errors)],
            };
        }

        effects.extend(payment_result.effects);

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

    pub fn execute_purchase_at_best(
        &self,
        buyer: AgentId,
        good_id: GoodId,
        max_notional: f64,
        state: &SimState,
    ) -> ConsumptionResult {
         let market = match state.financial_system.exchange.goods_market(&good_id) {
            Some(m) => m,
            None => return ConsumptionResult { success: true, effects: vec![], errors: vec![] }, // Market doesn't exist
        };

        let mut remaining_notional = max_notional;
        let mut effects = vec![];

        let mut asks = market.order_book.asks.clone();
        asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));

        for ask in asks {
            if remaining_notional <= 1e-6 {
                break;
            }

            let cost_at_ask_price = ask.quantity * ask.price;
            let bid_quantity;

            if cost_at_ask_price <= remaining_notional {
                bid_quantity = ask.quantity;
                remaining_notional -= cost_at_ask_price;
            } else {
                bid_quantity = remaining_notional / ask.price;
                remaining_notional = 0.0;
            }

            if bid_quantity > 1e-6 {
                effects.push(StateEffect::Market(MarketEffect::PlaceOrderInBook {
                    market_id: MarketId::Goods(good_id),
                    order: Order::Bid(Bid {
                        agent_id: buyer,
                        quantity: bid_quantity,
                        price: ask.price,
                    }),
                }));
            }
        }

        ConsumptionResult { success: true, effects, errors: vec![] }
    }


    pub fn execute_consume(&self, agent_id: AgentId, good_id: GoodId, amount: f64) -> ConsumptionResult {
        let effects =
            vec![StateEffect::Inventory(InventoryEffect::RemoveInventory { owner: agent_id, good_id, quantity: amount })];

        ConsumptionResult { success: true, effects, errors: vec![] }
    }
}

impl Default for ConsumptionDomain {
    fn default() -> Self {
        Self::new()
    }
}