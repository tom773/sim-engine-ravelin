#[allow(unused_variables)]
use super::SerializableExecutionDomain;
use crate::validation::{FinancialValidator, FirmValidator};
use crate::{SimState, StateEffect, *};
use ravelin_traits::ExecutionResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct TradingDomain {}

impl TradingDomain {
    pub fn new() -> Self {
        Self {}
    }
}
impl_execution_domain! {
    TradingDomain,
    "TradingDomain",
    validate = |action, state| {
        match action {
            SimAction::PostAsk { agent_id, market_id, quantity, price } => {
                if let MarketId::Goods(good_id) = market_id {
                    let validator = FirmValidator::new(&state.financial_system);
                    validator.validate_sell_inventory(agent_id, good_id, *quantity, *price).is_ok()
                } else {
                    true
                }
            }
            SimAction::PostBid { agent_id, quantity, price, .. } => {
                let validator = FinancialValidator::new(&state.financial_system);
                validator.ensure_sufficient_cash(agent_id, *quantity * *price).is_ok()
            }
            _ => true,
        }
    },
    execute = |_self_domain, _action, _state| {
        SimAction::PostBid { agent_id: _agent_id, market_id: _market_id, quantity: _quantity, price: _price } => {

            let effects = vec![StateEffect::PlaceOrderInBook {
                market_id: _market_id.clone(),
                order: Order::Bid(Bid { agent_id: *_agent_id, quantity: *_quantity, price: *_price }),
            }];
            ExecutionResult { success: true, effects, errors: vec![] }
        },
        SimAction::PostAsk { agent_id: _agent_id, market_id: _market_id, quantity: _quantity, price: _price } => {
            let effects = vec![StateEffect::PlaceOrderInBook {
                market_id: _market_id.clone(),
                order: Order::Ask(Ask { agent_id: *_agent_id, quantity: *_quantity, price: *_price }),
            }];
            ExecutionResult { success: true, effects, errors: vec![] }
        }
    }
}

#[typetag::serde]
impl SerializableExecutionDomain for TradingDomain {
    fn clone_box_serializable(&self) -> Box<dyn SerializableExecutionDomain> {
        Box::new(self.clone())
    }
}