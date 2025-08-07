use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MarketEffect {
    PlaceOrderInBook { market_id: MarketId, order: Order },
    ExecuteTrade(Trade),
    UpdatePrice { market_id: MarketId, new_price: f64 },
    ClearMarket { market_id: MarketId },
}

impl MarketEffect {
    pub fn name(&self) -> &'static str {
        match self {
            MarketEffect::PlaceOrderInBook { .. } => "PlaceOrderInBook",
            MarketEffect::ExecuteTrade(_) => "ExecuteTrade",
            MarketEffect::UpdatePrice { .. } => "UpdatePrice",
            MarketEffect::ClearMarket { .. } => "ClearMarket",
        }
    }
}