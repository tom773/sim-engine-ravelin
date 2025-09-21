use crate::prelude::*;
use crate::types::instrument::archetypes::{
    BondArchetype, BondType, CashFlow, CashType, CreditRating, DerivativesMarketSegment, InstrumentMarket,
    MoneyMarketSegment,
};
use crate::types::instrument::inst_core::{
    BondState, CapitalMarketSegment, CashState, InstrumentIdentifiers, Listability, MarketProfile, VenueType,
};
use crate::types::instrument::inst_registry::{LotId, SeriesId, TemplateId};
use crate::types::instrument::instrument::{
    DerivativeContract, DerivativeState, DividendPolicy, EquityClass, EquityProfile, EquityState, RealAssetState,
    RepoState, StructuredTrancheState, StructuredTrancheType, UnderlyingAsset,
};
use crate::types::instrument::money::Rate;
use crate::types::instrument::{Instrument, InstrumentRuntime};
use chrono::{Months, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::num::NonZeroU32;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("maturity must be after issue (issue={0}, maturity={1})")]
    BadDates(NaiveDate, NaiveDate),
    #[error("coupon-bearing bonds must have payment frequency > 0")]
    BadFrequency,
    #[error("zero-coupon bonds must have coupon == 0 bps")]
    ZeroCouponHasCoupon,
}

fn classify_market(bond_type: BondType, issue: NaiveDate, maturity: NaiveDate) -> InstrumentMarket {
    let days = (maturity - issue).num_days();
    let money = days <= 365;
    match (bond_type, money) {
        (BondType::Government, true)
        | (BondType::Municipal, true)
        | (BondType::Agency, true)
        | (BondType::Supranational, true)
        | (BondType::InterbankLoan, true) => InstrumentMarket::MoneyMarket(MoneyMarketSegment::SovereignShortTerm),
        (BondType::Corporate, true) => InstrumentMarket::MoneyMarket(MoneyMarketSegment::CorporateShortTerm),
        (BondType::Government, false)
        | (BondType::Municipal, false)
        | (BondType::Agency, false)
        | (BondType::Supranational, false)
        | (BondType::InterbankLoan, false) => InstrumentMarket::CapitalMarket(CapitalMarketSegment::SovereignLongTerm),
        (BondType::Corporate, false) => InstrumentMarket::CapitalMarket(CapitalMarketSegment::CorporateCredit),
    }
}

fn profile_for_market(market: InstrumentMarket) -> MarketProfile {
    MarketProfile::from_market(market)
}

fn derivatives_market(contract: &DerivativeContract) -> InstrumentMarket {
    let segment = match contract {
        DerivativeContract::Option(_) => DerivativesMarketSegment::Options,
        DerivativeContract::Future(_) => DerivativesMarketSegment::Futures,
        DerivativeContract::Custom { .. } => DerivativesMarketSegment::Swaps,
    };

    InstrumentMarket::DerivativesMarket(segment)
}

pub struct CashBuilder {
    identifiers: InstrumentIdentifiers,
    market_profile: Option<MarketProfile>,
    listability: Listability,
    state: CashState,
}

impl Instrument {
    pub fn cash(
        id: InstrumentId, issuer: AgentId, cash_type: CashType, currency: Currency, rate: BasisPoints,
    ) -> CashBuilder {
        CashBuilder {
            identifiers: InstrumentIdentifiers::new(id),
            market_profile: None,
            listability: Listability::Unlisted,
            state: CashState::new(issuer, cash_type, currency, rate),
        }
    }
}

impl CashBuilder {
    pub fn template(mut self, template_id: TemplateId) -> Self {
        self.identifiers = self.identifiers.with_template(template_id);
        self
    }

    pub fn series(mut self, series_id: SeriesId) -> Self {
        self.identifiers = self.identifiers.with_series(series_id);
        self
    }

    pub fn lot(mut self, lot_id: LotId) -> Self {
        self.identifiers = self.identifiers.with_lot(lot_id);
        self
    }

