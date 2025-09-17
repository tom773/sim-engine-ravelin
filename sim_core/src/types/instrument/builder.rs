use crate::prelude::*;
use chrono::Months;
use chrono::NaiveDate;
use rust_decimal_macros::dec;
use std::num::NonZeroU32;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("maturity must be after issue (issue={0}, maturity={1})")]
    BadDates(NaiveDate, NaiveDate),
    #[error("frequency must be > 0 for coupon_rate_bpsed bonds")]
    BadFrequency,
    #[error("zero-coupon_rate_bps bonds must have coupon == 0 bps")]
    ZeroCouponHasCoupon,
}

fn classify_market(bond_type: BondType, issue: NaiveDate, maturity: NaiveDate) -> InstrumentMarket {
    let days = (maturity - issue).num_days();
    let money = days <= 365;
    match (bond_type, money) {
        (BondType::Government, true) | (BondType::InterbankLoan, true) => {
            InstrumentMarket::MoneyMarket(MoneyMarketSegment::SovereignShortTerm)
        }
        (BondType::Corporate, true) => InstrumentMarket::MoneyMarket(MoneyMarketSegment::CorporateShortTerm),
        (BondType::Government, false) | (BondType::InterbankLoan, false) => {
            InstrumentMarket::CapitalMarket(CapitalMarketSegment::SovereignLongTerm)
        }
        (BondType::Corporate, false) => InstrumentMarket::CapitalMarket(CapitalMarketSegment::CorporateCredit),
    }
}

pub struct CashBuilder {
    id: InstrumentId,
    market: Option<InstrumentMarket>,
    details: CashDetails,
    listability: Listability,
}

impl Instrument {
    pub fn cash(
        id: InstrumentId, issuer: AgentId, cash_type: CashType, currency: Currency, rate: BasisPoints,
    ) -> CashBuilder {
        CashBuilder {
            id,
            market: None,
            details: CashDetails { issuer, cash_type, currency, interest_bps: rate },
            listability: Listability::Unlisted,
        }
    }
}

impl CashBuilder {
    pub fn market(mut self, m: InstrumentMarket) -> Self {
        self.market = Some(m);
        self
    }
    pub fn build(self) -> Instrument {
        Instrument {
            id: self.id,
            instrument_type: InstrumentType::Cash(self.details),
            instrument_market: self.market.unwrap_or(InstrumentMarket::MoneyMarket(MoneyMarketSegment::Interbank)),
            listability: self.listability,
        }
    }
}

pub struct BondBuilder {
    id: InstrumentId,
    market: Option<InstrumentMarket>,
    details: BondDetails,
    listability: Listability,
}

impl Instrument {
    pub fn bond(
        id: InstrumentId, issuer: AgentId, bond_type: BondType, face_value: Money, issue_date: NaiveDate,
        maturity_date: NaiveDate,
    ) -> BondBuilder {
        BondBuilder {
            id,
            market: None,
            details: BondDetails {
                bond_type,
                issuer,
                cash_flow: CashFlow::Fixed,
                coupon_rate_bps: BasisPoints::abs(&dec!(0.0)),
                face_value,
                issue_date,
                maturity_date,
                frequency: 2,
                day_count: DayCount::ActAct,
                rating: CreditRating::Corporate(SpCreditRating::BBB),
                last_accrual_date: Some(maturity_date - chrono::Duration::days(1)),
            },
            listability: Listability::Unlisted,
        }
    }
}

impl BondBuilder {
    pub fn zero_coupon_rate_bps(mut self) -> Self {
        self.details.cash_flow = CashFlow::Zero;
        self.details.coupon_rate_bps = BasisPoints::abs(&dec!(0.0));
        self.details.frequency = 0;
        self
    }
    pub fn fixed(mut self) -> Self {
        self.details.cash_flow = CashFlow::Fixed;
        self
    }
    pub fn floating(mut self) -> Self {
        self.details.cash_flow = CashFlow::Floating;
        self
    }

