use sim_prelude::*;
use crate::TradingResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradingOperations;

impl TradingOperations {
    pub fn new() -> Self {
        Self
    }

    pub fn execute_post_bid(
        &self,
        agent_id: AgentId,
        market_id: MarketId,
        quantity: f64,
        price: f64,
    ) -> TradingResult {
        let effects = vec![StateEffect::Market(MarketEffect::PlaceOrderInBook {
            market_id,
            order: Order::Bid(Bid { agent_id, quantity, price }),
        })];

        TradingResult { success: true, effects, errors: vec![] }
    }

    pub fn execute_post_ask(
        &self,
        agent_id: AgentId,
        market_id: MarketId,
        quantity: f64,
        price: f64,
    ) -> TradingResult {
        let effects = vec![StateEffect::Market(MarketEffect::PlaceOrderInBook {
            market_id,
            order: Order::Ask(Ask { agent_id, quantity, price }),
        })];

        TradingResult { success: true, effects, errors: vec![] }
    }
}