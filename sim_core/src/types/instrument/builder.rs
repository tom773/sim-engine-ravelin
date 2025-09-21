use crate::prelude::*;
use crate::types::instrument::archetypes::{BondArchetype, BondType, CashFlow, CashType, CreditRating};
use crate::types::instrument::inst_core::{
    BondState, CapitalMarketSegment, CashState, InstrumentIdentifiers, InstrumentMarket, Listability, MarketProfile,
    MoneyMarketSegment, VenueType,
};
use crate::types::instrument::inst_registry::{LotId, SeriesId, TemplateId};
use crate::types::instrument::{Instrument, InstrumentRuntime};
use chrono::{Months, NaiveDate};
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
