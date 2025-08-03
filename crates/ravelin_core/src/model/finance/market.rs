use crate::*;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::HashMap;
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy)]
pub enum Tenor {
    T2Y,
    T5Y,
    T10Y,
    T30Y,
}
impl fmt::Display for Tenor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Tenor {
    pub fn name(&self) -> &'static str {
        match self {
            Tenor::T2Y => "US 2Y Note",
            Tenor::T5Y => "US 5Y Note",
            Tenor::T10Y => "US 10Y Bond",
            Tenor::T30Y => "US 30Y Bond",
        }
    }
}
#[derive(Debug, Error)]
#[error("Invalid Tenor string: {0}")]
pub struct ParseTenorError(String);
impl FromStr for Tenor {
    type Err = ParseTenorError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "T2Y" => Ok(Tenor::T2Y),
            "T5Y" => Ok(Tenor::T5Y),
            "T10Y" => Ok(Tenor::T10Y),
            "T30Y" => Ok(Tenor::T30Y),
            _ => Err(ParseTenorError(s.to_string())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FinancialMarketId {
    SecuredOvernightFinancing,
    Treasury { tenor: Tenor },
    CorporateBond { rating: CreditRating },
}

impl fmt::Display for FinancialMarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FinancialMarketId::SecuredOvernightFinancing => write!(f, "SOFR"),
            FinancialMarketId::Treasury { tenor } => write!(f, "Treasury_{}", tenor),
            FinancialMarketId::CorporateBond { rating } => write!(f, "CorpBond_{}", rating),
        }
    }
}

