use crate::{Any, Domain, DomainResult, ResolutionContext, ResolutionPhase, ResolutionResult, inventory};
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumptionDomain {}

impl ConsumptionDomain {
    pub fn new() -> Self {
        Self {}
    }

    fn get_agent_inventory(fs: &FinancialSystem, agent_id: &AgentId) -> HashMap<GoodId, InventoryItem> {
        if let Some(bs) = fs.balance_sheets.get(agent_id) {
            for inst_id in bs.assets.keys() {
                if let Some(inst) = fs.instruments.instruments.get(inst_id) {
                    if let InstrumentRuntime::RealAsset(RealAssetState::Inventory { goods, .. }) = &inst.state() {
                        return goods.clone();
                    }
                }
            }
        }
        HashMap::new()
    }
}

impl Domain for ConsumptionDomain {
    fn name(&self) -> &'static str {
        "Consumption"
    }

    fn resolve_intention(&self, intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::Consumption(ConsumptionIntention::ConsumeGood { agent_id, good_id, quantity }) => {
                let qty = quantity.round();
                if qty >= 1.0 {
                    vec![SimAction::Consumption(ConsumptionAction::Consume {
                        agent_id: *agent_id,
                        good_id: *good_id,
                        amount: qty,
                    })]
                } else {
                    vec![SimAction::Consumption(ConsumptionAction::NoAction { agent_id: *agent_id })]
                }
            }
            _ => return None,
        };

        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::Consumption(ConsumptionIntention::ConsumeGood { .. }) => Some(ResolutionPhase::Independent),
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let consumption_action = match action {
            SimAction::Consumption(action) => action,
            _ => return DomainResult::failure(vec!["Not a consumption action".to_string()]),
        };

        if let Err(error) = self.validate(consumption_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match consumption_action {
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                tracing::info!("Agent {:?} consuming {:.2} of good {:?}", agent_id, amount, good_id);
                self.execute_consume(*agent_id, *good_id, *amount)
            }
            ConsumptionAction::NoAction { .. } => DomainResult::empty(),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ConsumptionDomain {
    fn validate(&self, action: &ConsumptionAction, state: &SimState) -> Result<(), String> {
        match action {
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                Validator::positive_amount(*amount)?;
                Validator::agent_exists(*agent_id, state)?;

                let fs = &state.financial_system;
                let inventory = Self::get_agent_inventory(fs, agent_id);

                let available = inventory.get(good_id).map_or(0.0, |item| item.quantity);

                if available < *amount {
                    return Err(format!(
                        "Agent has insufficient goods to consume: needs {:.2}, has {:.2}",
                        amount, available
                    ));
                }
                Ok(())
            }

            ConsumptionAction::NoAction { .. } => Ok(()),
        }
    }

    fn execute_consume(&self, agent_id: AgentId, good_id: GoodId, amount: f64) -> DomainResult {
        let effects = vec![StateEffect::Inventory(InventoryEffect::RemoveInventory {
            owner: agent_id,
            good_id,
            quantity: amount,
        })];

        DomainResult::success(effects)
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Consumption",
        constructor: || Box::new(ConsumptionDomain::new()),
    }
}
