use crate::*;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::{collections::HashMap, fmt, str::FromStr};
use thiserror::Error;
use uuid::Uuid;

pub trait Tradable {
    fn check_holdings(&self, agent_id: &AgentId, quantity: f64, fs: &FinancialSystem) -> Result<(), String>;
}

impl Tradable for GoodId {
    fn check_holdings(&self, agent_id: &AgentId, quantity: f64, fs: &FinancialSystem) -> Result<(), String> {
        let bs = fs.get_bs_by_id(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
        let available = bs.get_inventory().and_then(|inv| inv.get(self)).map_or(0.0, |item| item.quantity);
        if available < quantity {
            Err(format!("Insufficient inventory for GoodId({:?}): have {:.2}, need {:.2}", self.0, available, quantity))
        } else {
            Ok(())
        }
    }
}

impl Tradable for FinancialMarketId {
    fn check_holdings(&self, agent_id: &AgentId, quantity: f64, fs: &FinancialSystem) -> Result<(), String> {
        match self {
            FinancialMarketId::SecuredOvernightFinancing => {
                let reserves = fs.get_bank_reserves(agent_id).unwrap_or(0.0);
                if reserves < quantity {
                    Err(format!(
                        "Insufficient reserves for SOFR ask (lending): need ${:.2}, has ${:.2}",
                        quantity, reserves
                    ))
                } else {
                    Ok(())
                }
            }
            FinancialMarketId::Treasury { tenor } => {
                let bs = fs.get_bs_by_id(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
                let held_quantity = bs
                    .assets
                    .values()
                    .map(|inst| {
                        if let Some(bond_details) = inst.details.as_any().downcast_ref::<BondDetails>() {
                            if bond_details.bond_type == BondType::Government && &bond_details.tenor == tenor {
                                bond_details.quantity as f64
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        }
                    })
                    .sum::<f64>();

                if held_quantity < quantity {
                    Err(format!(
                        "Insufficient Treasury holdings ({:?}): need {:.0}, has {:.0}",
                        tenor, quantity, held_quantity
                    ))
                } else {
                    Ok(())
                }
            }
            FinancialMarketId::CorporateBond { .. } => Ok(()), // Placeholder
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LabourMarketId {
    GeneralLabour,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MarketId {
    Goods(GoodId),
    Financial(FinancialMarketId),
    Labour(LabourMarketId),
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MarketTick {
    pub date: chrono::NaiveDate,
    pub last_price: Option<f64>,
    pub last_qty: Option<f64>,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub spread: Option<f64>,
    pub volume: f64,
    pub turnover: f64, // volume * price
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MarketView {
    pub last: Option<f64>,
    pub mid: Option<f64>,
    pub spread: Option<f64>,
    pub volume: f64,
    pub turnover: f64,
    pub vwap_5: Option<f64>,
    pub ma_20: Option<f64>,
    pub realized_vol_20: Option<f64>,
}

impl MarketView {
    pub fn last_or_mid(&self) -> Option<f64> {
        self.last.or(self.mid)
    }
}


impl Tradable for MarketId {
    fn check_holdings(&self, agent_id: &AgentId, quantity: f64, fs: &FinancialSystem) -> Result<(), String> {
        match self {
            MarketId::Goods(good_id) => good_id.check_holdings(agent_id, quantity, fs),
            MarketId::Financial(fin_id) => fin_id.check_holdings(agent_id, quantity, fs),
            MarketId::Labour(_) => Err("Labour market holdings check not implemented".to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy)]
pub enum Tenor {
    T2Y,
    T5Y,
    T10Y,
    T30Y,
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

pub trait MarketSnapshotProvider {
    fn snapshot(&self) -> MarketSnapshot;
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarketSnapshot {
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub spread: Option<f64>,
}


#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Exchange {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub goods_markets: HashMap<GoodId, GoodsMarket>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub financial_markets: HashMap<FinancialMarketId, FinancialMarket>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub labour_markets: HashMap<LabourMarketId, LabourMarket>,
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

    pub fn clear_markets(&mut self) -> (Vec<Trade>, HashMap<MarketId, MarketSnapshot>) {
        let mut all_trades = Vec::new();
        let mut snapshots = HashMap::new();

        for (id, market) in self.goods_markets.iter_mut() {
            let market_id = MarketId::Goods(*id);
            snapshots.insert(market_id.clone(), market.snapshot());
            all_trades.extend(market.order_book.clear_and_match(&market_id));
        }
        for (id, market) in self.financial_markets.iter_mut() {
            let market_id = MarketId::Financial(id.clone());
            snapshots.insert(market_id.clone(), market.snapshot());
            all_trades.extend(market.order_book.clear_and_match(&market_id));
        }
        (all_trades, snapshots)
    }
    pub fn register_labour_market(&mut self, market_id: LabourMarketId) {
        let name = market_id.clone().to_string();
        self.labour_markets.entry(market_id.clone()).or_insert_with(|| LabourMarket {
            market_id,
            name,
            job_offers: Vec::new(),
            job_applications: Vec::new(),
        });
    }

    pub fn labour_market_mut(&mut self, market_id: &LabourMarketId) -> Option<&mut LabourMarket> {
        self.labour_markets.get_mut(market_id)
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
        self.order_book.best_ask()
    }
}

impl MarketSnapshotProvider for GoodsMarket {
    fn snapshot(&self) -> MarketSnapshot {
        MarketSnapshot {
            best_bid: self.order_book.best_bid().map(|b| b.price),
            best_ask: self.order_book.best_ask().map(|a| a.price),
            spread: self.order_book.spread(),
        }
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

impl MarketSnapshotProvider for FinancialMarket {
    fn snapshot(&self) -> MarketSnapshot {
        MarketSnapshot {
            best_bid: self.order_book.best_bid().map(|b| b.price),
            best_ask: self.order_book.best_ask().map(|a| a.price),
            spread: self.order_book.spread(),
        }
    }
}

impl fmt::Display for LabourMarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LabourMarketId::GeneralLabour => write!(f, "GeneralLabour"),
        }
    }
}

impl FromStr for LabourMarketId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GeneralLabour" => Ok(LabourMarketId::GeneralLabour),
            _ => Err(format!("Unknown LabourMarketId: {}", s)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobOffer {
    pub offer_id: Uuid,
    pub firm_id: AgentId,
    pub wage_rate: f64,
    pub hours_required: f64,
    pub quantity: u32, // Number of positions open
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobApplication {
    pub application_id: Uuid,
    pub consumer_id: AgentId,
    pub reservation_wage: f64, // Minimum wage acceptable
    pub hours_desired: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LabourMarket {
    pub market_id: LabourMarketId,
    pub name: String,
    pub job_offers: Vec<JobOffer>,
    pub job_applications: Vec<JobApplication>,
}