    pub fn market(mut self, market: InstrumentMarket) -> Self {
        self.market_profile = Some(profile_for_market(market));
        self
    }

    pub fn market_profile(mut self, profile: MarketProfile) -> Self {
        self.market_profile = Some(profile);
        self
    }

    pub fn listability(mut self, listability: Listability) -> Self {
        self.listability = listability;
        self
    }

    pub fn build(self) -> Instrument {
        let Self { identifiers, market_profile, listability, state } = self;

        let market_profile = market_profile
            .unwrap_or_else(|| profile_for_market(InstrumentMarket::MoneyMarket(MoneyMarketSegment::Interbank)));

        Instrument::new(identifiers, market_profile, listability, InstrumentRuntime::Cash(state))
    }
}

pub struct BondBuilder {
    identifiers: InstrumentIdentifiers,
    market_profile: Option<MarketProfile>,
    listability: Listability,
    issuer: AgentId,
    issue_date: NaiveDate,
    maturity_date: NaiveDate,
    archetype: BondArchetype,
    outstanding_units: f64,
    last_accrual_date: Option<NaiveDate>,
    rating: Option<CreditRating>,
}

impl Instrument {
    pub fn bond(
        id: InstrumentId, issuer: AgentId, bond_type: BondType, face_value: Money, issue_date: NaiveDate,
        maturity_date: NaiveDate,
    ) -> BondBuilder {
        let archetype = BondArchetype {
            bond_type,
            cash_flow_type: CashFlow::Fixed,
            day_count: DayCount::ActAct,
            face_value,
            coupon_rate_bps: dec!(0.0),
            frequency_per_year: 2,
            covenants: Vec::new(),
        };

        BondBuilder {
            identifiers: InstrumentIdentifiers::new(id),
            market_profile: None,
            listability: Listability::Unlisted,
            issuer,
            issue_date,
            maturity_date,
            archetype,
            outstanding_units: 0.0,
            last_accrual_date: None,
            rating: None,
        }
    }
}

impl BondBuilder {
    pub fn template(mut self, template_id: TemplateId) -> Self {
        self.identifiers = self.identifiers.with_template(template_id);
        self
    }

    pub fn series(mut self, series_id: SeriesId) -> Self {
        self.identifiers = self.identifiers.with_series(series_id);
        self
    }

    pub fn lot(mut self, lot_id: LotId) -> Self {
        self.identifiers = self.identifiers.with_lot(lot_id);
        self
    }

    pub fn zero_coupon(mut self) -> Self {
        self.archetype.cash_flow_type = CashFlow::Zero;
        self.archetype.coupon_rate_bps = dec!(0.0);
        self.archetype.frequency_per_year = 0;
        self
    }

    pub fn fixed(mut self) -> Self {
        self.archetype.cash_flow_type = CashFlow::Fixed;
        self
    }

    pub fn floating(mut self) -> Self {
        self.archetype.cash_flow_type = CashFlow::Floating;
        self
    }

    pub fn coupon_bps(mut self, bps: BasisPoints) -> Self {
        self.archetype.coupon_rate_bps = bps;
        self
    }

    pub fn frequency(mut self, per_year: u32) -> Self {
        self.archetype.frequency_per_year = per_year;
        self
    }

    pub fn day_count(mut self, day_count: DayCount) -> Self {
        self.archetype.day_count = day_count;
        self
    }

    pub fn rating(mut self, rating: CreditRating) -> Self {
        self.rating = Some(rating);
        self
    }

    pub fn outstanding_units(mut self, units: f64) -> Self {
        self.outstanding_units = units;
        self
    }

    pub fn last_accrual_date(mut self, date: NaiveDate) -> Self {
        self.last_accrual_date = Some(date);
        self
    }

    pub fn market(mut self, market: InstrumentMarket) -> Self {
        self.market_profile = Some(profile_for_market(market));
        self
    }

