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
                if let Some(inst) = fs.instruments.get(inst_id) {
                    if let InstrumentType::RealAsset(RealAssetType::Inventory { goods, .. }) = &inst.instrument_type {
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

    fn resolve_intention(&self, intention: &SimIntention, context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::Production(ProductionIntention::ApplyForJob { agent_id: _, market_id, application }) => {
                vec![SimAction::Transaction(TransactionAction::PostJobApplication {
                    market_id: *market_id,
                    application: application.clone(),
                })]
            }

            SimIntention::Consumption(ConsumptionIntention::ConsumeGood { agent_id, good_id, quantity }) => {
                let qty = quantity.round();
                vec![SimAction::Consumption(ConsumptionAction::Consume {

                    agent_id: *agent_id,
                    good_id: *good_id,
                    amount: qty,
                })]
            }

            SimIntention::Consumption(ConsumptionIntention::SpendOnGood { agent_id, good_id, max_notional }) => {
                let px =
                    context.state.financial_system.exchange.goods_market(good_id).and_then(|v| v.book.best_ask()).unwrap_or(Money::from(1));


                let qty = (max_notional / px.to_f64()).max(0.0);
                let start_bid = Money::from_f64(px.to_f64() * 1.01).unwrap_or_default(); // +1%

                vec![SimAction::Transaction(TransactionAction::PostMarketOrder {
                    agent_id: *agent_id,
                    market_id: MarketId::Goods(*good_id),
                    side: Side::Bid,
                    quantity: qty.round(),
                    price: Some(start_bid),
                    order_type: OrderType::Limit,
                })]
            }

            _ => return None,
        };

        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::Production(ProductionIntention::ApplyForJob { .. }) => Some(ResolutionPhase::Independent),
            SimIntention::Consumption(ConsumptionIntention::ConsumeGood { .. }) => Some(ResolutionPhase::Independent),
            SimIntention::Consumption(ConsumptionIntention::SpendOnGood { .. }) => Some(ResolutionPhase::Market),
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
