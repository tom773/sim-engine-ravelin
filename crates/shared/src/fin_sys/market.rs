use serde::{Serialize, Deserialize};
use crate::*;
use std::collections::HashMap;


#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GoodId(pub String);

impl GoodId {
    pub fn new(category: &str, name: &str) -> Self {
        Self(format!("{}:{}", category, name))
    }
    
    pub fn generic() -> Self {
        Self("goods:generic".to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FinancialMarketId {
    SecuredOvernightFinancing, // Represents the market for interbank lending of reserves.
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
pub enum MarketId {
    Goods(GoodId),
    Financial(FinancialMarketId),
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trade {
    pub market_id: MarketId,
    pub buyer: AgentId,
    pub seller: AgentId,
    pub quantity: f64,
    pub price: f64, // For goods: price per unit. For loans: interest rate.
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
        Self {
            bids: Vec::new(),
            asks: Vec::new(),
        }
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
    order_book: OrderBook,
}

impl GoodsMarket {
    pub fn new(good_id: GoodId) -> Self {
        Self {
            good_id,
            order_book: OrderBook::new(),
        }
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
        let trades = Vec::new();
        trades
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialMarket {
    pub market_id: FinancialMarketId,
    order_book: OrderBook,
}

impl FinancialMarket {
    pub fn new(market_id: FinancialMarketId) -> Self {
        Self {
            market_id,
            order_book: OrderBook::new(),
        }
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
        let mut trades = Vec::new();
        trades
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Exchange {
    pub goods_markets: HashMap<GoodId, GoodsMarket>,
    pub financial_markets: HashMap<FinancialMarketId, FinancialMarket>,
}

impl Exchange {
    pub fn new() -> Self {
        let mut exchange = Self {
            goods_markets: HashMap::new(),
            financial_markets: HashMap::new(),
        };
        
        exchange.register_goods_market(GoodId::generic());
        
        exchange.register_financial_market(FinancialMarketId::SecuredOvernightFinancing);
        
        exchange
    }
    
    pub fn register_goods_market(&mut self, good_id: GoodId) {
        self.goods_markets.entry(good_id.clone())
            .or_insert_with(|| GoodsMarket::new(good_id));
    }
    
    pub fn register_financial_market(&mut self, market_id: FinancialMarketId) {
        self.financial_markets.entry(market_id.clone())
            .or_insert_with(|| FinancialMarket::new(market_id));
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
}