
use chrono::{Datelike, NaiveDate};

pub struct FlatCurve {
    pub rate: f64,
}

impl FlatCurve {
    pub fn new(rate: f64) -> Self {
        Self { rate }
    }
    #[inline]
    fn df(&self, t: f64) -> f64 {
        (-self.rate * t).exp()
    }
}

pub trait DiscountCurve {
    fn discount(&self, year_frac: f64) -> f64;
}

impl DiscountCurve for FlatCurve {
    #[inline]
    fn discount(&self, t: f64) -> f64 {
        self.df(t)
    }
}

fn year_fraction(start: NaiveDate, end: NaiveDate) -> f64 {
    (end - start).num_days() as f64 / 365.0
}

pub trait Bond {
    fn price<C: DiscountCurve>(&self, settle: NaiveDate, curve: &C) -> f64;
}

pub struct CouponGovBond {
    pub face: f64,          // usually 100
    pub coupon_rate: f64,   // 0.04 → 4 %
    pub frequency: usize,   // 1 = annual, 2 = semi‑annual, 4 = quarterly
    pub maturity: NaiveDate,
}

impl CouponGovBond {
    fn cashflows(&self, settle: NaiveDate) -> Vec<(NaiveDate, f64)> {
        let mut flows = Vec::new();
        let months = 12 / self.frequency as i32;
        let mut pay_date = self.maturity;
        let coupon = self.face * self.coupon_rate / self.frequency as f64;

        while pay_date > settle {
            flows.push((pay_date, coupon));
            pay_date = pay_date
                .with_month0(pay_date.month0().saturating_sub(months as u32))
                .unwrap(); // crude – ignores EoM roll
        }
        if self.maturity > settle {
            flows.push((self.maturity, self.face));
        }
        flows
    }
}

impl Bond for CouponGovBond {
    fn price<C: DiscountCurve>(&self, settle: NaiveDate, curve: &C) -> f64 {
        let pv: f64 = self
            .cashflows(settle)
            .into_iter()
            .map(|(d, cf)| {
                let t = year_fraction(settle, d);
                cf * curve.discount(t)
            })
            .sum();
        pv / self.face * 100.0
    }
}

pub struct ZeroCouponGov {
    pub face: f64,
    pub maturity: NaiveDate,
}

impl Bond for ZeroCouponGov {
    fn price<C: DiscountCurve>(&self, settle: NaiveDate, curve: &C) -> f64 {
        let t = year_fraction(settle, self.maturity);
        curve.discount(t) * 100.0 // price as % of face
    }
}

#[cfg(test)]
mod bond_tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn zero_coupon_matches_theory() {
        let settle = NaiveDate::from_ymd_opt(2025, 7, 30).unwrap();
        let curve  = FlatCurve::new(0.05); // 5 %
        let zcb = ZeroCouponGov {
            face: 100.0,
            maturity: NaiveDate::from_ymd_opt(2026, 7, 30).unwrap(),
        };
        let price = zcb.price(settle, &curve);
        assert!((price - 95.122).abs() < 0.02);
    }
}
