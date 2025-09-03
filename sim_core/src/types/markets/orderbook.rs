use crate::{prelude::*, types::money::Money};
use ordered_float::NotNan;
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketId {
    Financial(InstrumentId), // Uniquely identifies a financial market via one of its instruments
    Goods(GoodId),
    Labour(LabourMarketId),
}

impl std::fmt::Display for MarketId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketId::Financial(id) => write!(f, "Financial:{}", id),
            MarketId::Goods(id) => write!(f, "Goods:{}", id),
            MarketId::Labour(id) => write!(f, "Labour:{}", id),
        }
    }
}

impl std::str::FromStr for MarketId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = s.strip_prefix("Financial:") {
            return rest
                .parse::<InstrumentId>()
                .map(MarketId::Financial)
                .map_err(|e| e.to_string());
        }
        if let Some(rest) = s.strip_prefix("Goods:") {
            return rest.parse::<GoodId>().map(MarketId::Goods).map_err(|e| e.to_string());
        }
        if let Some(rest) = s.strip_prefix("Labour:") {
            return rest
                .parse::<LabourMarketId>()
                .map(MarketId::Labour)
                .map_err(|e| e.to_string());
        }
        Err(format!("Unrecognized MarketId format: {}", s))
    }
}

// Helper function to convert Money to NotNan<f64> for BTreeMap keys
#[inline]
fn money_to_key(money: Money) -> NotNan<f64> {
    NotNan::new(money.to_f64()).expect("Money value was NaN")
}

// Helper function to convert NotNan<f64> back to Money
#[inline]
fn key_to_money(key: NotNan<f64>) -> Money {
    Money::from_f64(key.into_inner()).unwrap_or(Money::ZERO)
}

