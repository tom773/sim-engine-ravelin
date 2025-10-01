use crate::prelude::*;
use crate::types::markets::pricers::BondPricingTerms;
use chrono::NaiveDate;
use ordered_float::OrderedFloat;
use parking_lot::RwLock;
use serde::{
    de::Deserializer,
    ser::{SerializeStruct, Serializer},
};
use std::collections::{HashMap, HashSet};
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
    GovBond { tenor_years: u16 },
    CorpBond { rating: CreditRating, tenor_bucket: TenorBucket },
    Cash { cash_type: CashType },
    CreditLoan { loan_type: LoanType },
    ConsumerLoan { category: ConsumerLoanCategory },
    CreditFacility { facility_type: FacilityType },
    CreditCard,
    TradeCredit,
    RealAsset,
    Equity { issuer: AgentId },
    Derivative { underlying: UnderlyingAsset, derivative_type_key: DerivativeTypeKey },
    StructuredProduct { rating: CreditRating, tranche_type: StructuredTrancheType },
    Repo,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DerivativeTypeKey {
    Option { style: OptionStyle, strike: OrderedFloat<f64>, expiry: NaiveDate },
    Future { expiry: NaiveDate },
    Custom { label: String },
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
    match inst.state() {
        InstrumentRuntime::Bond(b) => match b.bond_type() {
            BondType::Government | BondType::Municipal | BondType::Agency | BondType::Supranational => {
                let years = b.original_tenor_years().round() as u16;
                ListingKey::GovBond { tenor_years: years.max(1) }
            }
            BondType::Corporate => {
                let years = b.original_tenor_years();
                ListingKey::CorpBond {
                    rating: b.rating.unwrap_or(CreditRating::corporate_bbb()),
                    tenor_bucket: TenorBucket::from_years(years),
                }
            }
            BondType::InterbankLoan => ListingKey::CreditLoan { loan_type: LoanType::WorkingCapital },
        },
        InstrumentRuntime::Cash(c) => ListingKey::Cash { cash_type: c.cash_type },
        InstrumentRuntime::Credit(credit) => match credit {
            CreditState::Loan(loan) => ListingKey::CreditLoan { loan_type: loan.loan_type },
            CreditState::ConsumerLoan { category, .. } => ListingKey::ConsumerLoan { category: *category },
            CreditState::ConsumerCreditCard(_) => ListingKey::CreditCard,
            CreditState::TradeCredit(_) => ListingKey::TradeCredit,
            CreditState::Facility(facility) => ListingKey::CreditFacility { facility_type: facility.facility_type },
            CreditState::OvernightCredit(_) => ListingKey::CreditLoan { loan_type: LoanType::WorkingCapital },
        },
        InstrumentRuntime::RealAsset(_) => ListingKey::RealAsset,
        InstrumentRuntime::Equity(e) => ListingKey::Equity { issuer: e.profile.issuer },
        InstrumentRuntime::Repo(_) => ListingKey::Repo,
        InstrumentRuntime::Structured(tranche) => {
            ListingKey::StructuredProduct { rating: tranche.rating, tranche_type: tranche.tranche_type }
        }
        InstrumentRuntime::Derivative(derivative) => {
            let fallback_date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            let derivative_type_key = match &derivative.contract {
                DerivativeContract::Option(option) => {
                    let strike = option.strike_price.to_f64();
                    let expiry = derivative.expiry_date.unwrap_or(fallback_date);
                    DerivativeTypeKey::Option { style: option.style, strike: OrderedFloat(strike), expiry }
                }
                DerivativeContract::Future(_) => {
                    let expiry = derivative.expiry_date.unwrap_or(fallback_date);
                    DerivativeTypeKey::Future { expiry }
                }
                DerivativeContract::Custom { description } => DerivativeTypeKey::Custom { label: description.clone() },
            };

            ListingKey::Derivative { underlying: derivative.underlying.clone(), derivative_type_key }
        }
    }
}