    pub fn market_profile(mut self, profile: MarketProfile) -> Self {
        self.market_profile = Some(profile);
        self
    }

    pub fn listability(mut self, listability: Listability) -> Self {
        self.listability = listability;
        self
    }

    pub fn auto_market(mut self) -> Self {
        let market = classify_market(self.archetype.bond_type, self.issue_date, self.maturity_date);
        self.market_profile = Some(profile_for_market(market));
        self.listability = Listability::Listed(VenueType::CentralLimitOrderBook);
        self
    }

    pub fn build(self) -> Result<Instrument, BuildError> {
        if self.maturity_date <= self.issue_date {
            return Err(BuildError::BadDates(self.issue_date, self.maturity_date));
        }

        if matches!(self.archetype.cash_flow_type, CashFlow::Zero) && self.archetype.coupon_rate_bps != dec!(0.0) {
            return Err(BuildError::ZeroCouponHasCoupon);
        }

        if matches!(self.archetype.cash_flow_type, CashFlow::Fixed | CashFlow::Floating)
            && self.archetype.frequency_per_year == 0
        {
            return Err(BuildError::BadFrequency);
        }

        let market_profile = self.market_profile.unwrap_or_else(|| {
            profile_for_market(classify_market(self.archetype.bond_type, self.issue_date, self.maturity_date))
        });

        let archetype = self.archetype;

        let state = BondState::new(
            self.issuer,
            archetype,
            self.issue_date,
            self.maturity_date,
            self.outstanding_units,
            self.last_accrual_date,
            self.rating,
        );

        Ok(Instrument::new(self.identifiers, market_profile, self.listability, InstrumentRuntime::Bond(state)))
    }
}

pub struct EquityBuilder {
    identifiers: InstrumentIdentifiers,
    market_profile: Option<MarketProfile>,
    listability: Listability,
    profile: EquityProfile,
    outstanding_shares: u64,
}

impl Instrument {
    pub fn equity(
        id: InstrumentId, issuer: AgentId, share_class: EquityClass, outstanding_shares: u64,
    ) -> EquityBuilder {
        EquityBuilder {
            identifiers: InstrumentIdentifiers::new(id),
            market_profile: None,
            listability: Listability::Listed(VenueType::CentralLimitOrderBook),
            profile: EquityProfile {
                issuer,
                share_class,
                authorized_shares: None,
                par_value: None,
                dividend_policy: None,
            },
            outstanding_shares,
        }
    }
}

impl EquityBuilder {
    pub fn template(mut self, template_id: TemplateId) -> Self {
        self.identifiers = self.identifiers.with_template(template_id);
        self
    }

    pub fn series(mut self, series_id: SeriesId) -> Self {
        self.identifiers = self.identifiers.with_series(series_id);
        self
    }

    pub fn lot(mut self, lot_id: LotId) -> Self {
        self.identifiers = self.identifiers.with_lot(lot_id);
        self
    }

    pub fn authorized_shares(mut self, shares: u64) -> Self {
        self.profile.authorized_shares = Some(shares);
        self
    }

    pub fn par_value(mut self, value: Money) -> Self {
        self.profile.par_value = Some(value);
        self
    }

    pub fn dividend_policy(mut self, policy: DividendPolicy) -> Self {
        self.profile.dividend_policy = Some(policy);
        self
    }

    pub fn share_class(mut self, class: EquityClass) -> Self {
        self.profile.share_class = class;
        self
    }

    pub fn outstanding_shares(mut self, shares: u64) -> Self {
        self.outstanding_shares = shares;
        self
    }

    pub fn market(mut self, market: InstrumentMarket) -> Self {
        self.market_profile = Some(profile_for_market(market));
        self
    }

    pub fn market_profile(mut self, profile: MarketProfile) -> Self {
        self.market_profile = Some(profile);
        self
    }

    pub fn listability(mut self, listability: Listability) -> Self {
        self.listability = listability;
        self
    }