pub fn notnan_map_as_money<S>(m: &HashMap<NotNan<f64>, f64>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = s.serialize_map(Some(m.len()))?;
    for (k, v) in m {
        let money_key = key_to_money(*k);
        map.serialize_entry(&money_key.to_f64(), v)?;
    }
    map.end()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarketDepthSummary {
    pub best_bid: Option<Money>,
    pub best_ask: Option<Money>,
    pub bid_size_at_best: f64,
    pub ask_size_at_best: f64,
    #[serde(serialize_with = "notnan_map_as_money")]
    pub bid_levels: HashMap<NotNan<f64>, f64>, // Key represents Money as f64, value is quantity
    #[serde(serialize_with = "notnan_map_as_money")]
    pub ask_levels: HashMap<NotNan<f64>, f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarketSnapshot {
    pub best_bid: Option<Money>,
    pub best_ask: Option<Money>,
    pub mid_price: Option<Money>,
    pub last_price: Option<Money>,
    pub spread: Option<Money>,
    pub volume_24h: f64,
    pub depth: MarketDepthSummary,
    pub best_bid_yield: Option<f64>,
    pub best_ask_yield: Option<f64>,
    pub mid_yield: Option<f64>,
    pub last_yield: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trade {
    pub trade_id: Uuid,
    pub market_id: MarketId,
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
    pub quantity: f64,           // Remaining quantity
    pub price: Option<Money>,    // None for Market orders, Some for Limit orders
    pub order_type: OrderType,
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
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct OrderBook {
    #[serde(with = "order_book_side_serde")]
    pub bids: BTreeMap<NotNan<f64>, Vec<Order>>, // Key is Money converted to NotNan<f64>
    #[serde(with = "order_book_side_serde")]
    pub asks: BTreeMap<NotNan<f64>, Vec<Order>>,
    pub last_price: Option<Money>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn best_bid(&self) -> Option<Money> {
        self.bids.keys().next_back().map(|k| key_to_money(*k))
    }

    pub fn best_ask(&self) -> Option<Money> {
        self.asks.keys().next().map(|k| key_to_money(*k))
    }

    pub fn spread(&self) -> Option<Money> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }

    pub fn mid_price(&self) -> Option<Money> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / 2.0),
            _ => None,
        }
    }

    pub fn representative_price(&self) -> Option<Money> {
        self.last_price.or_else(|| self.mid_price())
    }

    pub fn submit_order(&mut self, order: Order, market_id: &MarketId) -> Vec<Trade> {
        let mut trades = Vec::new();
        self.match_order(order, &mut trades, market_id);
        trades
    }

    fn match_order(
        &mut self,
        mut incoming_order: Order,
        trades: &mut Vec<Trade>,
        market_id: &MarketId,
    ) {
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

    fn match_bid(
        &mut self,
        incoming_bid: &mut Order,
        trades: &mut Vec<Trade>,
        market_id: &MarketId,
    ) {
        let mut asks_to_remove = Vec::new();
        for (ask_price_nn, ask_queue) in self.asks.iter_mut() {
            let ask_price = key_to_money(*ask_price_nn);

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
                    let trade_price = ask_price; // Trades execute at the resting order's price

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
                    asks_to_remove.push(*ask_price_nn);
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

    fn match_ask(
        &mut self,
        incoming_ask: &mut Order,
        trades: &mut Vec<Trade>,
        market_id: &MarketId,
    ) {
        let mut bids_to_remove = Vec::new();
        for (bid_price_nn, bid_queue) in self.bids.iter_mut().rev() {
            let bid_price = key_to_money(*bid_price_nn);

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
                    bids_to_remove.push(*bid_price_nn);
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
        let price_key = money_to_key(order.price.unwrap());
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
        let mut bid_levels: HashMap<NotNan<f64>, f64> = HashMap::new();
        for (price_nn, queue) in &self.bids {
            let total_qty = queue.iter().map(|o| o.quantity).sum();
            bid_levels.insert(*price_nn, total_qty);
        }

        let mut ask_levels: HashMap<NotNan<f64>, f64> = HashMap::new();
        for (price_nn, queue) in &self.asks {
            let total_qty = queue.iter().map(|o| o.quantity).sum();
            ask_levels.insert(*price_nn, total_qty);
        }

        let best_bid = self.best_bid();
        let best_ask = self.best_ask();

        let bid_size_at_best = best_bid
            .and_then(|price| bid_levels.get(&money_to_key(price)).copied())
            .unwrap_or(0.0);
        let ask_size_at_best = best_ask
            .and_then(|price| ask_levels.get(&money_to_key(price)).copied())
            .unwrap_or(0.0);

        MarketDepthSummary {
            best_bid,
            best_ask,
            bid_size_at_best,
            ask_size_at_best,
            bid_levels,
            ask_levels,
        }
    }

    pub fn clear_and_match(&mut self, market_id: &MarketId) -> Vec<Trade> {
        let mut trades = Vec::new();
        loop {
            let (best_bid_opt, best_ask_opt) = (self.best_bid(), self.best_ask());

            if let (Some(bid_price), Some(ask_price)) = (best_bid_opt, best_ask_opt) {
                if bid_price < ask_price {
                    break;
                }

                let bid_price_nn = money_to_key(bid_price);
                let mut aggressing_bid = {
                    let bid_queue = self.bids.get_mut(&bid_price_nn).unwrap();
                    let order = bid_queue.remove(0); // FIFO
                    if bid_queue.is_empty() {
                        self.bids.remove(&bid_price_nn);
                    }
                    order
                };

                self.match_bid(&mut aggressing_bid, &mut trades, market_id);

                if aggressing_bid.quantity > 1e-6 {
                    self.add_to_book(aggressing_bid);
                }
            } else {
                break;
            }
        }
        trades
    }
}

mod order_book_side_serde {
    use super::*;
    use serde::{de::Deserializer, ser::Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S>(
        map: &BTreeMap<NotNan<f64>, Vec<Order>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let transformed_map: BTreeMap<String, &Vec<Order>> = map
            .iter()
            .map(|(k, v)| (key_to_money(*k).to_string(), v))
            .collect();
        transformed_map.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<BTreeMap<NotNan<f64>, Vec<Order>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map = BTreeMap::<String, Vec<Order>>::deserialize(deserializer)?;
        map.into_iter()
            .map(|(k, v)| {
                let money = Money::from_str(&k)
                    .ok_or_else(|| serde::de::Error::custom("Invalid Money format"))?;
                Ok((money_to_key(money), v))
            })
            .collect()
    }
}

impl From<OrderBook> for Market {
    fn from(book: OrderBook) -> Self {
        Self { book }
    }
}