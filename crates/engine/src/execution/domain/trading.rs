use shared::*; 
use crate::{state::SimState, effects::{StateEffect, ExecutionResult}, domain::ExecutionDomain};

pub struct TradingDomain {
}

impl TradingDomain {
    pub fn new() -> Self {
        TradingDomain {
        }
    }
}

impl ExecutionDomain for TradingDomain {
    fn name(&self) -> &'static str {
        "TradingDomain"
    }

    fn can_handle(&self, action: &SimAction) -> bool {
        matches!(action, SimAction::PostBid { .. } | SimAction::PostAsk { .. })
    }
    fn validate(&self, _action: &SimAction, _state: &SimState) -> bool {
        true
    }
    fn execute(&self, action: &SimAction, _state: &SimState) -> ExecutionResult {
        let mut effects = Vec::new();
        match action {
            SimAction::PostBid { agent_id, market_id, quantity, price } => {
                effects.push(StateEffect::PlaceOrderInBook {
                    market_id: market_id.clone(),
                    order: Order::Bid(Bid {
                        agent_id: agent_id.clone(),
                        quantity: *quantity,
                        price: *price,
                    }),
                });
            }
            SimAction::PostAsk { agent_id, market_id, quantity, price } => {
                effects.push(StateEffect::PlaceOrderInBook {
                    market_id: market_id.clone(),
                    order: Order::Ask(Ask {
                        agent_id: agent_id.clone(),
                        quantity: *quantity,
                        price: *price,
                    }),
                });
            }
            _ => unreachable!(),
        }
        ExecutionResult {
            success: true,
            effects, 
            errors: vec![],
        }
    }
}