    pub fn coupon_bps(mut self, bps: BasisPoints) -> Self {
        self.details.coupon_rate_bps = bps;
        self
    }
    pub fn frequency(mut self, per_year: u32) -> Self {
        self.details.frequency = per_year;
        self
    }
    pub fn day_count(mut self, dc: DayCount) -> Self {
        self.details.day_count = dc;
        self
    }
    pub fn rating(mut self, r: CreditRating) -> Self {
        self.details.rating = r;
        self
    }
    pub fn market(mut self, m: InstrumentMarket) -> Self {
        self.market = Some(m);
        self
    }

    pub fn auto_market(mut self) -> Self {
        let m = classify_market(self.details.bond_type, self.details.issue_date, self.details.maturity_date);
        self.market = Some(m);
        self.listability = Listability::Listed(VenueType::CentralLimitOrderBook);
        self
    }

    pub fn build(self) -> Result<Instrument, BuildError> {
        let d = &self.details.clone();

        if d.maturity_date <= d.issue_date {
            return Err(BuildError::BadDates(d.issue_date, d.maturity_date));
        }
        if matches!(d.cash_flow, CashFlow::Zero) && d.coupon_rate_bps != dec!(0.0) {
            return Err(BuildError::ZeroCouponHasCoupon);
        }
        if matches!(d.cash_flow, CashFlow::Fixed | CashFlow::Floating) && d.frequency == 0 {
            return Err(BuildError::BadFrequency);
        }

        Ok(Instrument {
            id: self.id,
            instrument_type: InstrumentType::Debt(DebtInstrument::Bond(self.details)),
            instrument_market: self
                .market
                .unwrap_or_else(|| classify_market(d.bond_type, d.issue_date, d.maturity_date)),
            listability: self.listability,
        })
    }
}

pub fn today() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

#[derive(Debug, Clone, Copy)]
pub enum MarketChoice {
    Auto,
    Set(InstrumentMarket),
}

#[derive(Debug, Clone, Copy)]
pub enum BondTerms {
    Zero,
    Fixed { coupon_rate_bps_bps: BasisPoints, frequency: NonZeroU32, day_count: DayCount },
    Floating { spread_bps: BasisPoints, reset_freq: NonZeroU32, day_count: DayCount },
}

impl Instrument {
    #[allow(clippy::too_many_arguments)]
    pub fn bond_full(
        id: InstrumentId, issuer: AgentId, bond_type: BondType, face_value: Money, issue_date: NaiveDate,
        maturity_date: NaiveDate, terms: BondTerms, rating: CreditRating, market: MarketChoice,
    ) -> Result<Self, BuildError> {
        if maturity_date <= issue_date {
            return Err(BuildError::BadDates(issue_date, maturity_date));
        }

        let (cash_flow, coupon_rate_bps, frequency, day_count) = match terms {
            BondTerms::Zero => (CashFlow::Zero, BasisPoints::abs(&dec!(0.0)), 0u32, DayCount::ActAct),
            BondTerms::Fixed { coupon_rate_bps_bps, frequency, day_count } => {
                (CashFlow::Fixed, coupon_rate_bps_bps, frequency.get(), day_count)
            }
            BondTerms::Floating { spread_bps, reset_freq, day_count } => {
                (CashFlow::Floating, spread_bps, reset_freq.get(), day_count)
            }
        };

        if matches!(cash_flow, CashFlow::Fixed | CashFlow::Floating) && frequency == 0 {
            return Err(BuildError::BadFrequency);
        }

        let details = BondDetails {
            bond_type,
            issuer,
            cash_flow,
            coupon_rate_bps,
            face_value,
            issue_date,
            maturity_date,
            frequency,
            day_count,
            rating,
            last_accrual_date: Some(maturity_date - chrono::Duration::days(1)),
        };

        let instrument_market = match market {
            MarketChoice::Set(m) => m,
            MarketChoice::Auto => classify_market(bond_type, issue_date, maturity_date),
        };

        Ok(Instrument {
            id,
            instrument_type: InstrumentType::Debt(DebtInstrument::Bond(details)),
            instrument_market,
            listability: Listability::Listed(VenueType::CentralLimitOrderBook),
        })
    }

