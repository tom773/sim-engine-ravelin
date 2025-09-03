use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MarketEffect {
    PlaceOrderInBook {
        market_id: MarketId,
        order: Order,
    },
    ExecuteTrade(Trade),
    UpdatePrice {
        market_id: MarketId,
        new_price: f64,
    },
    ClearMarket {
        market_id: MarketId,
    },

    UpdateLabourMarket {
        market_id: LabourMarketId,
        update: LabourMarketUpdate,
    },
    ClearLabourMarketOrders {
        market_id: LabourMarketId,
        filled_applications: Vec<Uuid>,
    },
    OpenDebtAuction {
        auction: DebtAuction,
    },
    SubmitAuctionBid {
        auction_id: Uuid,
        bid: AuctionBid,
    },
    CloseDebtAuction {
        auction_id: Uuid,
    },
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
            MarketEffect::OpenDebtAuction { .. } => "OpenDebtAuction",
            MarketEffect::SubmitAuctionBid { .. } => "SubmitAuctionBid",
            MarketEffect::CloseDebtAuction { .. } => "CloseDebtAuction",
        }
    }
}

impl StateEffectApplicator {
    pub fn apply_market_effect(
        state: &mut SimState,
        effect: &MarketEffect,
    ) -> Result<(), EffectError> {
        match effect {
            MarketEffect::PlaceOrderInBook { market_id, order } => {
                match market_id {
                    MarketId::Goods(id) => {
                        let market = state
                            .financial_system
                            .exchange
                            .goods_market_mut(id)
                            .ok_or_else(|| EffectError::MarketNotFound {
                                market: format!("{:?}", market_id),
                            })?;
                        market.book.submit_order(order.clone(), market_id);
                    }
                    MarketId::Financial(inst_id) => {
                        let order_book = state
                            .financial_system
                            .exchange
                            .financial_market_mut(inst_id)
                            .ok_or_else(|| EffectError::MarketNotFound {
                                market: format!("{:?}", inst_id),
                            })?;

                        order_book.submit_order(order.clone(), market_id);
                    }
                    MarketId::Labour(_) => {
                        return Err(EffectError::InvalidState(
                            "Cannot place direct orders in a labour market".to_string(),
                        ));
                    }
                }
                Ok(())
            }
            MarketEffect::ExecuteTrade(_trade) => Ok(()),
            MarketEffect::UpdatePrice { .. } => Ok(()),
            MarketEffect::ClearMarket { market_id } => {
                let book = match market_id {
                    MarketId::Goods(id) => state
                        .financial_system
                        .exchange
                        .goods_market_mut(id)
                        .map(|m| &mut m.book),
                    MarketId::Financial(id) => {
                        state.financial_system.exchange.financial_market_mut(id)
                    }
                    MarketId::Labour(_) => {
                        return Err(EffectError::InvalidState(
                            "ClearMarket is not applicable to labour markets.".to_string(),
                        ));
                    }
                }
                .ok_or_else(|| EffectError::MarketNotFound {
                    market: format!("{:?}", market_id),
                })?;
                book.bids.clear();
                book.asks.clear();
                Ok(())
            }
            MarketEffect::UpdateLabourMarket { market_id, update } => {
                let market = state
                    .financial_system
                    .exchange
                    .labour_market_mut(market_id)
                    .ok_or_else(|| EffectError::MarketNotFound {
                        market: format!("{:?}", market_id),
                    })?;
                match update {
                    LabourMarketUpdate::AddApplication(app) => {
                        market.job_applications.push(app.clone())
                    }
                    LabourMarketUpdate::AddOffer(offer) => market.job_offers.push(offer.clone()),
                }
                Ok(())
            }
            MarketEffect::ClearLabourMarketOrders {
                market_id,
                filled_applications,
            } => {
                let market = state
                    .financial_system
                    .exchange
                    .labour_market_mut(market_id)
                    .ok_or_else(|| EffectError::MarketNotFound {
                        market: format!("{:?}", market_id),
                    })?;
                let filled_ids: std::collections::HashSet<_> = filled_applications.iter().collect();
                market
                    .job_applications
                    .retain(|app| !filled_ids.contains(&&app.application_id));
                Ok(())
            }
            MarketEffect::OpenDebtAuction { auction } => {
                state
                    .financial_system
                    .exchange
                    .open_auctions
                    .insert(auction.auction_id, auction.clone());
                Ok(())
            }
            MarketEffect::SubmitAuctionBid { auction_id, bid } => {
                if let Some(auction) = state
                    .financial_system
                    .exchange
                    .open_auctions
                    .get_mut(auction_id)
                {
                    if auction.status == AuctionStatus::Open {
                        auction.bids.push(bid.clone());
                    }
                }
                Ok(())
            }
            MarketEffect::CloseDebtAuction { auction_id } => {
                if let Some(auction) = state
                    .financial_system
                    .exchange
                    .open_auctions
                    .get_mut(auction_id)
                {
                    auction.status = AuctionStatus::Closed;
                }
                Ok(())
            }
        }
    }
}
