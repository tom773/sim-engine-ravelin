use crate::{prelude::*, types::money::Money};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use uuid::Uuid;

pub fn decimal_map_as_money<S>(m: &HashMap<Decimal, f64>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = s.serialize_map(Some(m.len()))?;
    for (k, v) in m {
        let money_key = Money(*k);
        map.serialize_entry(&money_key.to_f64(), v)?;
    }
    map.end()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct MarketDepthSummary {
    pub best_bid: Option<Money>,
    pub best_ask: Option<Money>,
    pub bid_size_at_best: f64,
    pub ask_size_at_best: f64,
    #[serde(serialize_with = "decimal_map_as_money")]
    pub bid_levels: HashMap<Decimal, f64>,
    #[serde(serialize_with = "decimal_map_as_money")]
    pub ask_levels: HashMap<Decimal, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketSnapshot {
    pub best_bid: Option<Money>,
    pub best_ask: Option<Money>,
    pub last_price: Option<Money>,
    pub spread: Option<Money>,
    pub depth: MarketDepthSummary,
    pub yield_bid: Option<f64>,
    pub yield_ask: Option<f64>,
    pub yield_mid: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trade {
    pub trade_id: Uuid,
    pub market_id: Symbol,
    pub buyer: AgentId,
    pub seller: AgentId,
    pub quantity: f64,
    pub price: Money,
}

#[derive(Clone, Debug, PartialEq, Copy, Serialize, Deserialize)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OrderType {
    Limit,
    Market,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub id: Uuid,
    pub agent_id: AgentId,
    pub side: Side,
    pub quantity: f64,
    pub price: Option<Money>,
    pub order_type: OrderType,
    pub market: Symbol,
}

impl Default for Order {
    fn default() -> Self {
        Order {
            id: Uuid::new_v4(),
            agent_id: Default::default(),
            side: Side::Bid,
            quantity: 0.0,
            price: None,
            order_type: OrderType::Market,
            market: Symbol("".to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct OrderBook {
    #[serde(with = "order_book_side_serde")]
    pub bids: BTreeMap<Decimal, Vec<Order>>,
    #[serde(with = "order_book_side_serde")]
    pub asks: BTreeMap<Decimal, Vec<Order>>,
    pub last_price: Option<Money>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn best_bid(&self) -> Option<Money> {
        self.bids.keys().next_back().map(|k| Money(*k))
    }

    pub fn best_ask(&self) -> Option<Money> {
        self.asks.keys().next().map(|k| Money(*k))
    }

    pub fn spread(&self) -> Option<Money> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }

    pub fn mid_price(&self) -> Option<Money> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(Money((bid.0 + ask.0) / dec!(2.0))),
            _ => None,
        }
    }

    pub fn representative_price(&self) -> Option<Money> {
        self.last_price.or_else(|| self.mid_price())
    }

    pub fn submit_order(&mut self, order: Order, market_id: &Symbol) -> Vec<Trade> {

        let mut trades = Vec::new();
        self.match_order(order, &mut trades, market_id);
        trades
    }

    fn match_order(&mut self, mut incoming_order: Order, trades: &mut Vec<Trade>, market_id: &Symbol) {
        match incoming_order.side {
            Side::Bid => self.match_bid(&mut incoming_order, trades, market_id),
            Side::Ask => self.match_ask(&mut incoming_order, trades, market_id),
        }

        if incoming_order.quantity > 1e-6 {
            match incoming_order.order_type {
                OrderType::Limit => self.add_to_book(incoming_order),
                OrderType::Market => {}
            }
        }
    }

    fn match_bid(&mut self, incoming_bid: &mut Order, trades: &mut Vec<Trade>, market_id: &Symbol) {
        let mut asks_to_remove = Vec::new();
        for (ask_price_decimal, ask_queue) in self.asks.iter_mut() {
            let ask_price = Money(*ask_price_decimal);

            let crosses = match incoming_bid.order_type {
                OrderType::Market => true,
                OrderType::Limit => incoming_bid.price.unwrap() >= ask_price,
            };

            if crosses {
                for resting_ask in ask_queue.iter_mut() {
                    if incoming_bid.quantity <= 1e-6 {
                        break;
                    }

                    let trade_qty = incoming_bid.quantity.min(resting_ask.quantity);
                    let trade_price = ask_price;

                    trades.push(Trade {
                        trade_id: Uuid::new_v4(),
                        market_id: market_id.clone(),
                        buyer: incoming_bid.agent_id,
                        seller: resting_ask.agent_id,
                        quantity: trade_qty,
                        price: trade_price,
                    });
                    self.last_price = Some(trade_price);

                    incoming_bid.quantity -= trade_qty;
                    resting_ask.quantity -= trade_qty;
                }

                ask_queue.retain(|o| o.quantity > 1e-6);
                if ask_queue.is_empty() {
                    asks_to_remove.push(*ask_price_decimal);
                }

                if incoming_bid.quantity <= 1e-6 {
                    break;
                }
            } else {
                break;
            }
        }
        for price in asks_to_remove {
            self.asks.remove(&price);
        }
    }

    fn match_ask(&mut self, incoming_ask: &mut Order, trades: &mut Vec<Trade>, market_id: &Symbol) {
        let mut bids_to_remove = Vec::new();
        for (bid_price_decimal, bid_queue) in self.bids.iter_mut().rev() {
            let bid_price = Money(*bid_price_decimal);

            let crosses = match incoming_ask.order_type {
                OrderType::Market => true,
                OrderType::Limit => incoming_ask.price.unwrap() <= bid_price,
            };

            if crosses {
                for resting_bid in bid_queue.iter_mut() {
                    if incoming_ask.quantity <= 1e-6 {
                        break;
                    }

                    let trade_qty = incoming_ask.quantity.min(resting_bid.quantity);
                    let trade_price = bid_price;

                    trades.push(Trade {
                        trade_id: Uuid::new_v4(),
                        market_id: market_id.clone(),
                        buyer: resting_bid.agent_id,
                        seller: incoming_ask.agent_id,
                        quantity: trade_qty,
                        price: trade_price,
                    });
                    self.last_price = Some(trade_price);

                    incoming_ask.quantity -= trade_qty;
                    resting_bid.quantity -= trade_qty;
                }

                bid_queue.retain(|o| o.quantity > 1e-6);
                if bid_queue.is_empty() {
                    bids_to_remove.push(*bid_price_decimal);
                }

                if incoming_ask.quantity <= 1e-6 {
                    break;
                }
            } else {
                break;
            }
        }
        for price in bids_to_remove {
            self.bids.remove(&price);
        }
    }

    fn add_to_book(&mut self, order: Order) {
        let price_key = order.price.unwrap().0;
        match order.side {
            Side::Bid => {
                self.bids.entry(price_key).or_default().push(order);
            }
            Side::Ask => {
                self.asks.entry(price_key).or_default().push(order);
            }
        }
    }

    pub fn depth_summary(&self) -> MarketDepthSummary {
        let mut bid_levels: HashMap<Decimal, f64> = HashMap::new();
        for (price_decimal, queue) in &self.bids {
            let total_qty = queue.iter().map(|o| o.quantity).sum();
            bid_levels.insert(*price_decimal, total_qty);
        }

        let mut ask_levels: HashMap<Decimal, f64> = HashMap::new();
        for (price_decimal, queue) in &self.asks {
            let total_qty = queue.iter().map(|o| o.quantity).sum();
            ask_levels.insert(*price_decimal, total_qty);
        }

        let best_bid = self.best_bid();
        let best_ask = self.best_ask();

        let bid_size_at_best = best_bid.and_then(|price| bid_levels.get(&price.0).copied()).unwrap_or(0.0);
        let ask_size_at_best = best_ask.and_then(|price| ask_levels.get(&price.0).copied()).unwrap_or(0.0);

        MarketDepthSummary { best_bid, best_ask, bid_size_at_best, ask_size_at_best, bid_levels, ask_levels }
    }

    pub fn clear_and_match(&mut self, market_id: &Symbol) -> Vec<Trade> {
        let trades = self.match_to_completion(market_id, |_, ask_price| ask_price);

        self.bids.clear();
        self.asks.clear();

        trades
    }

    pub fn match_to_completion(
        &mut self, market_id: &Symbol, choose_price: impl Fn(Money, Money) -> Money,
    ) -> Vec<Trade> {
        let mut trades = Vec::new();
        loop {
            let best_bid_price = match self.bids.keys().next_back() {
                Some(p) => *p,
                None => break,
            };
            let best_ask_price = match self.asks.keys().next() {
                Some(p) => *p,
                None => break,
            };

            if best_bid_price < best_ask_price {
                break;
            }

            let bid_queue = self.bids.get_mut(&best_bid_price).unwrap();
            let mut bid = bid_queue.remove(0);

            let ask_queue = self.asks.get_mut(&best_ask_price).unwrap();
            let mut ask = ask_queue.remove(0);

            let trade_qty = bid.quantity.min(ask.quantity);
            let trade_price = choose_price(Money(best_bid_price), Money(best_ask_price));

            trades.push(Trade {
                trade_id: Uuid::new_v4(),
                market_id: market_id.clone(),
                buyer: bid.agent_id,
                seller: ask.agent_id,
                quantity: trade_qty,
                price: trade_price,
            });
            self.last_price = Some(trade_price);

            bid.quantity -= trade_qty;
            ask.quantity -= trade_qty;

            if bid.quantity > 1e-9 {
                bid_queue.insert(0, bid);
            }
            if ask.quantity > 1e-9 {
                ask_queue.insert(0, ask);
            }

            if bid_queue.is_empty() {
                self.bids.remove(&best_bid_price);
            }
            if ask_queue.is_empty() {
                self.asks.remove(&best_ask_price);
            }
        }
        trades
    }

    pub fn validate_sell_order(
        &self, order: &Order, csd: &CentralSecuritiesDepository, instrument_id: &InstrumentId,
    ) -> Result<(), String> {
        if order.side == Side::Ask && csd.is_security(instrument_id) {
            let available = csd
                .custody_accounts
                .get(&order.agent_id)
                .and_then(|account| account.holdings.get(instrument_id))
                .map(|holding| holding.available)
                .unwrap_or(0.0);

            if available < order.quantity {
                return Err(format!(
                    "Insufficient securities in CSD: agent {} has {} available, order requires {}",
                    order.agent_id, available, order.quantity
                ));
            }
        }
        Ok(())
    }

    pub fn submit_order_with_validation(
        &mut self, order: Order, market_id: &Symbol, csd: &CentralSecuritiesDepository, instrument_id: &InstrumentId,
    ) -> Result<Vec<Trade>, String> {
        self.validate_sell_order(&order, csd, instrument_id)?;

        let mut trades = Vec::new();
        self.match_order(order, &mut trades, market_id);
        Ok(trades)
    }
}

mod order_book_side_serde {
    use super::*;
    use serde::{de::Deserializer, ser::Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S>(map: &BTreeMap<Decimal, Vec<Order>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let transformed_map: BTreeMap<String, &Vec<Order>> = map.iter().map(|(k, v)| (k.to_string(), v)).collect();
        transformed_map.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<Decimal, Vec<Order>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map = BTreeMap::<String, Vec<Order>>::deserialize(deserializer)?;
        map.into_iter()
            .map(|(k, v)| {
                let decimal = Decimal::from_str(&k).map_err(|_| serde::de::Error::custom("Invalid Decimal format"))?;
                Ok((decimal, v))
            })
            .collect()
    }
}
