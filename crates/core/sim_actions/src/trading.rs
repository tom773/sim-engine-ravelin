use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TradingAction {
    PostBid { agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64 },
    PostAsk { agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64 },
}

impl TradingAction {
    pub fn name(&self) -> &'static str {
        match self {
            TradingAction::PostBid { .. } => "PostBid",
            TradingAction::PostAsk { .. } => "PostAsk",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            TradingAction::PostBid { agent_id, .. } => *agent_id,
            TradingAction::PostAsk { agent_id, .. } => *agent_id,
        }
    }
}