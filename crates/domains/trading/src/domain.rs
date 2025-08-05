use sim_prelude::*;
use crate::{TradingOperations, TradingValidator};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradingDomain {
    validator: TradingValidator,
    operations: TradingOperations,
}

#[derive(Debug, Clone)]
pub struct TradingResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl TradingDomain {
    pub fn new() -> Self {
        Self {
            validator: TradingValidator::new(),
            operations: TradingOperations::new(),
        }
    }

    pub fn can_handle(&self, action: &TradingAction) -> bool {
        matches!(action, TradingAction::PostBid { .. } | TradingAction::PostAsk { .. })
    }

    pub fn validate(&self, action: &TradingAction, state: &SimState) -> Result<(), String> {
        self.validator.validate(action, state)
    }

    pub fn execute(&self, action: &TradingAction, state: &SimState) -> TradingResult {
        if let Err(error) = self.validate(action, state) {
            return TradingResult { success: false, effects: vec![], errors: vec![error] };
        }

        match action {
            TradingAction::PostBid { agent_id, market_id, quantity, price } => {
                self.operations.execute_post_bid(*agent_id, market_id.clone(), *quantity, *price)
            }
            TradingAction::PostAsk { agent_id, market_id, quantity, price } => {
                self.operations.execute_post_ask(*agent_id, market_id.clone(), *quantity, *price)
            }
        }
    }
}

impl Default for TradingDomain {
    fn default() -> Self {
        Self::new()
    }
}