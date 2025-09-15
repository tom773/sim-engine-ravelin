use chrono::{NaiveDate, Months, Datelike, Duration};
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use rust_decimal_macros::dec;
use crate::*;

pub fn is_weekend(date: NaiveDate) -> bool {
    matches!(date.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun)
}

pub fn next_business_day(date: NaiveDate) -> NaiveDate {
    let mut next = date + Duration::days(1);
    while is_weekend(next) {
        next = next + Duration::days(1);
    }
    next
}

pub fn previous_business_day(date: NaiveDate) -> NaiveDate {
    let mut prev = date - Duration::days(1);
    while is_weekend(prev) {
        prev = prev - Duration::days(1);
    }
    prev
}

pub fn add_business_days(date: NaiveDate, days: i32) -> NaiveDate {
    let mut current = date;
    let mut remaining = days.abs();
    let step = if days >= 0 { 1 } else { -1 };
    
    while remaining > 0 {
        current = current + Duration::days(step as i64);
        if !is_weekend(current) {
            remaining -= 1;
        }
    }
    current
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TimePeriod {
    Days(u32),
    Weeks(u32),
    Months(u32),
    Years(u32),
    Overnight,
    Weekly,
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
}

impl TimePeriod {
    pub fn to_days(&self) -> u32 {
        match self {
            TimePeriod::Days(d) => *d,
            TimePeriod::Weeks(w) => w * 7,
            TimePeriod::Months(m) => m * 30,
            TimePeriod::Years(y) => y * 365,
            TimePeriod::Overnight => 1,
            TimePeriod::Weekly => 7,
            TimePeriod::Monthly => 30,
            TimePeriod::Quarterly => 90,
            TimePeriod::SemiAnnual => 183,
            TimePeriod::Annual => 365,
        }
    }
    
    pub fn add_to_date(&self, date: NaiveDate) -> NaiveDate {
        match self {
            TimePeriod::Days(d) => date + Duration::days(*d as i64),
            TimePeriod::Weeks(w) => date + Duration::weeks(*w as i64),
            TimePeriod::Months(m) => {
                let mut result = date;
                for _ in 0..*m {
                    result = add_months(result, 1);
                }
                result
            },
            TimePeriod::Years(y) => {
                date.with_year(date.year() + *y as i32)
                    .unwrap_or(date)
            },
            TimePeriod::Overnight => date + Duration::days(1),
            TimePeriod::Weekly => date + Duration::weeks(1),
            TimePeriod::Monthly => add_months(date, 1),
            TimePeriod::Quarterly => add_months(date, 3),
            TimePeriod::SemiAnnual => add_months(date, 6),
            TimePeriod::Annual => date.with_year(date.year() + 1).unwrap_or(date),
        }
    }
}

pub fn add_months(date: NaiveDate, months: u32) -> NaiveDate {
    let mut year = date.year();
    let mut month = date.month() as i32 + months as i32;
    
    while month > 12 {
        year += 1;
        month -= 12;
    }
    
    let day = date.day();
    
    let max_day = days_in_month(year, month as u32);
    let adjusted_day = day.min(max_day);
    
    NaiveDate::from_ymd_opt(year, month as u32, adjusted_day)
        .unwrap_or(date)
}
pub fn is_last_day_of_month(date: NaiveDate) -> bool {
    date.day() == 4
}
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) { 29 } else { 28 }
        },
        _ => 30,
    }
}
pub fn is_coupon_date(date: NaiveDate, bond: &BondDetails) -> bool {
    if bond.frequency == 0 {
        return false;
    }
    let months_since_issue =
        ((date.year() - bond.issue_date.year()) * 12 + date.month() as i32 - bond.issue_date.month() as i32) as u32;
    date.day() == bond.issue_date.day() && months_since_issue > 0 && months_since_issue % (12 / bond.frequency) == 0
}
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BusinessDayConvention {
    None,
    Following,
    Preceding,
    ModifiedFollowing,
    ModifiedPreceding,
}

