use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr, collections::HashMap}; // Add HashMap
use thiserror::Error;
use serde_with::{serde_as, DisplayFromStr}; // Add serde_with
use crate::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy)]
pub enum Tenor {
    T2Y, T5Y, T10Y, T30Y,
}
impl Tenor {
    pub fn to_days(&self) -> u32 {
        match self {
            Tenor::T2Y => 730,
            Tenor::T5Y => 1825,
            Tenor::T10Y => 3650,
            Tenor::T30Y => 10950,
        }
    }
    pub fn add_to_date(&self, date: chrono::NaiveDate) -> chrono::NaiveDate {
        date + chrono::Duration::days(self.to_days() as i64)
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
    #[error("Invalid FinancialMarketId string format: {0}")]
    InvalidFormat(String),
    #[error("Failed to parse tenor: {0}")]
    ParseTenor(#[from] ParseTenorError),
    #[error("Failed to parse credit rating: {0}")]
    ParseRating(#[from] ParseCreditRatingError),
}

impl FromStr for FinancialMarketId {
    type Err = ParseFinancialMarketIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "SOFR" {
            return Ok(FinancialMarketId::SecuredOvernightFinancing);
        }
        if let Some(tenor_str) = s.strip_prefix("Treasury_") {
            let tenor = tenor_str.parse()?;
            return Ok(FinancialMarketId::Treasury { tenor });
        }
        if let Some(rating_str) = s.strip_prefix("CorpBond_") {
            let rating = rating_str.parse()?;
            return Ok(FinancialMarketId::CorporateBond { rating });
        }
        Err(ParseFinancialMarketIdError::InvalidFormat(s.to_string()))
    }
}


pub trait RatesMarket {
    fn price_to_daily_rate(&self, price: f64) -> f64;
    fn daily_rate_to_annual_bps(&self, daily_rate: f64) -> f64;
    fn annual_bps_to_daily_rate(&self, annual_bps: f64) -> f64;
}

impl RatesMarket for FinancialMarketId {
    fn price_to_daily_rate(&self, price: f64) -> f64 {
        if price <= 0.0 { return f64::INFINITY; }
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
pub enum LabourMarketId {
    Labour,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketId {
    Goods(GoodId),
    Financial(FinancialMarketId),
    Labour(LabourMarketId),
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
        self.bids.iter().max_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
    }

    pub fn best_ask(&self) -> Option<&Ask> {
        self.asks.iter().min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
    }

    pub fn clear_and_match(&mut self, market_id: &MarketId) -> Vec<Trade> {
        let mut trades = Vec::new();
        self.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
        self.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

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
}

impl fmt::Display for Tenor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
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

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Exchange {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub goods_markets: HashMap<GoodId, GoodsMarket>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub financial_markets: HashMap<FinancialMarketId, FinancialMarket>,
}

impl Exchange {
    pub fn register_goods_market(&mut self, good_id: GoodId, goods_registry: &GoodsRegistry) {
        let name = goods_registry.get_good_name(&good_id).unwrap_or("Unknown Good").to_string();
        self.goods_markets.entry(good_id).or_insert_with(|| GoodsMarket::new(good_id, name));
    }

    pub fn register_financial_market(&mut self, market_id: FinancialMarketId) {
        let name = match &market_id {
            FinancialMarketId::SecuredOvernightFinancing => "Secured Overnight Financing".to_string(),
            FinancialMarketId::Treasury { tenor } => format!("Treasury {}", tenor),
            FinancialMarketId::CorporateBond { rating } => format!("Corporate Bond {:?}", rating),
        };
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
        for (id, market) in self.goods_markets.iter_mut() {
            all_trades.extend(market.order_book.clear_and_match(&MarketId::Goods(*id)));
        }
        for (id, market) in self.financial_markets.iter_mut() {
            all_trades.extend(market.order_book.clear_and_match(&MarketId::Financial(id.clone())));
        }
        all_trades
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoodsMarket {
    pub good_id: GoodId,
    pub name: String,
    pub order_book: OrderBook,
}

impl GoodsMarket {
    pub fn new(good_id: GoodId, name: String) -> Self {
        Self { good_id, name, order_book: OrderBook::new() }
    }

    pub fn best_ask(&self) -> Option<&Ask> {
        self.order_book.asks.iter().min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialMarket {
    pub market_id: FinancialMarketId,
    pub name: String,
    pub order_book: OrderBook,
}

impl FinancialMarket {
    pub fn new(market_id: FinancialMarketId, name: String) -> Self {
        Self { market_id, name, order_book: OrderBook::new() }
    }
}
