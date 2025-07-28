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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trade {
    pub buyer: AgentId,
    pub seller: AgentId,
    pub asset: AssetId,
    pub quantity: f64,
    pub price: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetId {
    Good(GoodId),
    Instrument(InstrumentId),
}

pub trait Market: Send + Sync {
    type AssetId;
    fn quote(&self, asset: &Self::AssetId) -> Option<f64>;
    fn post_ask(&mut self, seller: AgentId, asset: Self::AssetId, qty: f64, price: f64);
    fn post_bid(&mut self, buyer: AgentId, asset: Self::AssetId, qty: f64, price: f64);
    fn match_orders(&mut self) -> Vec<Trade>;
    fn best_bid(&self, asset: &Self::AssetId) -> Option<&Bid>;
    fn best_ask(&self, asset: &Self::AssetId) -> Option<&Ask>;
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


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoodsMarket {
    good_id: GoodId,
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
    type AssetId = GoodId;
    
    fn quote(&self, asset: &Self::AssetId) -> Option<f64> {
        if asset == &self.good_id {
            self.order_book.asks.first().map(|ask| ask.price)
        } else {
            None
        }
    }
    fn best_bid(&self, asset: &Self::AssetId) -> Option<&Bid> { if asset == &self.good_id { self.order_book.bids.first() } else { None } }
    fn best_ask(&self, asset: &Self::AssetId) -> Option<&Ask> { if asset == &self.good_id { self.order_book.asks.first() } else { None } }
    
    fn post_ask(&mut self, seller: AgentId, asset: Self::AssetId, qty: f64, price: f64) {
        if asset == self.good_id {
            self.order_book.asks.push(Ask {
                agent_id: seller,
                price,
                quantity: qty,
            });
        }
    }
    
    fn post_bid(&mut self, buyer: AgentId, asset: Self::AssetId, qty: f64, price: f64) {
        if asset == self.good_id {
            self.order_book.bids.push(Bid {
                agent_id: buyer,
                price,
                quantity: qty,
            });
        }
    }
    fn match_orders(&mut self) -> Vec<Trade> {
        let mut trades = Vec::new();
        trades
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialMarket {
    instrument_id: InstrumentId,
    order_book: OrderBook,
}

impl FinancialMarket {
    pub fn new(instrument_id: InstrumentId) -> Self {
        Self {
            instrument_id,
            order_book: OrderBook::new(),
        }
    }
}

impl Market for FinancialMarket {
    type AssetId = InstrumentId;
    
    fn quote(&self, asset: &Self::AssetId) -> Option<f64> {
        if asset == &self.instrument_id {
            self.order_book.asks.first().map(|ask| ask.price)
        } else {
            None
        }
    }
    fn best_bid(&self, asset: &Self::AssetId) -> Option<&Bid> { if asset == &self.instrument_id { self.order_book.bids.first() } else { None } }
    fn best_ask(&self, asset: &Self::AssetId) -> Option<&Ask> { if asset == &self.instrument_id { self.order_book.asks.first() } else { None } }

    fn post_ask(&mut self, seller: AgentId, asset: Self::AssetId, qty: f64, price: f64) {
        if asset == self.instrument_id {
            self.order_book.asks.push(Ask {
                agent_id: seller,
                price,
                quantity: qty,
            });
        }
    }
    fn post_bid(&mut self, buyer: AgentId, asset: Self::AssetId, qty: f64, price: f64) {
        if asset == self.instrument_id {
            self.order_book.bids.push(Bid {
                agent_id: buyer,
                price,
                quantity: qty,
            });
        }
    }
    // TODO
    fn match_orders(&mut self) -> Vec<Trade> {
        let mut trades = Vec::new();
        trades
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Exchange {
    pub goods_markets: HashMap<GoodId, GoodsMarket>,
    pub financial_markets: HashMap<InstrumentId, FinancialMarket>,
}

impl Exchange {
    pub fn new() -> Self {
        let mut exchange = Self {
            goods_markets: HashMap::new(),
            financial_markets: HashMap::new(),
        };
        
        exchange.register_goods_market(GoodId::generic());
        exchange
    }
    
    pub fn register_goods_market(&mut self, good_id: GoodId) {
        self.goods_markets.entry(good_id.clone())
            .or_insert_with(|| GoodsMarket::new(good_id));
    }
    
    pub fn register_financial_market(&mut self, instrument_id: InstrumentId) {
        self.financial_markets.entry(instrument_id.clone())
            .or_insert_with(|| FinancialMarket::new(instrument_id));
    }
    
    pub fn goods_market(&self, good_id: &GoodId) -> Option<&GoodsMarket> {
        self.goods_markets.get(good_id)
    }
    
    pub fn goods_market_mut(&mut self, good_id: &GoodId) -> Option<&mut GoodsMarket> {
        self.goods_markets.get_mut(good_id)
    }
    
    pub fn financial_market(&self, instrument_id: &InstrumentId) -> Option<&FinancialMarket> {
        self.financial_markets.get(instrument_id)
    }
    
    pub fn financial_market_mut(&mut self, instrument_id: &InstrumentId) -> Option<&mut FinancialMarket> {
        self.financial_markets.get_mut(instrument_id)
    }

    pub fn quote(&self, asset: &AssetId) -> Option<f64> {
        match asset {
            AssetId::Good(good_id) => {
                self.goods_market(good_id).and_then(|market| market.quote(good_id))
            },
            AssetId::Instrument(instrument_id) => {
                self.financial_market(instrument_id).and_then(|market| market.quote(instrument_id))
            },
        }
    }
}