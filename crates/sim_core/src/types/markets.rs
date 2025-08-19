use crate::utils::{BasisPoints, bps_to_decimal, decimal_to_bps};
use crate::*;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::VecDeque;
use std::{collections::HashMap, fmt, str::FromStr};
use thiserror::Error;
use uuid::Uuid;
use std::collections::BTreeMap;

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
            FinancialMarketId::FederalFundsOvernight | FinancialMarketId::TreasuryRepoOvernight => {
                let reserves = fs.get_bank_reserves(agent_id).unwrap_or(0.0);
                if reserves < quantity {
                    let market_name = match self {
                        FinancialMarketId::FederalFundsOvernight => "federal funds",
                        FinancialMarketId::TreasuryRepoOvernight => "Treasury repo",
                        _ => "overnight funding",
                    };
                    Err(format!(
                        "Insufficient reserves for {} ask (lending): need ${:.2}, has ${:.2}",
                        market_name, quantity, reserves
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
            FinancialMarketId::CorporateBond { .. }
            | FinancialMarketId::DiscountWindow
            | FinancialMarketId::StandingRepoFacility
            | FinancialMarketId::OvernightReverseRepo => Ok(()),
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

impl std::hash::Hash for MarketId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            MarketId::Goods(id) => {
                0.hash(state);
                id.hash(state);
            }
            MarketId::Financial(id) => {
                1.hash(state);
                id.hash(state);
            }
            MarketId::Labour(id) => {
                2.hash(state);
                id.hash(state);
            }
        }
    }
}

impl std::cmp::PartialEq for MarketId {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MarketId::Goods(id1), MarketId::Goods(id2)) => id1 == id2,
            (MarketId::Financial(id1), MarketId::Financial(id2)) => id1 == id2,
            (MarketId::Labour(id1), MarketId::Labour(id2)) => id1 == id2,
            _ => false,
        }
    }
}

impl std::cmp::Eq for MarketId {}

impl std::fmt::Display for MarketId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketId::Goods(id) => write!(f, "Goods({})", id),
            MarketId::Financial(id) => write!(f, "Financial({})", id),
            MarketId::Labour(id) => write!(f, "Labour({})", id),
        }
    }
}

#[derive(Debug, Error)]
pub enum ParseMarketIdError {
    #[error("Invalid MarketId format: {0}")]
    InvalidFormat(String),
    #[error("Failed to parse GoodId: {0}")]
    ParseGoodId(String),
    #[error("Failed to parse FinancialMarketId: {0}")]
    ParseFinancialMarketId(#[from] ParseFinancialMarketIdError),
    #[error("Failed to parse LabourMarketId: {0}")]
    ParseLabourMarketId(String),
}


impl std::str::FromStr for MarketId {
    type Err = ParseMarketIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(content) = s.strip_prefix("Goods(").and_then(|s| s.strip_suffix(')')) {
            let id = content.parse().map_err(|_| ParseMarketIdError::ParseGoodId(content.to_string()))?;
            return Ok(MarketId::Goods(id));
        }
        if let Some(content) = s.strip_prefix("Financial(").and_then(|s| s.strip_suffix(')')) {
            let id = content.parse()?;
            return Ok(MarketId::Financial(id));
        }
        if let Some(content) = s.strip_prefix("Labour(").and_then(|s| s.strip_suffix(')')) {
            let id = content.parse().map_err(|e| ParseMarketIdError::ParseLabourMarketId(e))?;
            return Ok(MarketId::Labour(id));
        }
        Err(ParseMarketIdError::InvalidFormat(s.to_string()))
    }
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
    pub turnover: f64,
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

    pub fn to_years(&self) -> f64 {
        match self {
            Tenor::T2Y => 2.0,
            Tenor::T5Y => 5.0,
            Tenor::T10Y => 10.0,
            Tenor::T30Y => 30.0,
        }
    }

    pub fn add_to_date(&self, date: chrono::NaiveDate) -> chrono::NaiveDate {
        date + chrono::Duration::days(self.to_days() as i64)
    }

