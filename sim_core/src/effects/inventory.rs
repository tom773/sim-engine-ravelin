use crate::*;
use crate::types::money::Money;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InventoryEffect {
    AddInventory { owner: AgentId, good_id: GoodId, quantity: f64, unit_cost: f64 },
    RemoveInventory { owner: AgentId, good_id: GoodId, quantity: f64 },
}

impl InventoryEffect {
    pub fn name(&self) -> &'static str {
        match self {
            InventoryEffect::AddInventory { .. } => "AddInventory",
            InventoryEffect::RemoveInventory { .. } => "RemoveInventory",
        }
    }
}

impl StateEffectApplicator {
    pub fn apply_inventory_effect(
        state: &mut SimState,
        effect: &InventoryEffect,
    ) -> Result<(), EffectError> {
        match effect {
            InventoryEffect::AddInventory {
                owner,
                good_id,
                quantity,
                unit_cost,
            } => {
                let money_unit_cost = Money::from_f64(*unit_cost).unwrap_or(Money::ZERO);
                state
                    .financial_system
                    .add_to_inventory(owner, good_id, *quantity, money_unit_cost);
                Ok(())
            }
            InventoryEffect::RemoveInventory {
                owner,
                good_id,
                quantity,
            } => state
                .financial_system
                .remove_from_inventory(owner, good_id, *quantity)
                .map_err(EffectError::FinancialSystemError),
        }
    }
}