    pub fn bond_fixed(
        id: InstrumentId, issuer: AgentId, bond_type: BondType, face_value: Money, issue_date: NaiveDate,
        maturity_date: NaiveDate, coupon_rate_bps_bps: BasisPoints, frequency: NonZeroU32, day_count: DayCount,
        rating: CreditRating, market: MarketChoice,
    ) -> Result<Self, BuildError> {
        Self::bond_full(
            id,
            issuer,
            bond_type,
            face_value,
            issue_date,
            maturity_date,
            BondTerms::Fixed { coupon_rate_bps_bps, frequency, day_count },
            rating,
            market,
        )
    }

    pub fn gov_bond(tenor_years: f64, coupon_rate_bps_bps: BasisPoints) -> Result<Self, BuildError> {
        let issue = today();
        let years = years_from_f64(tenor_years).ok_or_else(|| BuildError::BadDates(issue, issue))?;

        let maturity = add_years(issue, years).ok_or_else(|| BuildError::BadDates(issue, issue))?;

        Self::bond_full(
            InstrumentId(Uuid::new_v4()),
            AgentId(Uuid::new_v4()),
            BondType::Government,
            Money::from(1_000 as i64),
            issue,
            maturity,
            BondTerms::Fixed {
                coupon_rate_bps_bps,
                frequency: NonZeroU32::new(2).unwrap(),
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
            Money::from(1_000 as i64),
            issue,
            maturity,
            BondTerms::Zero,
            CreditRating::Government(SpCreditRating::AAA),
            MarketChoice::Auto,
        )
    }

    pub fn corp_bond(issuer: AgentId, tenor_years: f64) -> Result<Self, BuildError> {
        let issue = today();
        let maturity = add_years(issue, years_from_f64(tenor_years).unwrap()).unwrap();
        Self::bond_full(
            InstrumentId(Uuid::new_v4()),
            issuer,
            BondType::Corporate,
            Money::from(1_000 as i64),
            issue,
            maturity,
            BondTerms::Fixed {
                coupon_rate_bps_bps: BasisPoints::abs(&dec!(500.0)),
                frequency: NonZeroU32::new(2).unwrap(),
                day_count: DayCount::ActAct,
            },
            CreditRating::Corporate(SpCreditRating::BBB),
            MarketChoice::Auto,
        )
    }
}

impl Instrument {
    pub fn loan(id: InstrumentId, loan_details: LoanDetails) -> Self {
        Instrument {
            id,
            instrument_type: InstrumentType::Debt(DebtInstrument::Loan(loan_details)),
            instrument_market: InstrumentMarket::Unlisted,
            listability: Listability::Unlisted,
        }
    }

    pub fn credit_line(id: InstrumentId, credit_line_details: CreditLineDetails) -> Self {
        Instrument {
            id,
            instrument_type: InstrumentType::Debt(DebtInstrument::CreditLine(credit_line_details)),
            instrument_market: InstrumentMarket::Unlisted,
            listability: Listability::Unlisted,
        }
    }
    pub fn consumer_mortgage(id: InstrumentId, details: LoanDetails) -> Self {
        Instrument {
            id,
            instrument_type: InstrumentType::Debt(DebtInstrument::Consumer(ConsumerDebt::ResidentialMortgage(details))),
            instrument_market: InstrumentMarket::Unlisted,
            listability: Listability::Unlisted,
        }
    }

    pub fn consumer_auto(id: InstrumentId, details: LoanDetails) -> Self {
        Instrument {
            id,
            instrument_type: InstrumentType::Debt(DebtInstrument::Consumer(ConsumerDebt::AutoLoan(details))),
            instrument_market: InstrumentMarket::Unlisted,
            listability: Listability::Unlisted,
        }
    }

    pub fn consumer_personal(id: InstrumentId, details: LoanDetails) -> Self {
        Instrument {
            id,
            instrument_type: InstrumentType::Debt(DebtInstrument::Consumer(ConsumerDebt::PersonalLoan(details))),
            instrument_market: InstrumentMarket::Unlisted,
            listability: Listability::Unlisted,
        }
    }

    pub fn consumer_credit_card(id: InstrumentId, details: CreditLineDetails) -> Self {
        Instrument {
            id,
            instrument_type: InstrumentType::Debt(DebtInstrument::Consumer(ConsumerDebt::CreditCard(details))),
            instrument_market: InstrumentMarket::Unlisted,
            listability: Listability::Unlisted,
        }
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
