use sim_prelude::*;
use crate::ConsumptionResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumptionOperations;

impl ConsumptionOperations {
    pub fn new() -> Self {
        Self
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

        // Determine price from the best ask on the market
        let price = state
            .financial_system
            .exchange
            .goods_market(&good_id)
            .and_then(|m| m.best_ask())
            .map_or(1.0, |ask| ask.price); // Default price if market is empty

        let total_cost = amount * price;

        // 1. Transfer funds from buyer to seller
        // This is a meta-operation; we create the primitive effects directly.
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

        // 2. Transfer inventory from seller to buyer
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