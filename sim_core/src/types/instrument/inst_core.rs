use crate::prelude::*;
// Runtime layer for instruments: concrete, mutable state composed with identifiers and markets.
use crate::types::instrument::archetypes::{
    BondArchetype, BondType, CashFlow, CashType, Covenant, CreditRating, LoanArchetype, RateIndex,
};
use crate::types::instrument::credit::{ConsumerLoanCategory, CreditFacilityState, LoanType, TradeCreditState};
use crate::types::instrument::inst_registry::{LotId, SeriesId, TemplateId};
use crate::types::instrument::instrument::{
    Currency, DerivativeState, EquityState, RealAssetState, RepoState, StructuredTrancheState,
};
use crate::types::money::Money;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VenueType {
    CentralLimitOrderBook,
    PostedRates,
    OTC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoneyMarketSegment {
    Interbank,
    SovereignShortTerm,
    CorporateShortTerm,
    Repo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapitalMarketSegment {
    Equity,
    SovereignLongTerm,
    CorporateCredit,
    StructuredFinance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DerivativesMarketSegment {
    Options,
    Futures,
    Swaps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstrumentMarket {
    MoneyMarket(MoneyMarketSegment),
    CapitalMarket(CapitalMarketSegment),
    DerivativesMarket(DerivativesMarketSegment),
    Unlisted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketProfile {
    pub market: InstrumentMarket,
    pub default_venue_type: Option<VenueType>,
    pub is_exchange_tradeable: bool,
    pub requires_csd_custody: bool,
}

impl MarketProfile {
    pub fn unlisted() -> Self {
        Self {
            market: InstrumentMarket::Unlisted,
            default_venue_type: None,
            is_exchange_tradeable: false,
            requires_csd_custody: false,
        }
    }

    pub fn from_market(market: InstrumentMarket) -> Self {
        let (default_venue_type, is_exchange_tradeable, requires_csd_custody) = match market {
            InstrumentMarket::MoneyMarket(_) => (Some(VenueType::PostedRates), true, false),
            InstrumentMarket::CapitalMarket(_) => (Some(VenueType::CentralLimitOrderBook), true, true),
            InstrumentMarket::DerivativesMarket(_) => (Some(VenueType::CentralLimitOrderBook), true, true),
            InstrumentMarket::Unlisted => (None, false, false),
        };

        Self { market, default_venue_type, is_exchange_tradeable, requires_csd_custody }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstrumentIdentifiers {
    pub instrument_id: InstrumentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<TemplateId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_id: Option<SeriesId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lot_id: Option<LotId>,
}

impl InstrumentIdentifiers {
    pub fn new(instrument_id: InstrumentId) -> Self {
        Self { instrument_id, template_id: None, series_id: None, lot_id: None }
    }

    pub fn instrument_id(&self) -> InstrumentId {
        self.instrument_id
    }

    pub fn template_id(&self) -> Option<TemplateId> {
        self.template_id
    }

    pub fn series_id(&self) -> Option<SeriesId> {
        self.series_id
    }

    pub fn lot_id(&self) -> Option<LotId> {
        self.lot_id
    }

    pub fn with_template(mut self, template_id: TemplateId) -> Self {
        self.template_id = Some(template_id);
        self
    }

    pub fn with_series(mut self, series_id: SeriesId) -> Self {
        self.series_id = Some(series_id);
        self
    }

    pub fn with_lot(mut self, lot_id: LotId) -> Self {
        self.lot_id = Some(lot_id);
        self
    }
}

impl From<InstrumentId> for InstrumentIdentifiers {
    fn from(value: InstrumentId) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Listability {
    Unlisted,
    Listed(VenueType),
}

impl Listability {
    pub fn listed(venue: VenueType) -> Self {
        Listability::Listed(venue)
    }

    pub fn is_listed(&self) -> bool {
        matches!(self, Listability::Listed(_))
    }

    pub fn should_create_order_book(&self) -> bool {
        matches!(self, Listability::Listed(VenueType::CentralLimitOrderBook))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstrumentCore<S> {
    pub identifiers: InstrumentIdentifiers,
    pub market_profile: MarketProfile,
    pub listability: Listability,
    pub state: S,
}

impl<S> InstrumentCore<S> {
    pub fn new(
        identifiers: InstrumentIdentifiers, market_profile: MarketProfile, listability: Listability, state: S,
    ) -> Self {
        Self { identifiers, market_profile, listability, state }
    }

    pub fn instrument_id(&self) -> InstrumentId {
        self.identifiers.instrument_id
    }

    pub fn template_id(&self) -> Option<TemplateId> {
        self.identifiers.template_id
    }

    pub fn series_id(&self) -> Option<SeriesId> {
        self.identifiers.series_id
    }

    pub fn lot_id(&self) -> Option<LotId> {
        self.identifiers.lot_id
    }

    pub fn map_state<T>(self, map: impl FnOnce(S) -> T) -> InstrumentCore<T> {
        InstrumentCore {
            identifiers: self.identifiers,
            market_profile: self.market_profile,
            listability: self.listability,
            state: map(self.state),
        }
    }

    pub fn market_profile(&self) -> &MarketProfile {
        &self.market_profile
    }

    pub fn market_profile_mut(&mut self) -> &mut MarketProfile {
        &mut self.market_profile
    }

    pub fn set_market_profile(&mut self, profile: MarketProfile) {
        self.market_profile = profile;
    }

    pub fn identifiers(&self) -> &InstrumentIdentifiers {
        &self.identifiers
    }

    pub fn identifiers_mut(&mut self) -> &mut InstrumentIdentifiers {
        &mut self.identifiers
    }

    pub fn listability(&self) -> &Listability {
        &self.listability
    }

    pub fn set_listability(&mut self, listability: Listability) {
        self.listability = listability;
    }

    pub fn with_listability(mut self, listability: Listability) -> Self {
        self.listability = listability;
        self
    }

    pub fn state(&self) -> &S {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut S {
        &mut self.state
    }

    pub fn into_state(self) -> S {
        self.state
    }

    pub fn into_parts(self) -> (InstrumentIdentifiers, MarketProfile, Listability, S) {
        (self.identifiers, self.market_profile, self.listability, self.state)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstrumentRuntime {
    Cash(CashState),
    Bond(BondState),
    Credit(CreditState),
    Equity(EquityState),
    Structured(StructuredTrancheState),
    Derivative(DerivativeState),
    Repo(RepoState),
    RealAsset(RealAssetState),
}

impl InstrumentRuntime {
    pub fn label(&self) -> &'static str {
        match self {
            InstrumentRuntime::Cash(_) => "Cash",
            InstrumentRuntime::Bond(_) => "Bond",
            InstrumentRuntime::Credit(state) => state.label(),
            InstrumentRuntime::Equity(_) => "Equity",
            InstrumentRuntime::Structured(tranche) => tranche.tranche_type.label(),
            InstrumentRuntime::Derivative(state) => state.label(),
            InstrumentRuntime::Repo(_) => "Repurchase Agreement",
            InstrumentRuntime::RealAsset(asset) => asset.label(),
        }
    }

    pub fn face_value(&self) -> Option<Money> {
        match self {
            InstrumentRuntime::Cash(_) => None,
            InstrumentRuntime::Bond(state) => Some(state.outstanding_notional()),
            InstrumentRuntime::Credit(state) => Some(state.exposure()),
            InstrumentRuntime::Equity(state) => {
                state.profile.par_value.map(|par| par * state.outstanding_shares as f64)
            }
            InstrumentRuntime::Structured(state) => Some(state.outstanding_notional),
            InstrumentRuntime::Derivative(state) => state.notional,
            InstrumentRuntime::Repo(state) => Some(state.cash_principal),
            InstrumentRuntime::RealAsset(asset) => asset.face_value(),
        }
    }

    pub fn unit_par_value(&self) -> Option<Money> {
        match self {
            InstrumentRuntime::Cash(_) => Some(Money::ONE),
            InstrumentRuntime::Bond(state) => Some(state.archetype.face_value),
            InstrumentRuntime::Credit(_) => Some(Money::ONE),
            InstrumentRuntime::Equity(state) => state.profile.par_value.or(Some(Money::ONE)),
            InstrumentRuntime::Structured(state) => Some(state.face_value),
            InstrumentRuntime::Derivative(state) => state.notional,
            InstrumentRuntime::Repo(state) => Some(state.cash_principal),
            InstrumentRuntime::RealAsset(asset) => asset.face_value(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CreditState {
    Loan(LoanState),
    ConsumerLoan {
        category: ConsumerLoanCategory,
        loan: LoanState,
    },
    ConsumerCreditCard(CreditFacilityState),
    TradeCredit(TradeCreditState),
    #[serde(alias = "CreditLine")]
    Facility(CreditFacilityState),
}

impl CreditState {
    pub fn exposure(&self) -> Money {
        match self {
            CreditState::Loan(loan) => loan.outstanding_principal,
            CreditState::ConsumerLoan { loan, .. } => loan.outstanding_principal,
            CreditState::ConsumerCreditCard(facility) => facility.drawn_amount,
            CreditState::Facility(facility) => facility.drawn_amount,
            CreditState::TradeCredit(trade) => trade.amount,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            CreditState::Loan(_) => "Corporate Loan",
            CreditState::ConsumerLoan { .. } => "Consumer Loan",
            CreditState::ConsumerCreditCard(_) => "Credit Card",
            CreditState::TradeCredit(_) => "Trade Credit",
            CreditState::Facility(_) => "Credit Facility",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashState {
    pub issuer: AgentId,
    pub cash_type: CashType,
    pub currency: Currency,
    pub interest_bps: BasisPoints,
}

impl CashState {
    pub fn new(issuer: AgentId, cash_type: CashType, currency: Currency, interest_bps: BasisPoints) -> Self {
        Self { issuer, cash_type, currency, interest_bps }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondState {
    pub issuer: AgentId,
    pub archetype: BondArchetype,
    pub issue_date: NaiveDate,
    pub maturity_date: NaiveDate,
    pub outstanding_units: f64,
    pub last_accrual_date: Option<NaiveDate>,
    pub rating: Option<CreditRating>,
}

impl BondState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        issuer: AgentId, archetype: BondArchetype, issue_date: NaiveDate, maturity_date: NaiveDate,
        outstanding_units: f64, last_accrual_date: Option<NaiveDate>, rating: Option<CreditRating>,
    ) -> Self {
        Self { issuer, archetype, issue_date, maturity_date, outstanding_units, last_accrual_date, rating }
    }

    pub fn outstanding_notional(&self) -> Money {
        self.archetype.face_value * self.outstanding_units
    }

    pub fn bond_type(&self) -> BondType {
        self.archetype.bond_type
    }

    pub fn cash_flow_type(&self) -> CashFlow {
        self.archetype.cash_flow_type
    }

    pub fn remaining_tenor_years(&self, as_of: NaiveDate) -> f64 {
        let days = (self.maturity_date - as_of).num_days().max(0);
        days as f64 / 365.25
    }

    pub fn original_tenor_years(&self) -> f64 {
        let days = (self.maturity_date - self.issue_date).num_days().max(0);
        days as f64 / 365.25
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanState {
    pub loan_id: Uuid,
    pub lender: AgentId,
    pub borrower: AgentId,
    pub facility_id: Option<Uuid>,
    pub loan_type: LoanType,
    pub archetype: LoanArchetype,
    pub outstanding_principal: Money,
    pub origination_date: NaiveDate,
    pub maturity_date: NaiveDate,
    pub next_payment_date: NaiveDate,
    pub last_accrual_date: NaiveDate,
    pub collateral: Vec<LienId>,
    pub covenants: Vec<Covenant>,
    pub rating: Option<CreditRating>,
    pub impairment: ImpairmentState,
    pub accrued_interest: Money,
    pub unamortized_fees: Money,
}

impl LoanState {
    pub fn new(
        loan_id: Uuid, lender: AgentId, borrower: AgentId, facility_id: Option<Uuid>, loan_type: LoanType,
        archetype: LoanArchetype, outstanding_principal: Money, origination_date: NaiveDate, maturity_date: NaiveDate,
        next_payment_date: NaiveDate, last_accrual_date: NaiveDate, collateral: Vec<LienId>, covenants: Vec<Covenant>,
        rating: Option<CreditRating>, impairment: ImpairmentState, accrued_interest: Money, unamortized_fees: Money,
    ) -> Self {
        Self {
            loan_id,
            lender,
            borrower,
            facility_id,
            loan_type,
            archetype,
            outstanding_principal,
            origination_date,
            maturity_date,
            next_payment_date,
            last_accrual_date,
            collateral,
            covenants,
            rating,
            impairment,
            accrued_interest,
            unamortized_fees,
        }
    }

    pub fn principal(&self) -> Money {
        self.archetype.principal
    }

    pub fn rate_index(&self) -> RateIndex {
        self.archetype.rate_structure.base_rate
    }

    pub fn spread_bps(&self) -> BasisPoints {
        self.archetype.rate_structure.spread_bps
    }

    pub fn rate_floor_bps(&self) -> Option<BasisPoints> {
        self.archetype.rate_structure.floor_bps
    }

    pub fn rate_cap_bps(&self) -> Option<BasisPoints> {
        self.archetype.rate_structure.cap_bps
    }
}

impl InstrumentCore<InstrumentRuntime> {
    pub fn face_value(&self) -> Option<Money> {
        self.state.face_value()
    }

    pub fn unit_par_value(&self) -> Option<Money> {
        self.state.unit_par_value()
    }

    pub fn label(&self) -> &'static str {
        self.state.label()
    }

    pub fn type_as_string(&self) -> &'static str {
        self.label()
    }
}
