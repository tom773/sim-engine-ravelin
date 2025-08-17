use serde::{Deserialize, Serialize};
use chrono::NaiveDate;

pub type BasisPoints = f64;

pub const BPS_PER_UNIT: f64 = 10000.0;

#[inline]
pub fn bps_to_decimal(bps: BasisPoints) -> f64 {
    bps / BPS_PER_UNIT
}

#[inline]
pub fn decimal_to_bps(decimal: f64) -> BasisPoints {
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
        principal: f64,
        rate_bps: BasisPoints,
        start_date: NaiveDate,
        end_date: NaiveDate
    ) -> f64 {
        let year_frac = self.year_fraction(start_date, end_date);
        let annual_rate = bps_to_decimal(rate_bps);
        principal * annual_rate * year_frac
    }
}