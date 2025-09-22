use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MarketEffect {
    PlaceOrderInBook { market_id: Symbol, order: Order },
    ExecuteTrade(Trade),
    UpdatePrice { market_id: Symbol, new_price: f64 },
    ClearMarket { market_id: Symbol },

    UpdateLabourMarket { market_id: LabourMarketId, update: LabourMarketUpdate },
    ClearLabourMarketOrders { market_id: LabourMarketId, filled_applications: Vec<Uuid> },
    OpenDebtAuction { auction: DebtAuction },
    SubmitAuctionBid { auction_id: Uuid, bid: AuctionBid },
    CloseDebtAuction { auction_id: Uuid },
    PostOvernightFundingQuote(ONQuote),
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
            MarketEffect::PostOvernightFundingQuote { .. } => "PostOvernightFundingOrder",
        }
    }
}

impl StateEffectApplicator {
    pub fn apply_market_effect(state: &mut SimState, effect: &MarketEffect) -> Result<(), EffectError> {
        match effect {
            MarketEffect::PlaceOrderInBook { market_id, order } => {
                let exchange = &mut state.financial_system.exchange;

                let trades = if let Some(good_id) = exchange.symbol_to_good.get(market_id).copied() {
                    let mkt = exchange
                        .goods_market_mut(&good_id)
                        .ok_or_else(|| EffectError::MarketNotFound { market: market_id.to_string() })?;
                    mkt.book.submit_order(order.clone(), market_id)
                } else if let Some(inst_id) = exchange.symbol_to_inst.get(market_id).copied() {
                    let book = exchange
                        .financial_market_mut(&inst_id)
                        .ok_or_else(|| EffectError::MarketNotFound { market: market_id.to_string() })?;
                    book.submit_order(order.clone(), market_id)
                } else {
                    return Err(EffectError::MarketNotFound { market: market_id.to_string() });
                };

                if !trades.is_empty() {
                    let now = std::time::SystemTime::now();
                    exchange.recent_trades.extend(trades.iter().cloned());
                    for t in &trades {
                        exchange
                            .tape
                            .entry(t.market_id.clone())
                            .or_default()
                            .push(TimedTrade { ts: now, trade: t.clone() });
                    }
                }
                Ok(())
            }
            MarketEffect::ExecuteTrade(_trade) => Ok(()),
            MarketEffect::UpdatePrice { .. } => Ok(()),
            MarketEffect::ClearMarket { market_id } => {
                let exchange = &mut state.financial_system.exchange;
                let book = if let Some(good_id) = exchange.symbol_to_good.get(market_id).copied() {
                    exchange.goods_market_mut(&good_id).map(|m| &mut m.book)
                } else if let Some(inst_id) = exchange.symbol_to_inst.get(market_id).copied() {
                    exchange.financial_market_mut(&inst_id)
                } else if exchange.labour_to_symbol.values().any(|s| s == market_id) {
                    return Err(EffectError::InvalidState(
                        "ClearMarket is not applicable to labour markets.".to_string(),
                    ));
                } else {
                    None
                };

                let book = book.ok_or_else(|| EffectError::MarketNotFound { market: market_id.to_string() })?;
                book.bids.clear();
                book.asks.clear();
                Ok(())
            }
            MarketEffect::UpdateLabourMarket { market_id, update } => {
                let market = state
                    .financial_system
                    .exchange
                    .labour_market_mut(market_id)
                    .ok_or_else(|| EffectError::MarketNotFound { market: format!("{:?}", market_id) })?;
                match update {
                    LabourMarketUpdate::AddApplication(app) => {
                        if let Some(idx) = market.job_applications.iter().position(|a| a.consumer_id == app.consumer_id)
                        {
                            market.job_applications[idx] = app.clone();
                        } else {
                            market.job_applications.push(app.clone());
                        }
                    }
                    LabourMarketUpdate::AddOffer(offer) => market.job_offers.push(offer.clone()),
                }
                state.financial_system.exchange.invalidate_labour_offer_cache();
                Ok(())
            }
            MarketEffect::ClearLabourMarketOrders { market_id, filled_applications } => {
                let market = state
                    .financial_system
                    .exchange
                    .labour_market_mut(market_id)
                    .ok_or_else(|| EffectError::MarketNotFound { market: format!("{:?}", market_id) })?;
                let filled_ids: std::collections::HashSet<_> = filled_applications.iter().collect();
                market.job_applications.retain(|app| !filled_ids.contains(&&app.application_id));
                Ok(())
            }
            MarketEffect::OpenDebtAuction { auction } => {
                state.financial_system.exchange.open_auctions.insert(auction.auction_id, auction.clone());
                Ok(())
            }
            MarketEffect::SubmitAuctionBid { auction_id, bid } => {
                if let Some(auction) = state.financial_system.exchange.open_auctions.get_mut(auction_id) {
                    if auction.status == AuctionStatus::Open {
                        auction.bids.push(bid.clone());
                    }
                }
                Ok(())
            }
            MarketEffect::CloseDebtAuction { auction_id } => {
                if let Some(auction) = state.financial_system.exchange.open_auctions.get_mut(auction_id) {
                    auction.status = AuctionStatus::Closed;
                }
                Ok(())
            }
            MarketEffect::PostOvernightFundingQuote(quote) => {
                state.financial_system.funding_markets.post_quote(quote.clone());
                Ok(())
            }
        }
    }
}
