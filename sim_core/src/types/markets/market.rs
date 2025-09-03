use crate::prelude::*;
use crate::types::money::Money;
use chrono::NaiveDate;
use ordered_float::OrderedFloat;
use rust_decimal::prelude::*;
use serde::{
    de::{Deserializer},
    Serialize, Deserialize,
    ser::{SerializeStruct, Serializer},
};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimedTrade {
    pub ts: std::time::SystemTime,
    pub trade: Trade,
}

pub trait Product {
    type Key: Eq + Hash + Clone + Debug + Serialize + for<'de> Deserialize<'de>;
    type Quote = Money;
    type Lot = f64;
    fn tick_size(&self) -> Money {
        Money::from_f64(0.01).unwrap_or(Money::ZERO)
    }
    fn lot_size(&self) -> f64 {
        1.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ListingKey {
    GovBond {
        tenor_years: u16,
    },
    CorpBond {
        rating: CreditRating,
        tenor_bucket: TenorBucket,
    },
    CashON,
    RealAsset,
    Equity {
        issuer: AgentId,
    },
    Derivative {
        underlying: UnderlyingAsset,
        derivative_type_key: DerivativeTypeKey,
    },
    StructuredTranche {
        rating: CreditRating,
        tranche_type: TrancheType,
    },
    Repo,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DerivativeTypeKey {
    Option {
        style: OptionStyle,
        strike: OrderedFloat<f64>,
        expiry: NaiveDate,
    },
    Future {
        expiry: NaiveDate,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TenorBucket {
    LT1Y,
    Y1_3,
    Y3_5,
    Y5_7,
    Y7_10,
    GT10,
}

impl TenorBucket {
    pub fn from_years(y: f64) -> Self {
        match y {
            y if y < 1.0 => TenorBucket::LT1Y,
            y if y < 3.0 => TenorBucket::Y1_3,
            y if y < 5.0 => TenorBucket::Y3_5,
            y if y < 7.0 => TenorBucket::Y5_7,
            y if y < 10.0 => TenorBucket::Y7_10,
            _ => TenorBucket::GT10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ListingRegistry {
    by_key: HashMap<ListingKey, Vec<InstrumentId>>,
    by_id: HashMap<InstrumentId, ListingKey>,
}

impl ListingRegistry {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn key_for_instrument(&self, id: &InstrumentId) -> Option<&ListingKey> {
        self.by_id.get(id)
    }
    pub fn register(&mut self, id: InstrumentId, inst: &Instrument) {
        let key = listing_key_from_instrument(inst);
        self.by_id.insert(id, key.clone());
        self.by_key.entry(key).or_default().push(id);
    }
    pub fn instruments(&self, key: &ListingKey) -> impl Iterator<Item = &InstrumentId> {
        self.by_key.get(key).into_iter().flatten()
    }
}

pub fn listing_key_from_instrument(inst: &Instrument) -> ListingKey {
    match &inst.instrument_type {
        InstrumentType::Bond(b) => match b.bond_type {
            BondType::Government => {
                let years =
                    ((b.maturity_date - b.issue_date).num_days() as f64 / 365.0).round() as u16;
                ListingKey::GovBond {
                    tenor_years: years.max(1),
                }
            }
            BondType::Corporate => {
                let years = ((b.maturity_date - b.issue_date).num_days() as f64 / 365.0).max(0.0);
                ListingKey::CorpBond {
                    rating: b.rating,
                    tenor_bucket: TenorBucket::from_years(years),
                }
            }
            BondType::InterbankLoan => ListingKey::CashON,
        },
        InstrumentType::Cash(_) => ListingKey::CashON,
        InstrumentType::RealAsset(_) => ListingKey::RealAsset,
        InstrumentType::Equity(e) => ListingKey::Equity { issuer: e.issuer },
        InstrumentType::Repo(_) => ListingKey::Repo,
        InstrumentType::Derivative(d) => {
            let derivative_type_key = match &d.derivative_type {
                DerivativeType::Option(o) => DerivativeTypeKey::Option {
                    style: o.style,
                    strike: OrderedFloat(o.strike_price.to_f64()),
                    expiry: d.expiry_date,
                },
                DerivativeType::Future(_) => DerivativeTypeKey::Future {
                    expiry: d.expiry_date,
                },
            };
            ListingKey::Derivative {
                underlying: d.underlying.clone(),
                derivative_type_key,
            }
        }
        InstrumentType::StructuredTranche(s) => ListingKey::StructuredTranche {
            rating: s.rating,
            tranche_type: s.tranche_type,
        },
    }
}

pub trait Pricer<P: Product>: Send + Sync + Debug {
    fn mid_yield(&self, key: &P::Key) -> Option<f64>;
    fn price_from_yield(&self, key: &P::Key, y: f64) -> Option<P::Quote>;
    fn yield_from_price(&self, key: &P::Key, px: P::Quote) -> Option<f64>;
}

pub trait Settlement<P: Product>: Send + Sync + Debug {
    fn dvp(
        &self,
        key: &P::Key,
        buyer: AgentId,
        seller: AgentId,
        qty: P::Lot,
        price: P::Quote,
    ) -> Result<(), String>;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Market {
    pub book: OrderBook,
}

#[derive(Clone, Debug)]
pub struct MarketGeneric<P: Product> {
    pub key: P::Key,
    pub book: OrderBook,
    pub pricer: Arc<dyn Pricer<P>>,
    pub settlement: Arc<dyn Settlement<P>>,
}

impl<P: Product<Quote = Money, Lot = f64>> MarketGeneric<P> {
    pub fn snapshot(&self) -> MarketSnapshot {
        let best_bid = self.book.best_bid();
        let best_ask = self.book.best_ask();
        let mid = self.book.mid_price();
        let y_bid = best_bid.and_then(|px| self.pricer.yield_from_price(&self.key, px));
        let y_ask = best_ask.and_then(|px| self.pricer.yield_from_price(&self.key, px));
        let y_mid = mid.and_then(|px| self.pricer.yield_from_price(&self.key, px));
        MarketSnapshot {
            best_bid,
            best_ask,
            mid_price: mid,
            last_price: self.book.representative_price(),
            spread: self.book.spread(),
            volume_24h: 0.0,
            depth: self.book.depth_summary(),
            best_bid_yield: y_bid,
            best_ask_yield: y_ask,
            mid_yield: y_mid,
            last_yield: None,
        }
    }

    pub fn clear_and_match(&mut self, exchange_id: &MarketId) -> Vec<Trade> {
        let trades = self.book.clear_and_match(exchange_id);
        for t in &trades {
            let _ = self
                .settlement
                .dvp(&self.key, t.buyer, t.seller, t.quantity, t.price);
        }
        trades
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MarketIndex {
    pub by_issuer: HashMap<AgentId, Vec<InstrumentId>>,
    pub by_rating_and_tenor: HashMap<(CreditRating, TenorBucket), Vec<InstrumentId>>,
    pub by_bond_type: HashMap<BondType, Vec<InstrumentId>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GoodProduct;
impl Product for GoodProduct {
    type Key = GoodId;
}
#[derive(Debug, Clone, PartialEq)]
pub struct FinancialProduct;
impl Product for FinancialProduct {
    type Key = InstrumentId;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuctionBid {
    pub agent_id: AgentId,
    pub quantity: u32,
    pub price: Money,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DebtAuction {
    pub auction_id: Uuid,
    pub instrument_id: InstrumentId,
    pub quantity_offered: u32,
    pub status: AuctionStatus,
    pub bids: Vec<AuctionBid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AuctionStatus {
    Open,
    Closed,
}

#[derive(Clone, Default, Debug)]
pub struct Exchange {
    pub markets: HashMap<InstrumentId, MarketGeneric<FinancialProduct>>,
    pub index: MarketIndex,
    pub goods_markets: HashMap<GoodId, MarketGeneric<GoodProduct>>,
    pub labour_markets: HashMap<LabourMarketId, LabourMarket>,
    pub open_auctions: HashMap<Uuid, DebtAuction>,
    pub tape: HashMap<MarketId, Vec<TimedTrade>>,
}

impl Exchange {
    pub fn ensure_listed(&mut self, inst_id: InstrumentId, inst: &Instrument) {
        if !inst.should_create_order_book() {
            self.update_index_only(inst_id, inst);
            return;
        }
        
        self.markets.entry(inst_id).or_insert_with(|| make_financial_market(inst_id));

        self.update_index_only(inst_id, inst);
    }

    fn update_index_only(&mut self, inst_id: InstrumentId, inst: &Instrument) {
        match &inst.instrument_type {
            InstrumentType::Bond(b) => {
                self.index
                    .by_issuer
                    .entry(b.issuer)
                    .or_default()
                    .push(inst_id);
                self.index
                    .by_bond_type
                    .entry(b.bond_type)
                    .or_default()
                    .push(inst_id);
                let years = ((b.maturity_date - b.issue_date).num_days() as f64 / 365.0).max(0.0);
                let tenor_bucket = TenorBucket::from_years(years);
                self.index
                    .by_rating_and_tenor
                    .entry((b.rating, tenor_bucket))
                    .or_default()
                    .push(inst_id);
            }
            InstrumentType::Cash(c) => {
                self.index
                    .by_issuer
                    .entry(c.issuer)
                    .or_default()
                    .push(inst_id);
            }
            InstrumentType::Equity(e) => {
                self.index
                    .by_issuer
                    .entry(e.issuer)
                    .or_default()
                    .push(inst_id);
            }
            InstrumentType::StructuredTranche(s) => {
                self.index
                    .by_rating_and_tenor
                    .entry((s.rating, TenorBucket::GT10))
                    .or_default()
                    .push(inst_id);
            }
            _ => {}
        }
    }

    pub fn get_posted_rate(
        &self,
        inst_id: &InstrumentId,
        financial_system: &FinancialSystem,
        agents: &AgentRegistry,
    ) -> Option<f64> {
        let inst = financial_system.instruments.get(inst_id)?;

        match (&inst.instrument_type, &inst.listability) {
            (InstrumentType::Cash(details), Listability::Listed(VenueType::PostedRates)) => {
                match details.cash_type {
                    CashType::DemandDeposit => {
                        if let Some(bank) = agents.banks.get(&details.issuer) {
                            let base_rate =
                                bps_to_decimal(financial_system.central_bank.policy_rate_bps);
                            let spread = bps_to_decimal(bank.deposit_spread_bps);
                            Some((base_rate + spread).to_f64().unwrap_or(0.0))
                        } else {
                            None
                        }
                    }
                    CashType::CentralBankReserves => Some(
                        bps_to_decimal(financial_system.central_bank.policy_rate_bps)
                            .to_f64()
                            .unwrap_or(0.0),
                    ),
                    _ => Some(bps_to_decimal(details.interest_bps).to_f64().unwrap_or(0.0)),
                }
            }
            _ => None,
        }
    }

    pub fn has_order_book(&self, inst_id: &InstrumentId) -> bool {
        self.markets.contains_key(inst_id)
    }

    pub fn register_goods_market(&mut self, good_id: GoodId) {
        self.goods_markets
            .entry(good_id)
            .or_insert_with(|| make_goods_market(good_id));
    }

    pub fn register_labour_market(&mut self, market_id: LabourMarketId) {
        self.labour_markets
            .entry(market_id)
            .or_insert_with(LabourMarket::default);
    }

    pub fn financial_market_mut(&mut self, id: &InstrumentId) -> Option<&mut OrderBook> {
        self.markets.get_mut(id).map(|m| &mut m.book)
    }
    pub fn financial_market(&self, id: &InstrumentId) -> Option<&OrderBook> {
        self.markets.get(id).map(|m| &m.book)
    }
    pub fn goods_market(&self, id: &GoodId) -> Option<&MarketGeneric<GoodProduct>> {
        self.goods_markets.get(id)
    }
    pub fn goods_market_mut(&mut self, id: &GoodId) -> Option<&mut MarketGeneric<GoodProduct>> {
        self.goods_markets.get_mut(id)
    }

    pub fn labour_market(&self, id: &LabourMarketId) -> Option<&LabourMarket> {
        self.labour_markets.get(id)
    }

    pub fn labour_market_mut(&mut self, id: &LabourMarketId) -> Option<&mut LabourMarket> {
        self.labour_markets.get_mut(id)
    }
    pub fn conduct_dutch_auction(
        &mut self,
        auction_id: &Uuid,
        instruments: &HashMap<InstrumentId, Instrument>,
    ) -> Vec<Trade> {
        let mut trades = Vec::new();
        if let Some(auction) = self.open_auctions.get_mut(auction_id) {
            if auction.status != AuctionStatus::Open || auction.bids.is_empty() {
                auction.status = AuctionStatus::Closed;
                return trades;
            }
            let government_id = if let Some(instrument) = instruments.get(&auction.instrument_id)
            {
                if let InstrumentType::Bond(details) = &instrument.instrument_type {
                    details.issuer
                } else {
                    auction.status = AuctionStatus::Closed;
                    return trades;
                }
            } else {
                auction.status = AuctionStatus::Closed;
                return trades;
            };

            auction.bids.sort_by(|a, b| b.price.cmp(&a.price));

            let mut quantity_filled: u32 = 0;
            let mut clearing_price = Money::ZERO;
            let mut winning_bids: Vec<(AgentId, u32)> = Vec::new();

            for bid in &auction.bids {
                if quantity_filled >= auction.quantity_offered {
                    break;
                }
                let quantity_to_fill =
                    (auction.quantity_offered - quantity_filled).min(bid.quantity);
                winning_bids.push((bid.agent_id, quantity_to_fill));
                quantity_filled += quantity_to_fill;
                clearing_price = bid.price;
            }

            if !winning_bids.is_empty() {
                for (winner_id, quantity) in winning_bids {
                    trades.push(Trade {
                        trade_id: Uuid::new_v4(),
                        market_id: MarketId::Financial(auction.instrument_id),
                        buyer: winner_id,
                        seller: government_id,
                        quantity: quantity as f64,
                        price: clearing_price,
                    });
                }
            }
            println!(
                "Auction {} concluded: {} of {} units sold at {:?}",
                auction.auction_id, quantity_filled, auction.quantity_offered, clearing_price
            );
            auction.status = AuctionStatus::Closed;
        }

        trades
    }
}

fn make_goods_market(key: GoodId) -> MarketGeneric<GoodProduct> {
    MarketGeneric {
        key,
        book: OrderBook::new(),
        pricer: Arc::new(NoOpPricer),
        settlement: Arc::new(NoopSettlement),
    }
}
fn make_financial_market(key: InstrumentId) -> MarketGeneric<FinancialProduct> {
    MarketGeneric {
        key,
        book: OrderBook::new(),
        pricer: Arc::new(NoOpPricer),
        settlement: Arc::new(NoopSettlement),
    }
}
#[derive(Debug, Clone, PartialEq)]
struct NoOpPricer;
impl Pricer<GoodProduct> for NoOpPricer {
    fn mid_yield(&self, _key: &GoodId) -> Option<f64> {
        None
    }
    fn price_from_yield(&self, _key: &GoodId, _y: f64) -> Option<Money> {
        None
    }
    fn yield_from_price(&self, _key: &GoodId, _px: Money) -> Option<f64> {
        None
    }
}
impl Pricer<FinancialProduct> for NoOpPricer {
    fn mid_yield(&self, _key: &<FinancialProduct as Product>::Key) -> Option<f64> {
        None
    }
    fn price_from_yield(
        &self,
        _key: &<FinancialProduct as Product>::Key,
        _y: f64,
    ) -> Option<<FinancialProduct as Product>::Quote> {
        None
    }
    fn yield_from_price(
        &self,
        _key: &<FinancialProduct as Product>::Key,
        _px: <FinancialProduct as Product>::Quote,
    ) -> Option<f64> {
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
struct NoopSettlement;
impl<P: Product> Settlement<P> for NoopSettlement {
    fn dvp(
        &self,
        _key: &P::Key,
        _buyer: AgentId,
        _seller: AgentId,
        _qty: P::Lot,
        _price: P::Quote,
    ) -> Result<(), String> {
        Ok(())
    }
}

impl Serialize for Exchange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Exchange", 5)?;

        let markets_books: HashMap<_, _> = self
            .markets
            .iter()
            .map(|(k, v)| (*k, Market { book: v.book.clone() }))
            .collect();
        state.serialize_field("markets", &markets_books)?;

        state.serialize_field("index", &self.index)?;

        let goods_books: HashMap<_, _> = self
            .goods_markets
            .iter()
            .map(|(k, v)| (k, Market { book: v.book.clone() }))
            .collect();
        state.serialize_field("goods_markets", &goods_books)?;

        state.serialize_field("labour_markets", &self.labour_markets)?;

        state.serialize_field("tape", &self.tape)?;
        state.end()
    }
}

#[derive(Deserialize)]
pub struct ExchangeData {
    markets: HashMap<InstrumentId, Market>,
    index: MarketIndex,
    goods_markets: HashMap<GoodId, Market>,
    labour_markets: HashMap<LabourMarketId, LabourMarket>,
    tape: HashMap<MarketId, Vec<TimedTrade>>,
}

impl<'de> Deserialize<'de> for Exchange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = ExchangeData::deserialize(deserializer)?;

        let markets = data
            .markets
            .into_iter()
            .map(|(key, market)| {
                let mut mg = make_financial_market(key);
                mg.book = market.book;
                (key, mg)
            })
            .collect();

        let goods_markets = data
            .goods_markets
            .into_iter()
            .map(|(key, market)| {
                let mut mg = make_goods_market(key);
                mg.book = market.book;
                (key, mg)
            })
            .collect();

        Ok(Exchange {
            markets,
            index: data.index,
            goods_markets,
            labour_markets: data.labour_markets,
            open_auctions: HashMap::new(),
            tape: data.tape,
        })
    }
}