    pub fn build(self) -> Instrument {
        let market_profile = self
            .market_profile
            .unwrap_or_else(|| profile_for_market(InstrumentMarket::CapitalMarket(CapitalMarketSegment::Equity)));

        let state = EquityState { profile: self.profile, outstanding_shares: self.outstanding_shares };

        Instrument::new(self.identifiers, market_profile, self.listability, InstrumentRuntime::Equity(state))
    }
}

pub struct RepoBuilder {
    identifiers: InstrumentIdentifiers,
    market_profile: Option<MarketProfile>,
    listability: Listability,
    state: RepoState,
}

impl Instrument {
    #[allow(clippy::too_many_arguments)]
    pub fn repo(
        id: InstrumentId, lender: AgentId, borrower: AgentId, collateral_id: InstrumentId, collateral_quantity: f64,
        cash_principal: Money, interest_bps: BasisPoints, start_date: NaiveDate, end_date: NaiveDate,
    ) -> RepoBuilder {
        RepoBuilder {
            identifiers: InstrumentIdentifiers::new(id),
            market_profile: None,
            listability: Listability::Unlisted,
            state: RepoState {
                lender,
                borrower,
                collateral_id,
                collateral_quantity,
                cash_principal,
                interest_bps,
                start_date,
                end_date,
                haircut: Decimal::ZERO,
                open_term: false,
            },
        }
    }
}

impl RepoBuilder {
    pub fn template(mut self, template_id: TemplateId) -> Self {
        self.identifiers = self.identifiers.with_template(template_id);
        self
    }

    pub fn series(mut self, series_id: SeriesId) -> Self {
        self.identifiers = self.identifiers.with_series(series_id);
        self
    }

    pub fn lot(mut self, lot_id: LotId) -> Self {
        self.identifiers = self.identifiers.with_lot(lot_id);
        self
    }

    pub fn interest_bps(mut self, interest: BasisPoints) -> Self {
        self.state.interest_bps = interest;
        self
    }

    pub fn haircut(mut self, haircut: Rate) -> Self {
        self.state.haircut = haircut;
        self
    }

    pub fn open_term(mut self, open: bool) -> Self {
        self.state.open_term = open;
        self
    }

    pub fn collateral_quantity(mut self, quantity: f64) -> Self {
        self.state.collateral_quantity = quantity;
        self
    }

    pub fn maturity(mut self, end_date: NaiveDate) -> Self {
        self.state.end_date = end_date;
        self
    }

    pub fn market(mut self, market: InstrumentMarket) -> Self {
        self.market_profile = Some(profile_for_market(market));
        self
    }

    pub fn market_profile(mut self, profile: MarketProfile) -> Self {
        self.market_profile = Some(profile);
        self
    }

    pub fn listability(mut self, listability: Listability) -> Self {
        self.listability = listability;
        self
    }

    pub fn build(self) -> Instrument {
        let market_profile = self
            .market_profile
            .unwrap_or_else(|| profile_for_market(InstrumentMarket::MoneyMarket(MoneyMarketSegment::Repo)));

        Instrument::new(self.identifiers, market_profile, self.listability, InstrumentRuntime::Repo(self.state))
    }
}

pub struct StructuredBuilder {
    identifiers: InstrumentIdentifiers,
    market_profile: Option<MarketProfile>,
    listability: Listability,
    state: StructuredTrancheState,
}

impl Instrument {
    #[allow(clippy::too_many_arguments)]
    pub fn structured_tranche(
        id: InstrumentId, issuer: AgentId, tranche_type: StructuredTrancheType, attachment_point: Rate,
        detachment_point: Rate, face_value: Money, coupon_rate_bps: BasisPoints, maturity_date: NaiveDate,
        rating: CreditRating,
    ) -> StructuredBuilder {
        StructuredBuilder {
            identifiers: InstrumentIdentifiers::new(id),
            market_profile: None,
            listability: Listability::Listed(VenueType::CentralLimitOrderBook),
            state: StructuredTrancheState {
                issuer,
                tranche_label: None,
                tranche_type,
                attachment_point,
                detachment_point,
                face_value,
                outstanding_notional: face_value,
                coupon_rate_bps,
                maturity_date,
                rating,
            },
        }
    }
}

