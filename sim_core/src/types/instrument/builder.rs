use crate::prelude::*;
use crate::types::instrument::archetypes::{
    BondArchetype, BondType, CashFlow, CashType, CreditRating, InstrumentMarket, MoneyMarketSegment,
};
use crate::types::instrument::inst_core::{
    BondState, CapitalMarketSegment, CashState, InstrumentIdentifiers, Listability, MarketProfile, VenueType,
};
use crate::types::instrument::inst_registry::{LotId, SeriesId, TemplateId};
use crate::types::instrument::instrument::{DividendPolicy, EquityClass, EquityProfile, EquityState};
use crate::types::instrument::{Instrument, InstrumentRuntime};
use chrono::NaiveDate;
use rust_decimal_macros::dec;

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
    lender: AgentId,
    borrower: AgentId,
    collateral_id: InstrumentId,
    collateral_quantity: f64,
    cash_principal: Money,
    interest_bps: BasisPoints,
    start_date: NaiveDate,
    end_date: NaiveDate,
    haircut: Rate,
    open_term: bool,
}

impl Instrument {
    pub fn repo(
        id: InstrumentId, lender: AgentId, borrower: AgentId, collateral_id: InstrumentId, collateral_quantity: f64,
        cash_principal: Money, interest_bps: BasisPoints, start_date: NaiveDate, end_date: NaiveDate, haircut: Rate,
    ) -> RepoBuilder {
        RepoBuilder {
            identifiers: InstrumentIdentifiers::new(id),
            market_profile: None,
            listability: Listability::Unlisted,
            lender,
            borrower,
            collateral_id,
            collateral_quantity,
            cash_principal,
            interest_bps,
            start_date,
            end_date,
            haircut,
            open_term: false,
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

    pub fn open_term(mut self, open: bool) -> Self {
        self.open_term = open;
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

        let state = crate::types::instrument::instrument::RepoState {
            lender: self.lender,
            borrower: self.borrower,
            collateral_id: self.collateral_id,
            collateral_quantity: self.collateral_quantity,
            cash_principal: self.cash_principal,
            interest_bps: self.interest_bps,
            start_date: self.start_date,
            end_date: self.end_date,
            haircut: self.haircut,
            open_term: self.open_term,
        };

        Instrument::new(self.identifiers, market_profile, self.listability, InstrumentRuntime::Repo(state))
    }
}
