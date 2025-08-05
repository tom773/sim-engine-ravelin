use chrono::{NaiveDate, Datelike, Duration};
use serde::{Deserialize, Serialize};

pub fn year_fraction(start: NaiveDate, end: NaiveDate) -> f64 {
    (end - start).num_days() as f64 / 365.0
}

pub fn year_fraction_360(start: NaiveDate, end: NaiveDate) -> f64 {
    (end - start).num_days() as f64 / 360.0
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    
    #[test]
    fn test_year_fraction() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        
        let frac = year_fraction(start, end);
        assert!((frac - 1.0).abs() < 0.01);
    }
    
    #[test]
    fn test_business_days() {
        let friday = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap();
        let next = next_business_day(friday);
        let monday = NaiveDate::from_ymd_opt(2024, 1, 8).unwrap();
        
        assert_eq!(next, monday);
    }
    
    #[test]
    fn test_add_months() {
        let jan_31 = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let result = add_months(jan_31, 1);
        let feb_29 = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        
        assert_eq!(result, feb_29);
    }
}