impl StructuredBuilder {
    pub fn template(mut self, template_id: TemplateId) -> Self {
        self.identifiers = self.identifiers.with_template(template_id);
        self
    }

    pub fn series(mut self, series_id: SeriesId) -> Self {
        self.identifiers = self.identifiers.with_series(series_id);
        self
    }

    pub fn lot(mut self, lot_id: LotId) -> Self {
        self.identifiers = self.identifiers.with_lot(lot_id);
        self
    }

    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.state.tranche_label = Some(label.into());
        self
    }

    pub fn outstanding_notional(mut self, notional: Money) -> Self {
        self.state.outstanding_notional = notional;
        self
    }

    pub fn coupon(mut self, coupon: BasisPoints) -> Self {
        self.state.coupon_rate_bps = coupon;
        self
    }

    pub fn maturity(mut self, maturity: NaiveDate) -> Self {
        self.state.maturity_date = maturity;
        self
    }

    pub fn rating(mut self, rating: CreditRating) -> Self {
        self.state.rating = rating;
        self
    }

    pub fn market(mut self, market: InstrumentMarket) -> Self {
        self.market_profile = Some(profile_for_market(market));
        self
    }

    pub fn market_profile(mut self, profile: MarketProfile) -> Self {
        self.market_profile = Some(profile);
        self
    }

    pub fn listability(mut self, listability: Listability) -> Self {
        self.listability = listability;
        self
    }

    pub fn build(self) -> Instrument {
        let market_profile = self.market_profile.unwrap_or_else(|| {
            profile_for_market(InstrumentMarket::CapitalMarket(CapitalMarketSegment::StructuredFinance))
        });

        Instrument::new(self.identifiers, market_profile, self.listability, InstrumentRuntime::Structured(self.state))
    }
}

pub struct DerivativeBuilder {
    identifiers: InstrumentIdentifiers,
    market_profile: Option<MarketProfile>,
    listability: Listability,
    state: DerivativeState,
}

impl Instrument {
    pub fn derivative(
        id: InstrumentId, issuer: AgentId, contract: DerivativeContract, underlying: UnderlyingAsset,
    ) -> DerivativeBuilder {
        DerivativeBuilder {
            identifiers: InstrumentIdentifiers::new(id),
            market_profile: None,
            listability: Listability::Listed(VenueType::CentralLimitOrderBook),
            state: DerivativeState {
                issuer,
                counterparty: None,
                contract,
                underlying,
                trade_date: None,
                expiry_date: None,
                settlement_date: None,
                notional: None,
                margin_requirement: None,
            },
        }
    }
}

impl DerivativeBuilder {
    pub fn template(mut self, template_id: TemplateId) -> Self {
        self.identifiers = self.identifiers.with_template(template_id);
        self
    }

    pub fn series(mut self, series_id: SeriesId) -> Self {
        self.identifiers = self.identifiers.with_series(series_id);
        self
    }

    pub fn lot(mut self, lot_id: LotId) -> Self {
        self.identifiers = self.identifiers.with_lot(lot_id);
        self
    }

    pub fn counterparty(mut self, counterparty: AgentId) -> Self {
        self.state.counterparty = Some(counterparty);
        self
    }

    pub fn trade_date(mut self, date: NaiveDate) -> Self {
        self.state.trade_date = Some(date);
        self
    }

    pub fn contract(mut self, contract: DerivativeContract) -> Self {
        self.state.contract = contract;
        self
    }

    pub fn expiry_date(mut self, date: NaiveDate) -> Self {
        self.state.expiry_date = Some(date);
        self
    }