fn instrument_listing_agent(inst: &Instrument) -> Option<AgentId> {
    match inst.state() {
        InstrumentRuntime::Bond(b) => Some(b.issuer),
        InstrumentRuntime::Cash(c) => Some(c.issuer),
        InstrumentRuntime::Credit(credit) => match credit {
            CreditState::Loan(loan) => Some(loan.lender),
            CreditState::ConsumerLoan { loan, .. } => Some(loan.lender),
            CreditState::ConsumerCreditCard(facility) => Some(facility.lender),
            CreditState::TradeCredit(trade) => Some(trade.creditor),
            CreditState::Facility(facility) => Some(facility.lender),
            CreditState::OvernightCredit(overnight) => Some(overnight.lender),
        },
        InstrumentRuntime::Equity(equity) => Some(equity.profile.issuer),
        InstrumentRuntime::Structured(tranche) => Some(tranche.issuer),
        InstrumentRuntime::Derivative(derivative) => Some(derivative.issuer),
        InstrumentRuntime::Repo(repo) => Some(repo.lender),
        InstrumentRuntime::RealAsset(real) => match real {
            RealAssetState::Inventory { owner, .. } => Some(*owner),
            RealAssetState::Property { owner, .. } => Some(*owner),
            RealAssetState::Custom { owner, .. } => Some(*owner),
        },
    }
}

impl ListingKey {
    pub fn label(&self) -> String {
        match self {
            ListingKey::GovBond { tenor_years } => format!("Gov Bond {}Y", tenor_years),
            ListingKey::CorpBond { rating, tenor_bucket } => format!("Corp Bond {:?} {:?}", rating, tenor_bucket),
            ListingKey::Cash { cash_type } => format!("Cash {:?}", cash_type),
            ListingKey::CreditLoan { loan_type } => format!("Loan {:?}", loan_type),
            ListingKey::ConsumerLoan { category } => format!("Consumer Loan {:?}", category),
            ListingKey::CreditFacility { facility_type } => format!("Credit Facility {:?}", facility_type),
            ListingKey::CreditCard => "Credit Card".into(),
            ListingKey::TradeCredit => "Trade Credit".into(),
            ListingKey::RealAsset => "Real Asset".into(),
            ListingKey::Equity { issuer } => format!("Equity {}", issuer.0),
            ListingKey::Derivative { underlying, derivative_type_key } => {
                format!("Derivative {} on {}", derivative_type_key.label(), underlying_label(underlying))
            }
            ListingKey::StructuredProduct { rating, tranche_type } => {
                format!("Structured {:?} {:?}", rating, tranche_type)
            }
            ListingKey::Repo => "Repo".into(),
        }
    }
}

impl DerivativeTypeKey {
    fn label(&self) -> String {
        match self {
            DerivativeTypeKey::Option { style, strike, expiry } => {
                format!("{:?} Option {:.2} @ {}", style, strike.into_inner(), expiry)
            }
            DerivativeTypeKey::Future { expiry } => format!("Future @ {}", expiry),
            DerivativeTypeKey::Custom { label } => label.clone(),
        }
    }
}

