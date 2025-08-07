use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid; // Import Uuid

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MarketEffect {
    PlaceOrderInBook { market_id: MarketId, order: Order },
    ExecuteTrade(Trade),
    UpdatePrice { market_id: MarketId, new_price: f64 },
    ClearMarket { market_id: MarketId },

    UpdateLabourMarket {
        market_id: LabourMarketId,
        update: LabourMarketUpdate,
    },
    ClearLabourMarketOrders {
        market_id: LabourMarketId,
        filled_applications: Vec<Uuid>,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LabourMarketUpdate {
    AddApplication(JobApplication),
    AddOffer(JobOffer),
}

impl MarketEffect {
    pub fn name(&self) -> &'static str {
        match self {
            MarketEffect::PlaceOrderInBook { .. } => "PlaceOrderInBook",
            MarketEffect::ExecuteTrade(_) => "ExecuteTrade",
            MarketEffect::UpdatePrice { .. } => "UpdatePrice",
            MarketEffect::ClearMarket { .. } => "ClearMarket",
            MarketEffect::UpdateLabourMarket { .. } => "UpdateLabourMarket",
            MarketEffect::ClearLabourMarketOrders { .. } => "ClearLabourMarketOrders",
        }
    }
}