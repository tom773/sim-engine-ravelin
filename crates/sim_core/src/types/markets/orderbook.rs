use crate::*;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trade {
    pub market_id: MarketId,
    pub buyer: AgentId,
    pub seller: AgentId,
    pub quantity: f64,
    pub price: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bid {
    pub agent_id: AgentId,
    pub price: f64,
    pub quantity: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
        Order::Bid(Bid {
            agent_id: Default::default(),
            price: 0.0,
            quantity: 0.0,
        })
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
        self.mid_price()
            .or_else(|| self.best_bid().map(|b| b.price))
            .or_else(|| self.best_ask().map(|a| a.price))
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

                if bid.quantity < 1e-6 { bid_idx += 1; }
                if ask.quantity < 1e-6 { ask_idx += 1; }
            } else {
                break;
            }
        }
        self.bids.retain(|b| b.quantity > 1e-6);
        self.asks.retain(|a| a.quantity > 1e-6);

        trades
    }

    pub fn depth_summary(&self) -> MarketDepthSummary {
        let best_bid = self.best_bid().map(|b| b.price);
        let best_ask = self.best_ask().map(|a| a.price);

        let bid_size_at_best = best_bid.map(|px| self.bids.iter().filter(|b| b.price == px).map(|b| b.quantity).sum()).unwrap_or(0.0);
        let ask_size_at_best = best_ask.map(|px| self.asks.iter().filter(|a| a.price == px).map(|a| a.quantity).sum()).unwrap_or(0.0);

        MarketDepthSummary {
            best_bid,
            best_ask,
            bid_size_at_best,
            ask_size_at_best,
            bid_levels: self.bids.len(),
            ask_levels: self.asks.len(),
        }
    }
}