impl BusinessDayConvention {
    pub fn adjust(&self, date: NaiveDate) -> NaiveDate {
        match self {
            BusinessDayConvention::None => date,
            BusinessDayConvention::Following => {
                if is_weekend(date) {
                    next_business_day(date)
                } else {
                    date
                }
            },
            BusinessDayConvention::Preceding => {
                if is_weekend(date) {
                    previous_business_day(date)
                } else {
                    date
                }
            },
            BusinessDayConvention::ModifiedFollowing => {
                if is_weekend(date) {
                    let next = next_business_day(date);
                    if next.month() != date.month() {
                        previous_business_day(date)
                    } else {
                        next
                    }
                } else {
                    date
                }
            },
            BusinessDayConvention::ModifiedPreceding => {
                if is_weekend(date) {
                    let prev = previous_business_day(date);
                    if prev.month() != date.month() {
                        next_business_day(date)
                    } else {
                        prev
                    }
                } else {
                    date
                }
            },
        }
    }
}

pub type BasisPoints = Rate; // Formerly f64

pub const BPS_PER_UNIT: Rate = dec!(10_000);

#[inline]
pub fn bps_to_decimal(bps: BasisPoints) -> Rate {
    bps / BPS_PER_UNIT
}

#[inline]
pub fn decimal_to_bps(decimal: Rate) -> BasisPoints {
    decimal * BPS_PER_UNIT
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Default)]
pub enum DayCount {
    Act360,
    #[default]
    Act365F,
    ActAct,
}

impl DayCount {
    pub fn year_fraction(&self, start: NaiveDate, end: NaiveDate) -> f64 {
        if start > end {
             return -self.year_fraction(end, start);
        }
        let days = (end - start).num_days() as f64;
        match self {
            DayCount::Act360 => days / 360.0,
            DayCount::Act365F => days / 365.0,
            DayCount::ActAct => days / 365.25,
        }
    }

    pub fn calculate_accrued_interest(
        &self,
        principal: Money,
        rate_bps: BasisPoints,
        start_date: NaiveDate,
        end_date: NaiveDate
    ) -> Money {
        let year_frac = self.year_fraction(start_date, end_date);
        let annual_rate = bps_to_decimal(rate_bps);
        principal * annual_rate * Rate::from_f64(year_frac).unwrap_or(Rate::ZERO)}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tenor {
    T1M,
    T2M,
    T3M,
    T6M,
    T1Y,
    T2Y,
    T5Y,
    T10Y,
    T30Y,
}

impl Tenor {
    pub fn to_years(&self) -> f64 {
        match self {
            Tenor::T1M => 1.0/12.0,
            Tenor::T2M => 2.0/12.0,
            Tenor::T3M => 0.25,
            Tenor::T6M => 0.5,
            Tenor::T1Y => 1.0,
            Tenor::T2Y => 2.0,
            Tenor::T5Y => 5.0,
            Tenor::T10Y => 10.0,
            Tenor::T30Y => 30.0,
        }
    }
    
    pub fn to_months(&self) -> u32 {
        match self {
            Tenor::T1M => 1,
            Tenor::T2M => 2,
            Tenor::T3M => 3,
            Tenor::T6M => 6,
            Tenor::T1Y => 12,
            Tenor::T2Y => 24,
            Tenor::T5Y => 60,
            Tenor::T10Y => 120,
            Tenor::T30Y => 360,
        }
    }
    
    pub fn add_to_date(&self, date: NaiveDate) -> NaiveDate {
        date.checked_add_months(Months::new(self.to_months()))
            .unwrap_or(date)
    }
    
    pub fn from_years(years: f64) -> Self {
        if years <= 0.1 { Tenor::T1M }
        else if years <= 0.15 { Tenor::T2M }
        else if years <= 0.35 { Tenor::T3M }
        else if years <= 0.75 { Tenor::T6M }
        else if years <= 1.5 { Tenor::T1Y }
        else if years <= 3.5 { Tenor::T2Y }
        else if years <= 7.5 { Tenor::T5Y }
        else if years <= 20.0 { Tenor::T10Y }
        else { Tenor::T30Y }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Session { AM, PM, EOD }

impl Session {
    pub const ALL: [Session; 3] = [Session::AM, Session::PM, Session::EOD];
    #[inline] pub fn idx(self) -> u8 { match self { Session::AM => 0, Session::PM => 1, Session::EOD => 2 } }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Slot(pub u64);

impl Slot {
    #[inline] pub fn of(tick: u32, session: Session) -> Self { Slot(tick as u64 * 3 + session.idx() as u64) }
}