fn underlying_label(underlying: &UnderlyingAsset) -> String {
    match underlying {
        UnderlyingAsset::Instrument(id) => format!("Instrument {}", id.0),
        UnderlyingAsset::Good(good_id) => format!("Good {}", good_id.0),
        UnderlyingAsset::Index(name) => name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::instrument::archetypes::{
        LoanArchetype, LoanRepaymentSchedule, PrepaymentPenalty, PrepaymentTerms,
    };
    use chrono::{Duration, NaiveDate};
    use uuid::Uuid;

    #[test]
    fn listing_key_maps_business_loans() {
        let issue_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let maturity_date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

        let loan_archetype = LoanArchetype {
            facility_type: FacilityType::TermLoanFacility,
            amortization: Amortization::Annuity,
            prepayment: PrepaymentTerms {
                allowed: true,
                penalty_type: PrepaymentPenalty::None,
                lockout_period_months: None,
            },
            principal: Money::from(500_000_i64),
            day_count: DayCount::Act360,
            compounding: Compounding::Simple,
            rate_structure: RateStructure {
                base_rate: RateIndex::SOFR,
                spread_bps: BasisPoints::new(250, 1),
                floor_bps: None,
                cap_bps: None,
            },
            repayment_schedule: LoanRepaymentSchedule {
                payment_frequency: PaymentFrequency::Quarterly,
                term_months: 12,
            },
            collateral_requirements: Vec::new(),
        };

        let impairment = ImpairmentState {
            stage: ImpairmentStage::Stage1Performing,
            provision_amount: Money::ZERO,
            days_past_due: 0,
            probability_of_default: 0.0,
            loss_given_default: 0.0,
            exposure_at_default: Money::ZERO,
        };

        let loan_state = LoanState::new(
            Uuid::new_v4(),
            AgentId(Uuid::new_v4()),
            AgentId(Uuid::new_v4()),
            None,
            LoanType::TermLoan,
            loan_archetype,
            Money::from(400_000_i64),
            issue_date,
            maturity_date,
            issue_date + Duration::days(30),
            issue_date,
            Vec::new(),
            Vec::new(),
            None,
            impairment,
            Money::ZERO,
            Money::ZERO,
        );

        let instrument = Instrument::new(
            InstrumentIdentifiers::from(InstrumentId(Uuid::new_v4())),
            MarketProfile::from_market(InstrumentMarket::MoneyMarket(MoneyMarketSegment::CorporateShortTerm)),
            Listability::Unlisted,
            InstrumentRuntime::Credit(CreditState::Loan(loan_state)),
        );

        let key = listing_key_from_instrument(&instrument);
        match key {
            ListingKey::CreditLoan { loan_type } => assert_eq!(loan_type, LoanType::TermLoan),
            other => panic!("expected credit loan listing key, got {:?}", other),
        }
    }
}

pub trait Pricer<P: Product>: Send + Sync + Debug {
    fn mid_yield(&self, key: &P::Key) -> Option<f64>;
    fn price_from_yield(&self, key: &P::Key, y: f64) -> Option<P::Quote>;
    fn yield_from_price(&self, key: &P::Key, px: P::Quote) -> Option<f64>;
    fn fair_price(&self, _key: &P::Key) -> Option<P::Quote> {
        None
    }
}

pub trait Settlement<P: Product>: Send + Sync + Debug {
    fn dvp(&self, key: &P::Key, buyer: AgentId, seller: AgentId, qty: P::Lot, price: P::Quote) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub enum MarketType {
    Financial(MarketGeneric<FinancialProduct>),
    Goods(MarketGeneric<GoodProduct>),
    Labour(LabourMarket),
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
            last_price: self.book.representative_price(),
            spread: self.book.spread(),
            depth: self.book.depth_summary(),
            yield_ask: y_ask,
            yield_bid: y_bid,
            yield_mid: y_mid,
        }
    }

    pub fn clear_and_match(&mut self, exchange_id: &Symbol) -> Vec<Trade> {
        let trades = self.book.clear_and_match(exchange_id);
        for t in &trades {
            let _ = self.settlement.dvp(&self.key, t.buyer, t.seller, t.quantity, t.price);
        }
        trades
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MarketIndex {
    pub by_issuer: HashMap<AgentId, Vec<InstrumentId>>,
    pub by_listing: HashMap<ListingKey, Vec<InstrumentId>>,
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

#[derive(Clone, Debug, Default)]
pub struct Exchange {
    pub markets: HashMap<Symbol, MarketType>,
    pub symbol_registry: SymbolRegistry,

    pub inst_to_symbol: HashMap<InstrumentId, Symbol>,
    pub good_to_symbol: HashMap<GoodId, Symbol>,
    pub labour_to_symbol: HashMap<LabourMarketId, Symbol>,

    pub symbol_to_inst: HashMap<Symbol, InstrumentId>,
    pub symbol_to_good: HashMap<Symbol, GoodId>,

    pub index: MarketIndex,
    pub open_auctions: HashMap<Uuid, DebtAuction>,
    pub tape: HashMap<Symbol, Vec<TimedTrade>>,
    pub recent_trades: Vec<Trade>,
    pub pricing_feeds: Option<PricingFeeds>,
    pub labour_offer_cache: Arc<RwLock<Option<HashSet<AgentId>>>>,
}

impl Exchange {
    pub fn invalidate_labour_offer_cache(&self) {
        self.labour_offer_cache.write().take();
    }

    pub fn firm_has_open_job_offer(&self, firm_id: &AgentId) -> bool {
        {
            let guard = self.labour_offer_cache.read();
            if let Some(cache) = guard.as_ref() {
                return cache.contains(firm_id);
            }
        }

        let mut write_guard = self.labour_offer_cache.write();
        if let Some(cache) = write_guard.as_ref() {
            return cache.contains(firm_id);
        }

        let mut rebuilt = HashSet::new();
        for market in self.markets.values() {
            if let MarketType::Labour(labour_market) = market {
                for offer in labour_market.job_offers.iter().filter(|offer| offer.quantity > 0) {
                    rebuilt.insert(offer.firm_id);
                }
            }
        }

        let contains = rebuilt.contains(firm_id);
        *write_guard = Some(rebuilt);
        contains
    }

    pub fn attach_pricing_feeds(&mut self, feeds: PricingFeeds) {
        self.pricing_feeds = Some(feeds.clone());
        for market in self.markets.values_mut() {
            match market {
                MarketType::Goods(mkt) => {
                    mkt.pricer = Arc::new(CostPlusGoodsPricer::new(feeds.clone(), CostPlusParams::default()));
                }
                _ => {}
            }
        }
    }

    pub fn attach_pricing_feeds_with_registry(
        &mut self, feeds: PricingFeeds, instruments: &std::collections::HashMap<InstrumentId, Instrument>,
    ) {
        self.pricing_feeds = Some(feeds.clone());
        for (_, market) in self.markets.iter_mut() {
            match market {
                MarketType::Goods(g) => {
                    g.pricer = Arc::new(CostPlusGoodsPricer::new(feeds.clone(), CostPlusParams::default()));
                }
                MarketType::Financial(f) => {
                    if let Some(inst) = instruments.get(&f.key) {
                        if let InstrumentRuntime::Bond(bond) = inst.state() {
                            if bond.bond_type() == BondType::Government {
                                let terms = BondPricingTerms::from(bond);
                                f.pricer = Arc::new(GovTermStructurePricer::new(
                                    terms,
                                    TermStructureMethod::default(),
                                    feeds.clone(),
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn ensure_financial_market(&mut self, inst_id: InstrumentId, inst: &Instrument) -> Symbol {
        let symbol = self.symbol_registry.ensure_instrument(inst_id, inst);

        self.inst_to_symbol.insert(inst_id, symbol.clone());
        self.symbol_to_inst.insert(symbol.clone(), inst_id);

        if inst.listability.should_create_order_book() {
            self.markets.entry(symbol.clone()).or_insert_with(|| {
                let pricer: Arc<dyn Pricer<FinancialProduct>> = match &inst.state() {
                    InstrumentRuntime::Bond(b) if b.bond_type() == BondType::Government => {
                        let feeds = self.pricing_feeds.clone().expect("attach_pricing_feeds before listing");
                        let method = TermStructureMethod::Bootstrapped;
                        let terms = BondPricingTerms::from(b);
                        Arc::new(GovTermStructurePricer::new(terms, method, feeds))
                    }
                    _ => Arc::new(NoOpPricer),
                };
                MarketType::Financial(MarketGeneric {
                    key: inst_id,
                    book: OrderBook::new(),
                    pricer,
                    settlement: Arc::new(NoopSettlement),
                })
            });
        }
        self.update_index_only(inst_id, inst);
        symbol
    }

    pub fn ensure_goods_market(&mut self, good_id: GoodId, good_name: &str) -> Symbol {
        let symbol = self.symbol_registry.register_good(good_id, good_name);
        self.good_to_symbol.insert(good_id, symbol.clone());
        self.symbol_to_good.insert(symbol.clone(), good_id);

        self.markets.entry(symbol.clone()).or_insert_with(|| {
            let pricer: Arc<dyn Pricer<GoodProduct>> = if let Some(feeds) = self.pricing_feeds.clone() {
                Arc::new(CostPlusGoodsPricer::new(feeds, CostPlusParams::default()))
            } else {
                Arc::new(NoOpPricer)
            };
            MarketType::Goods(MarketGeneric {
                key: good_id,
                book: OrderBook::new(),
                pricer,
                settlement: Arc::new(NoopSettlement),
            })
        });
        symbol
    }

    pub fn ensure_labour_market(&mut self, market_id: LabourMarketId, label: &str) -> Symbol {
        let symbol = self.symbol_registry.register_labour(market_id, label);
        self.labour_to_symbol.insert(market_id, symbol.clone());
        self.markets.entry(symbol.clone()).or_insert_with(|| MarketType::Labour(LabourMarket::default()));
        symbol
    }
    pub fn ensure_listed(&mut self, instrument_id: InstrumentId, inst: &Instrument) -> Symbol {
        self.ensure_financial_market(instrument_id, inst)
    }

    fn update_index_only(&mut self, inst_id: InstrumentId, inst: &Instrument) {
        let listing_key = listing_key_from_instrument(inst);
        self.index.by_listing.entry(listing_key).or_default().push(inst_id);

        if let Some(agent) = instrument_listing_agent(inst) {
            self.index.by_issuer.entry(agent).or_default().push(inst_id);
        }

        if let InstrumentRuntime::Bond(b) = inst.state() {
            let tenor_bucket = TenorBucket::from_years(b.original_tenor_years());
            self.index
                .by_rating_and_tenor
                .entry((b.rating.unwrap_or(CreditRating::government_aaa()), tenor_bucket))
                .or_default()
                .push(inst_id);

            self.index.by_bond_type.entry(b.bond_type()).or_default().push(inst_id);
        }
    }
    pub fn financial_market(&self, id: &InstrumentId) -> Option<&OrderBook> {
        let symbol = self.inst_to_symbol.get(id)?;
        self.markets.get(symbol).and_then(|market| match market {
            MarketType::Financial(m) => Some(&m.book),
            _ => None,
        })
    }

    pub fn financial_market_mut(&mut self, id: &InstrumentId) -> Option<&mut OrderBook> {
        let symbol = self.inst_to_symbol.get(id)?.clone();
        self.markets.get_mut(&symbol).and_then(|market| match market {
            MarketType::Financial(m) => Some(&mut m.book),
            _ => None,
        })
    }

    pub fn goods_market_mut(&mut self, id: &GoodId) -> Option<&mut MarketGeneric<GoodProduct>> {
        let symbol = self.good_to_symbol.get(id)?.clone();
        self.markets.get_mut(&symbol).and_then(|market| match market {
            MarketType::Goods(m) => Some(m),
            _ => None,
        })
    }

    pub fn labour_market_mut(&mut self, id: &LabourMarketId) -> Option<&mut LabourMarket> {
        let symbol = self.labour_to_symbol.get(id)?.clone();
        self.markets.get_mut(&symbol).and_then(|market| match market {
            MarketType::Labour(m) => Some(m),
            _ => None,
        })
    }
    pub fn fair_price_for_good(&self, good_id: &GoodId) -> Option<Money> {
        self.good_to_symbol.get(good_id).and_then(|symbol| self.markets.get(symbol)).and_then(|market| {
            if let MarketType::Goods(goods_market) = market { goods_market.pricer.fair_price(good_id) } else { None }
        })
    }

    pub fn conduct_dutch_auction(
        &mut self, auction_id: &Uuid, instruments: &std::collections::HashMap<InstrumentId, Instrument>,
    ) -> Result<Vec<Trade>, String> {
        let mut trades = Vec::new();
        let auction = match self.open_auctions.get_mut(auction_id) {
            Some(a) => a,
            None => return Err(format!("Auction {} not found", auction_id)),
        };

        if auction.status != AuctionStatus::Open || auction.bids.is_empty() {
            auction.status = AuctionStatus::Closed;
            return Ok(trades);
        }

        let instrument = instruments
            .get(&auction.instrument_id)
            .ok_or_else(|| format!("Auction instrument {} not found", auction.instrument_id))?;
        let issuer = if let InstrumentRuntime::Bond(details) = &instrument.state() {
            details.issuer
        } else {
            auction.status = AuctionStatus::Closed;
            return Err("Auction instrument is not a debt security".into());
        };
        let symbol = self
            .inst_to_symbol
            .get(&instrument.instrument_id())
            .cloned()
            .ok_or_else(|| format!("No symbol registered for instrument {}", instrument.instrument_id()))?;

        auction.bids.sort_by(|a, b| b.price.cmp(&a.price));

        let mut quantity_filled: u32 = 0;
        let mut clearing_price = Money::ZERO;
        let mut winning_bids: Vec<(AgentId, u32)> = Vec::new();

        for bid in &auction.bids {
            if quantity_filled >= auction.quantity_offered {
                break;
            }
            let quantity_to_fill = (auction.quantity_offered - quantity_filled).min(bid.quantity);
            winning_bids.push((bid.agent_id, quantity_to_fill));
            quantity_filled += quantity_to_fill;
            clearing_price = bid.price;
        }

        if !winning_bids.is_empty() {
            for (winner_id, quantity) in winning_bids {
                trades.push(Trade {
                    trade_id: Uuid::new_v4(),
                    market_id: symbol.clone(),
                    buyer: winner_id,
                    seller: issuer,
                    quantity: quantity as f64,
                    price: clearing_price,
                });
            }
        }
        auction.status = AuctionStatus::Closed;
        Ok(trades)
    }

    pub fn validate_and_submit_order(
        &mut self, order: Order, csd: &CentralSecuritiesDepository,
    ) -> Result<Vec<Trade>, String> {
        let symbol = order.market.clone();
        let market = self.markets.get_mut(&symbol).ok_or_else(|| format!("Market not found for symbol {}", symbol))?;

        match market {
            MarketType::Financial(mkt) => {
                if csd.is_security(&mkt.key) {
                    mkt.book.validate_sell_order(&order, csd, &mkt.key)?;
                }
                return mkt.book.submit_order_with_validation(order, &symbol, csd, &mkt.key);
            }
            MarketType::Goods(mkt) => {
                return Ok(mkt.book.submit_order(order, &symbol));
            }
            MarketType::Labour(_) => Err("Use dedicated labour APIs".into()),
        }
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
        &self, _key: &<FinancialProduct as Product>::Key, _y: f64,
    ) -> Option<<FinancialProduct as Product>::Quote> {
        None
    }
    fn yield_from_price(
        &self, _key: &<FinancialProduct as Product>::Key, _px: <FinancialProduct as Product>::Quote,
    ) -> Option<f64> {
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
struct NoopSettlement;
impl<P: Product> Settlement<P> for NoopSettlement {
    fn dvp(
        &self, _key: &P::Key, _buyer: AgentId, _seller: AgentId, _qty: P::Lot, _price: P::Quote,
    ) -> Result<(), String> {
        Ok(())
    }
}

impl Serialize for Exchange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Exchange", 2)?;
        state.serialize_field("index", &self.index)?;
        state.serialize_field("tape", &self.tape)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Exchange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let _ = serde::de::IgnoredAny::deserialize(deserializer)?;
        Err(serde::de::Error::custom(
            "Exchange does not support direct deserialization. \
             Serialize/deserialize ExchangeDto instead, and reconstruct Exchange at runtime.",
        ))
    }
}