    pub fn settlement_date(mut self, date: NaiveDate) -> Self {
        self.state.settlement_date = Some(date);
        self
    }

    pub fn notional(mut self, notional: Money) -> Self {
        self.state.notional = Some(notional);
        self
    }

    pub fn clear_notional(mut self) -> Self {
        self.state.notional = None;
        self
    }

    pub fn margin_requirement(mut self, margin: Money) -> Self {
        self.state.margin_requirement = Some(margin);
        self
    }

    pub fn market(mut self, market: InstrumentMarket) -> Self {
        self.market_profile = Some(profile_for_market(market));
        self
    }

    pub fn market_profile(mut self, profile: MarketProfile) -> Self {
        self.market_profile = Some(profile);
        self
    }

    pub fn listability(mut self, listability: Listability) -> Self {
        self.listability = listability;
        self
    }

    pub fn build(self) -> Instrument {
        let market_profile =
            self.market_profile.unwrap_or_else(|| profile_for_market(derivatives_market(&self.state.contract)));

        Instrument::new(self.identifiers, market_profile, self.listability, InstrumentRuntime::Derivative(self.state))
    }
}

pub struct RealAssetBuilder {
    identifiers: InstrumentIdentifiers,
    market_profile: Option<MarketProfile>,
    listability: Listability,
    state: RealAssetState,
}

impl Instrument {
    pub fn real_asset(id: InstrumentId, state: RealAssetState) -> RealAssetBuilder {
        RealAssetBuilder {
            identifiers: InstrumentIdentifiers::new(id),
            market_profile: None,
            listability: Listability::Unlisted,
            state,
        }
    }
}

impl RealAssetBuilder {
    pub fn template(mut self, template_id: TemplateId) -> Self {
        self.identifiers = self.identifiers.with_template(template_id);
        self
    }

    pub fn series(mut self, series_id: SeriesId) -> Self {
        self.identifiers = self.identifiers.with_series(series_id);
        self
    }

    pub fn lot(mut self, lot_id: LotId) -> Self {
        self.identifiers = self.identifiers.with_lot(lot_id);
        self
    }

    pub fn state(mut self, state: RealAssetState) -> Self {
        self.state = state;
        self
    }

    pub fn market(mut self, market: InstrumentMarket) -> Self {
        self.market_profile = Some(profile_for_market(market));
        self
    }

    pub fn market_profile(mut self, profile: MarketProfile) -> Self {
        self.market_profile = Some(profile);
        self
    }

    pub fn listability(mut self, listability: Listability) -> Self {
        self.listability = listability;
        self
    }

    pub fn build(self) -> Instrument {
        let market_profile = self.market_profile.unwrap_or_else(|| profile_for_market(InstrumentMarket::Unlisted));

        Instrument::new(self.identifiers, market_profile, self.listability, InstrumentRuntime::RealAsset(self.state))
    }
}

pub fn today() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

#[derive(Debug, Clone)]
pub enum MarketChoice {
    Auto,
    Set(MarketProfile),
}

#[derive(Debug, Clone, Copy)]
pub enum BondTerms {
    Zero,
    Fixed { coupon_rate_bps: BasisPoints, frequency: NonZeroU32, day_count: DayCount },
    Floating { spread_bps: BasisPoints, reset_freq: NonZeroU32, day_count: DayCount },
}

impl Instrument {
    #[allow(clippy::too_many_arguments)]
    pub fn bond_full(
        id: InstrumentId, issuer: AgentId, bond_type: BondType, face_value: Money, issue_date: NaiveDate,
        maturity_date: NaiveDate, terms: BondTerms, rating: CreditRating, market: MarketChoice,
    ) -> Result<Self, BuildError> {
        let builder = Instrument::bond(id, issuer, bond_type, face_value, issue_date, maturity_date);

        let builder = match terms {
            BondTerms::Zero => builder.zero_coupon(),
            BondTerms::Fixed { coupon_rate_bps, frequency, day_count } => {
                builder.fixed().coupon_bps(coupon_rate_bps).frequency(frequency.get()).day_count(day_count)
            }
            BondTerms::Floating { spread_bps, reset_freq, day_count } => {
                builder.floating().coupon_bps(spread_bps).frequency(reset_freq.get()).day_count(day_count)
            }
        };

        let builder = builder.rating(rating);

        let builder = match market {
            MarketChoice::Auto => builder.auto_market(),
            MarketChoice::Set(profile) => builder.market_profile(profile),
        };

        builder.build()
    }

