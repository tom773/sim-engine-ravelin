use super::*;
use crate::*;
use ordered_float::NotNan;
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn notnan_map_as_f64<S>(m: &HashMap<NotNan<f64>, f64>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = s.serialize_map(Some(m.len()))?;
    for (k, v) in m {
        map.serialize_entry(&k.into_inner(), v)?;
    }
    map.end()
}
#[inline]
fn k(px: f64) -> NotNan<f64> {
    NotNan::new(px).expect("price was NaN")
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trade {
    pub market_id: MarketId,
    pub buyer: AgentId,
    pub seller: AgentId,
    pub quantity: f64,
    pub price: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Bid {
    pub agent_id: AgentId,
    pub price: f64,
    pub quantity: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Ask {
    pub agent_id: AgentId,
    pub price: f64,
    pub quantity: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Order {
    Bid(Bid),
    Ask(Ask),
}

impl Default for Order {
    fn default() -> Self {
        Order::Bid(Bid { agent_id: Default::default(), price: 0.0, quantity: 0.0 })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrderBook {
    pub bids: Vec<Bid>,
    pub asks: Vec<Ask>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self { bids: Vec::new(), asks: Vec::new() }
    }

    pub fn best_bid(&self) -> Option<&Bid> {
        self.bids.iter().max_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn best_ask(&self) -> Option<&Ask> {
        self.asks.iter().min_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn spread(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask.price - bid.price),
            _ => None,
        }
    }

    pub fn mid_price(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid.price + ask.price) / 2.0),
            _ => None,
        }
    }

    pub fn representative_price(&self) -> Option<f64> {
        self.mid_price().or_else(|| self.best_bid().map(|b| b.price)).or_else(|| self.best_ask().map(|a| a.price))
    }

    pub fn clear_and_match(&mut self, market_id: &MarketId) -> Vec<Trade> {
        let mut trades = Vec::new();
        self.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));
        self.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));

        let mut bid_idx = 0;
        let mut ask_idx = 0;

        while bid_idx < self.bids.len() && ask_idx < self.asks.len() {
            let bid = &mut self.bids[bid_idx];
            let ask = &mut self.asks[ask_idx];

            if bid.price >= ask.price {
                let trade_qty = bid.quantity.min(ask.quantity);
                let trade_price = (bid.price + ask.price) / 2.0;

                trades.push(Trade {
                    market_id: market_id.clone(),
                    buyer: bid.agent_id,
                    seller: ask.agent_id,
                    quantity: trade_qty,
                    price: trade_price,
                });

                bid.quantity -= trade_qty;
                ask.quantity -= trade_qty;

                if bid.quantity < 1e-6 {
                    bid_idx += 1;
                }
                if ask.quantity < 1e-6 {
                    ask_idx += 1;
                }
            } else {
                break;
            }
        }
        self.bids.retain(|b| b.quantity > 1e-6);
        self.asks.retain(|a| a.quantity > 1e-6);

        trades
    }

    pub fn depth_summary(&self) -> MarketDepthSummary {
        // aggregate into hashable keys
        let mut bids_agg: HashMap<NotNan<f64>, f64> = HashMap::new();
        for b in &self.bids {
            *bids_agg.entry(k(b.price)).or_default() += b.quantity;
        }

        let mut asks_agg: HashMap<NotNan<f64>, f64> = HashMap::new();
        for a in &self.asks {
            *asks_agg.entry(k(a.price)).or_default() += a.quantity;
        }

        let best_bid = self.best_bid().map(|b| b.price);
        let best_ask = self.best_ask().map(|a| a.price);

        // keep NotNan maps; use k(px) when looking up best sizes
        let bid_levels: HashMap<NotNan<f64>, f64> = bids_agg;
        let ask_levels: HashMap<NotNan<f64>, f64> = asks_agg;

        let bid_size_at_best = best_bid.and_then(|px| bid_levels.get(&k(px)).copied()).unwrap_or(0.0);
        let ask_size_at_best = best_ask.and_then(|px| ask_levels.get(&k(px)).copied()).unwrap_or(0.0);

        MarketDepthSummary { best_bid, best_ask, bid_size_at_best, ask_size_at_best, bid_levels, ask_levels }
    }
}