    pub fn periods(&self, frequency: usize) -> usize {
        let years = match self {
            Tenor::T2Y => 2,
            Tenor::T5Y => 5,
            Tenor::T10Y => 10,
            Tenor::T30Y => 30,
        };
        years * frequency
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FinancialMarketId {
    FederalFundsOvernight,
    TreasuryRepoOvernight,
    Treasury { tenor: Tenor },
    CorporateBond { rating: CreditRating },
    DiscountWindow,
    StandingRepoFacility,
    OvernightReverseRepo,
}

impl fmt::Display for FinancialMarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FinancialMarketId::FederalFundsOvernight => write!(f, "FedFunds_ON"),
            FinancialMarketId::TreasuryRepoOvernight => write!(f, "TreasuryRepo_ON"),
            FinancialMarketId::Treasury { tenor } => write!(f, "Treasury_{}", tenor),
            FinancialMarketId::CorporateBond { rating } => write!(f, "CorpBond_{}", rating),
            FinancialMarketId::DiscountWindow => write!(f, "DiscountWindow"),
            FinancialMarketId::StandingRepoFacility => write!(f, "SRF"),
            FinancialMarketId::OvernightReverseRepo => write!(f, "ON_RRP"),
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
        match s {
            "FedFunds_ON" => Ok(FinancialMarketId::FederalFundsOvernight),
            "TreasuryRepo_ON" => Ok(FinancialMarketId::TreasuryRepoOvernight),
            "DiscountWindow" => Ok(FinancialMarketId::DiscountWindow),
            "SRF" => Ok(FinancialMarketId::StandingRepoFacility),
            "ON_RRP" => Ok(FinancialMarketId::OvernightReverseRepo),
            "SOFR" => Ok(FinancialMarketId::TreasuryRepoOvernight),
            _ => {
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
    }
}

impl RatesMarket for FinancialMarketId {
    fn price_to_daily_rate(&self, price: f64) -> f64 {
        match self {
            FinancialMarketId::FederalFundsOvernight | FinancialMarketId::TreasuryRepoOvernight => {
                if price <= 0.0 {
                    return f64::INFINITY;
                }
                (1.0 / price) - 1.0
            }
            _ => 0.0,
        }
    }

    fn daily_rate_to_annual_bps(&self, daily_rate: f64) -> BasisPoints {
        match self {
            FinancialMarketId::FederalFundsOvernight | FinancialMarketId::TreasuryRepoOvernight => {
                decimal_to_bps(daily_rate * 360.0)
            }
            _ => 0.0,
        }
    }

    fn annual_bps_to_daily_rate(&self, annual_bps: BasisPoints) -> f64 {
        match self {
            FinancialMarketId::FederalFundsOvernight | FinancialMarketId::TreasuryRepoOvernight => {
                bps_to_decimal(annual_bps) / 360.0
            }
            _ => 0.0,
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
        let best_bid = self.best_bid().map(|b| b.price);
        let best_ask = self.best_ask().map(|a| a.price);

        let bid_size_at_best =
            best_bid.map(|px| self.bids.iter().filter(|b| b.price == px).map(|b| b.quantity).sum()).unwrap_or(0.0);

        let ask_size_at_best =
            best_ask.map(|px| self.asks.iter().filter(|a| a.price == px).map(|a| a.quantity).sum()).unwrap_or(0.0);

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimedTrade {
    pub at: i64,
    pub trade: Trade,
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
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub trade_tape: HashMap<MarketId, VecDeque<TimedTrade>>,
}

impl Exchange {
    pub fn register_goods_market(&mut self, good_id: GoodId, goods_registry: &GoodsRegistry) {
        let name = goods_registry.get_good_name(&good_id).unwrap_or("Unknown Good").to_string();
        self.goods_markets.entry(good_id).or_insert_with(|| GoodsMarket::new(good_id, name));
    }

    pub fn register_financial_market(&mut self, market_id: FinancialMarketId) {
        let name = match &market_id {
            FinancialMarketId::FederalFundsOvernight => "Federal Funds Overnight".to_string(),
            FinancialMarketId::TreasuryRepoOvernight => "Treasury Repo Overnight".to_string(),
            FinancialMarketId::Treasury { tenor } => format!("Treasury {}", tenor),
            FinancialMarketId::CorporateBond { rating } => format!("Corporate Bond {:?}", rating),
            FinancialMarketId::DiscountWindow => "Discount Window".to_string(),
            FinancialMarketId::StandingRepoFacility => "Standing Repo Facility".to_string(),
            FinancialMarketId::OvernightReverseRepo => "Reverse Repo Overnight".to_string(),
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

    pub fn clear_markets(&mut self, now_ts: i64) -> (Vec<Trade>, HashMap<MarketId, MarketSnapshot>) {
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
        for tr in &all_trades {
            let e = self.trade_tape.entry(tr.market_id.clone()).or_default();
            e.push_back(TimedTrade { at: now_ts, trade: tr.clone() });
            if e.len() > 10_000 {
                e.pop_front();
            }
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

    pub fn get_treasury_bond_details(&self, fs: &FinancialSystem, tenor: &Tenor) -> Option<BondDetails> {
        for instrument in fs.instruments.values() {
            if let Some(bond_details) = instrument.details.as_any().downcast_ref::<BondDetails>() {
                if bond_details.bond_type == BondType::Government && &bond_details.tenor == tenor {
                    return Some(bond_details.clone());
                }
            }
        }
        None
    }
    pub fn get_treasury_market_summaries(&self, fs: &FinancialSystem) -> Vec<TreasuryMarketSummary> {
        let mut summaries = Vec::new();

        for (market_key, market) in &self.financial_markets {
            if let FinancialMarketId::Treasury { tenor } = market_key {
                let price = market.current_price();
                let ytm = market.calculate_ytm(fs).unwrap_or_else(|| market.default_yield());
                let spread_bps = market.spread_bps();

                summaries.push(TreasuryMarketSummary {
                    market_id: market_key.clone(),
                    tenor: *tenor,
                    price,
                    yield_to_maturity: ytm,
                    spread_bps,
                });
            }
        }
        summaries.sort_by_key(|s| s.tenor.to_days());
        summaries
    }

    pub fn calculate_candles(&self, market_id: &MarketId, bucket_secs: i64, limit: usize) -> Vec<Candle> {
        if bucket_secs <= 0 {
            return Vec::new();
        }

        let tape = match self.trade_tape.get(market_id) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut buckets: BTreeMap<i64, Vec<&Trade>> = BTreeMap::new();
        for t in tape {
            let k = (t.at / bucket_secs) * bucket_secs;
            buckets.entry(k).or_default().push(&t.trade);
        }

        buckets
            .into_iter()
            .rev()
            .take(limit)
            .rev()
            .map(|(ts, group)| {
                let open = group.first().map(|x| x.price).unwrap_or(0.0);
                let close = group.last().map(|x| x.price).unwrap_or(open);
                let high = group.iter().map(|x| x.price).fold(f64::MIN, f64::max);
                let low = group.iter().map(|x| x.price).fold(f64::MAX, f64::min);
                let volume = group.iter().map(|x| x.quantity).sum::<f64>();
                let vwap = if volume > 0.0 {
                    Some(group.iter().map(|x| x.price * x.quantity).sum::<f64>() / volume)
                } else {
                    None
                };

                Candle { ts, open, high, low, close, volume, vwap, trades_count: group.len() as u32 }
            })
            .collect()
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

    pub fn current_price(&self) -> Option<f64> {
        self.order_book.representative_price()
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

    pub fn current_price(&self) -> f64 {
        self.order_book.representative_price().unwrap_or(100.0)
    }

    pub fn last_or_mid(&self) -> Option<f64> {
        self.order_book
            .mid_price()
            .or_else(|| self.order_book.best_bid().map(|b| b.price))
            .or_else(|| self.order_book.best_ask().map(|a| a.price))
    }

    pub fn spread_bps(&self) -> f64 {
        if let Some(spread) = self.order_book.spread() {
            if let Some(mid) = self.order_book.mid_price() {
                if mid > 0.0 {
                    return (spread / mid) * 10000.0;
                }
            }
            spread * 100.0
        } else {
            0.0
        }
    }

    pub fn calculate_ytm(&self, fs: &FinancialSystem) -> Option<f64> {
        if let FinancialMarketId::Treasury { tenor } = &self.market_id {
            if let Some(bond_details) = fs.exchange.get_treasury_bond_details(fs, tenor) {
                let price = self.current_price();
                let face_value = bond_details.face_value;
                let coupon_rate = bond_details.coupon_rate_bps / 10000.0;
                let years_to_maturity = tenor.to_years();
                let frequency = bond_details.frequency;

                return Some(math::pricing::ytm_bond(price, face_value, coupon_rate, years_to_maturity, frequency));
            }
        }
        None
    }

    pub fn default_yield(&self) -> f64 {
        if let FinancialMarketId::Treasury { tenor } = &self.market_id {
            match tenor {
                Tenor::T2Y => 0.025,
                Tenor::T5Y => 0.030,
                Tenor::T10Y => 0.035,
                Tenor::T30Y => 0.040,
            }
        } else {
            0.0
        }
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
    pub quantity: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobApplication {
    pub application_id: Uuid,
    pub consumer_id: AgentId,
    pub reservation_wage: f64,
    pub hours_desired: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LabourMarket {
    pub market_id: LabourMarketId,
    pub name: String,
    pub job_offers: Vec<JobOffer>,
    pub job_applications: Vec<JobApplication>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MarketDepthSummary {
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub bid_size_at_best: f64,
    pub ask_size_at_best: f64,
    pub bid_levels: usize,
    pub ask_levels: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketSummary {
    pub depth: MarketDepthSummary,
    pub mid: Option<f64>,
    pub spread: Option<f64>,
    pub last_price: Option<f64>,
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TreasuryMarketSummary {
    pub market_id: FinancialMarketId,
    pub tenor: Tenor,
    pub price: f64,
    pub yield_to_maturity: f64,
    pub spread_bps: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candle {
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub vwap: Option<f64>,
    pub trades_count: u32,
}

impl MarketSummaryProvider for GoodsMarket {
    fn summary(&self) -> MarketSummary {
        MarketSummary {
            depth: self.order_book.depth_summary(),
            mid: self.order_book.mid_price(),
            spread: self.order_book.spread(),
            last_price: self.current_price(),
        }
    }
}
impl MarketSummaryProvider for FinancialMarket {
    fn summary(&self) -> MarketSummary {
        MarketSummary {
            depth: self.order_book.depth_summary(),
            mid: self.order_book.mid_price(),
            spread: self.order_book.spread(),
            last_price: self.order_book.representative_price(),
        }
    }
}