    pub fn bond_fixed(
        id: InstrumentId, issuer: AgentId, bond_type: BondType, face_value: Money, issue_date: NaiveDate,
        maturity_date: NaiveDate, coupon_rate_bps: BasisPoints, frequency: NonZeroU32, day_count: DayCount,
        rating: CreditRating, market: MarketChoice,
    ) -> Result<Self, BuildError> {
        Self::bond_full(
            id,
            issuer,
            bond_type,
            face_value,
            issue_date,
            maturity_date,
            BondTerms::Fixed { coupon_rate_bps, frequency, day_count },
            rating,
            market,
        )
    }

    pub fn gov_bond(tenor_years: f64, coupon_rate_bps: BasisPoints) -> Result<Self, BuildError> {
        let issue = today();
        let years = years_from_f64(tenor_years).ok_or_else(|| BuildError::BadDates(issue, issue))?;
        let maturity = add_years(issue, years).ok_or_else(|| BuildError::BadDates(issue, issue))?;

        Self::bond_full(
            InstrumentId(Uuid::new_v4()),
            AgentId(Uuid::new_v4()),
            BondType::Government,
            Money::from(1_000i64),
            issue,
            maturity,
            BondTerms::Fixed {
                coupon_rate_bps,
                frequency: NonZeroU32::new(2).expect("frequency > 0"),
                day_count: DayCount::ActAct,
            },
            CreditRating::Government(SpCreditRating::AAA),
            MarketChoice::Auto,
        )
    }

    pub fn gov_bill_months(gov: Government, months: NonZeroU32) -> Result<Self, BuildError> {
        let issue = today();
        let maturity =
            issue.checked_add_months(Months::new(months.get())).ok_or_else(|| BuildError::BadDates(issue, issue))?;

        Self::bond_full(
            InstrumentId(Uuid::new_v4()),
            gov.id,
            BondType::Government,
            Money::from(1_000i64),
            issue,
            maturity,
            BondTerms::Zero,
            CreditRating::Government(SpCreditRating::AAA),
            MarketChoice::Auto,
        )
    }

    pub fn corp_bond(issuer: AgentId, tenor_years: f64) -> Result<Self, BuildError> {
        let issue = today();
        let years = years_from_f64(tenor_years).ok_or_else(|| BuildError::BadDates(issue, issue))?;
        let maturity = add_years(issue, years).ok_or_else(|| BuildError::BadDates(issue, issue))?;

        Self::bond_full(
            InstrumentId(Uuid::new_v4()),
            issuer,
            BondType::Corporate,
            Money::from(1_000i64),
            issue,
            maturity,
            BondTerms::Fixed {
                coupon_rate_bps: BasisPoints::abs(&dec!(500.0)),
                frequency: NonZeroU32::new(2).expect("frequency > 0"),
                day_count: DayCount::ActAct,
            },
            CreditRating::Corporate(SpCreditRating::BBB),
            MarketChoice::Auto,
        )
    }
}

fn years_from_f64(t: f64) -> Option<NonZeroU32> {
    if !t.is_finite() || t < 1.0 {
        return None;
    }
    let r = t.round();
    if (t - r).abs() > 1e-9 {
        return None;
    }
    NonZeroU32::new(r as u32)
}

fn add_years(start: NaiveDate, years: NonZeroU32) -> Option<NaiveDate> {
    start.checked_add_months(Months::new(years.get() * 12))
}
