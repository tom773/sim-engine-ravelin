use super::{ExecutionDomain, SerializableExecutionDomain};
use shared::*; 
use crate::{SimState, ExecutionResult, StateEffect, EffectError};
use serde::{Serialize, Deserialize};

pub struct TradingDomainImpl {
}

impl TradingDomainImpl {
    pub fn new() -> Self {
        TradingDomainImpl {}
    }
    
    pub fn execute(&self, action: &SimAction, _state: &SimState) -> ExecutionResult {
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
            _ => return ExecutionResult {
                success: false,
                effects: vec![],
                errors: vec![EffectError::InvalidState("TradingDomain cannot handle this action".to_string())],
            },
        }
        ExecutionResult {
            success: true,
            effects, 
            errors: vec![],
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TradingDomain {
}

impl TradingDomain {
    pub fn new() -> Self {
        Self {
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
    
    fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        let impl_domain = TradingDomainImpl::new();
        impl_domain.execute(action, state)
    }
    
    fn clone_box(&self) -> Box<dyn SerializableExecutionDomain> {
        Box::new(self.clone())
    }
}

#[typetag::serde]
impl SerializableExecutionDomain for TradingDomain {}