use super::{ExecutionDomain, SerializableExecutionDomain};
use crate::{ExecutionResult, SimState, StateEffect};
use serde::{Deserialize, Serialize};
use shared::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct TradingDomain {}

impl TradingDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl ExecutionDomain for TradingDomain {
    fn name(&self) -> &'static str {
        "TradingDomain"
    }

    fn can_handle(&self, action: &SimAction) -> bool {
        matches!(action, SimAction::PostBid { .. } | SimAction::PostAsk { .. })
    }

    fn validate(&self, action: &SimAction, state: &SimState) -> bool {
        match action {
            SimAction::PostAsk { agent_id, market_id, quantity, price } => {
                if let MarketId::Goods(good_id) = market_id {
                    let validator = FirmValidator::new(&state.financial_system);
                    validator.validate_sell_inventory(agent_id, good_id, *quantity, *price).is_ok()
                } else {
                    // TODO: Add validation for selling financial instruments
                    true
                }
            }
            SimAction::PostBid { agent_id, quantity, price, .. } => {
                let validator = FinancialValidator::new(&state.financial_system);
                validator.ensure_sufficient_cash(agent_id, *quantity * *price).is_ok()
            }
            _ => true,
        }
    }

    fn execute(&self, action: &SimAction, _state: &SimState) -> ExecutionResult {
        let mut effects = Vec::new();
        match action {
            SimAction::PostBid { agent_id, market_id, quantity, price } => {
                effects.push(StateEffect::PlaceOrderInBook {
                    market_id: market_id.clone(),
                    order: Order::Bid(Bid { agent_id: agent_id.clone(), quantity: *quantity, price: *price }),
                });
            }
            SimAction::PostAsk { agent_id, market_id, quantity, price } => {
                effects.push(StateEffect::PlaceOrderInBook {
                    market_id: market_id.clone(),
                    order: Order::Ask(Ask { agent_id: agent_id.clone(), quantity: *quantity, price: *price }),
                });
            }
            _ => return ExecutionResult::unhandled(self.name()),
        }
        ExecutionResult { success: true, effects, errors: vec![] }
    }

    fn clone_box(&self) -> Box<dyn ExecutionDomain> {
        Box::new(self.clone())
    }
}

#[typetag::serde]
impl SerializableExecutionDomain for TradingDomain {
    fn clone_box_serializable(&self) -> Box<dyn SerializableExecutionDomain> {
        Box::new(self.clone())
    }
}