#[derive(Debug, Error)]
pub enum ParseFinancialMarketIdError {
    #[error("Unknown format for FinancialMarketId: {0}")]
    UnknownFormat(String),
    #[error("Invalid tenor part: {0}")]
    InvalidTenor(#[from] ParseTenorError),
    #[error("Invalid rating part: {0}")]
    InvalidRating(#[from] ParseCreditRatingError),
}

impl FromStr for FinancialMarketId {
    type Err = ParseFinancialMarketIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "SOFR" {
            return Ok(FinancialMarketId::SecuredOvernightFinancing);
        }
        if let Some(tenor_str) = s.strip_prefix("Treasury_") {
            let tenor = tenor_str.parse::<Tenor>()?;
            return Ok(FinancialMarketId::Treasury { tenor });
        }
        if let Some(rating_str) = s.strip_prefix("CorpBond_") {
            let rating = rating_str.parse::<CreditRating>()?;
            return Ok(FinancialMarketId::CorporateBond { rating });
        }
        Err(ParseFinancialMarketIdError::UnknownFormat(s.to_string()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LabourMarketId {
    Labour,
}

impl FinancialMarketId {
    pub fn name(&self) -> String {
        match self {
            FinancialMarketId::SecuredOvernightFinancing => "Secured Overnight Financing".to_string(),
            FinancialMarketId::Treasury { tenor } => tenor.name().to_string(),
            FinancialMarketId::CorporateBond { rating } => format!("Corporate Bond ({})", rating.name()),
        }
    }
}

impl LabourMarketId {
    pub fn name(&self) -> &'static str {
        match self {
            LabourMarketId::Labour => "Labour",
        }
    }
}
pub trait RatesMarket {
    fn price_to_daily_rate(&self, price: f64) -> f64;
    fn daily_rate_to_annual_bps(&self, daily_rate: f64) -> f64;
    fn annual_bps_to_daily_rate(&self, annual_bps: f64) -> f64;
}
impl RatesMarket for FinancialMarketId {
    fn price_to_daily_rate(&self, price: f64) -> f64 {
        if price <= 0.0 {
            return f64::INFINITY;
        }
        (1.0 / price) - 1.0
    }
    fn daily_rate_to_annual_bps(&self, daily_rate: f64) -> f64 {
        daily_rate * 360.0 * 10000.0
    }
    fn annual_bps_to_daily_rate(&self, annual_bps: f64) -> f64 {
        annual_bps / 10000.0 / 360.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketId {
    Goods(GoodId),
    Financial(FinancialMarketId),
    Labour(LabourMarketId),
}
impl MarketId {
    pub fn name(&self, registry: &GoodsRegistry) -> String {
        match self {
            MarketId::Goods(good_id) => registry.get_good_name(good_id).unwrap_or("Unknown Good").to_string(),
            MarketId::Financial(financial_id) => financial_id.name(),
            MarketId::Labour(labour_id) => labour_id.name().to_string(),
        }
    }
}
impl Default for MarketId {
    fn default() -> Self {
        MarketId::Goods(GoodId::default())
    }
}
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
pub struct OrderBook {
    pub bids: Vec<Bid>,
    pub asks: Vec<Ask>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self { bids: Vec::new(), asks: Vec::new() }
    }
    pub fn sort_orders(&mut self) {
        self.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
        self.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
    }
}

pub trait Market: Send + Sync {
    fn quote(&self) -> Option<f64>;
    fn post_ask(&mut self, seller: AgentId, qty: f64, price: f64);
    fn post_bid(&mut self, buyer: AgentId, qty: f64, price: f64);
    fn match_orders(&mut self) -> Vec<Trade>;
    fn best_bid(&self) -> Option<&Bid>;
    fn best_ask(&self) -> Option<&Ask>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoodsMarket {
    pub good_id: GoodId,
    pub name: String,
    order_book: OrderBook,
}
impl GoodsMarket {
    pub fn new(good_id: GoodId, name: String) -> Self {
        Self { good_id, name, order_book: OrderBook::new() }
    }
}
impl Market for GoodsMarket {
    fn quote(&self) -> Option<f64> {
        self.order_book.asks.first().map(|ask| ask.price)
    }
    fn best_bid(&self) -> Option<&Bid> {
        self.order_book.bids.first()
    }
    fn best_ask(&self) -> Option<&Ask> {
        self.order_book.asks.first()
    }
    fn post_ask(&mut self, seller: AgentId, qty: f64, price: f64) {
        self.order_book.asks.push(Ask { agent_id: seller, price, quantity: qty });
    }
    fn post_bid(&mut self, buyer: AgentId, qty: f64, price: f64) {
        self.order_book.bids.push(Bid { agent_id: buyer, price, quantity: qty });
    }
    fn match_orders(&mut self) -> Vec<Trade> {
        double_auction(&mut self.order_book, &MarketId::Goods(self.good_id))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialMarket {
    pub market_id: FinancialMarketId,
    pub name: String,
    order_book: OrderBook,
}
impl FinancialMarket {
    pub fn new(market_id: FinancialMarketId, name: String) -> Self {
        Self { market_id, name, order_book: OrderBook::new() }
    }
}
impl Market for FinancialMarket {
    fn quote(&self) -> Option<f64> {
        self.order_book.asks.first().map(|ask| ask.price)
    }
    fn best_bid(&self) -> Option<&Bid> {
        self.order_book.bids.first()
    }
    fn best_ask(&self) -> Option<&Ask> {
        self.order_book.asks.first()
    }
    fn post_ask(&mut self, seller: AgentId, qty: f64, price: f64) {
        self.order_book.asks.push(Ask { agent_id: seller, price, quantity: qty });
    }
    fn post_bid(&mut self, buyer: AgentId, qty: f64, price: f64) {
        self.order_book.bids.push(Bid { agent_id: buyer, price, quantity: qty });
    }
    fn match_orders(&mut self) -> Vec<Trade> {
        double_auction(&mut self.order_book, &MarketId::Financial(self.market_id.clone()))
    }
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
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Exchange {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub goods_markets: HashMap<GoodId, GoodsMarket>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub financial_markets: HashMap<FinancialMarketId, FinancialMarket>,
}
impl Exchange {
    pub fn new() -> Self {
        let mut exchange = Self { goods_markets: HashMap::new(), financial_markets: HashMap::new() };
        exchange.register_financial_market(FinancialMarketId::SecuredOvernightFinancing);
        exchange
    }
    pub fn register_goods_market(&mut self, good_id: GoodId) {
        let name = CATALOGUE.get_good_name(&good_id).unwrap_or("Unknown Good").to_string();
        self.goods_markets.entry(good_id).or_insert_with(|| GoodsMarket::new(good_id, name));
    }
    pub fn register_financial_market(&mut self, market_id: FinancialMarketId) {
        let name = market_id.name().to_string();
        self.financial_markets.entry(market_id.clone()).or_insert_with(|| FinancialMarket::new(market_id, name));
    }
    pub fn goods_market(&self, good_id: &GoodId) -> Option<&GoodsMarket> {
        self.goods_markets.get(good_id)
    }
    pub fn goods_market_mut(&mut self, good_id: &GoodId) -> Option<&mut GoodsMarket> {
        self.goods_markets.get_mut(good_id)
    }
    pub fn financial_market(&self, market_id: &FinancialMarketId) -> Option<&FinancialMarket> {
        self.financial_markets.get(market_id)
    }
    pub fn financial_market_mut(&mut self, market_id: &FinancialMarketId) -> Option<&mut FinancialMarket> {
        self.financial_markets.get_mut(market_id)
    }
    pub fn clear_markets(&mut self) -> Vec<Trade> {
        let mut all_trades = Vec::new();
        for market in self.goods_markets.values_mut() {
            all_trades.extend(market.match_orders());
        }
        for market in self.financial_markets.values_mut() {
            all_trades.extend(market.match_orders());
        }
        all_trades
    }
}

fn double_auction(order_book: &mut OrderBook, market_id: &MarketId) -> Vec<Trade> {
    let mut trades = Vec::new();
    order_book.sort_orders();
    let mut bid_idx = 0;
    let mut ask_idx = 0;
    while bid_idx < order_book.bids.len() && ask_idx < order_book.asks.len() {
        let bid = &mut order_book.bids[bid_idx];
        let ask = &mut order_book.asks[ask_idx];
        if bid.price >= ask.price {
            let trade_qty = bid.quantity.min(ask.quantity);
            let trade_price = (bid.price + ask.price) / 2.0;
            trades.push(Trade {
                market_id: market_id.clone(),
                buyer: bid.agent_id.clone(),
                seller: ask.agent_id.clone(),
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
    order_book.bids.drain(0..bid_idx);
    order_book.asks.drain(0..ask_idx);
    trades
}
