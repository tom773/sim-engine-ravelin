use crate::prelude::*;
use crate::types::markets::{FinancialProduct, GoodProduct, Pricer};
use crate::types::money::Money;
use crate::types::system::financial_system::{GoodMetric, PricingFeeds};
use chrono::NaiveDate;
use ordered_float::NotNan;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use std::fmt::Debug;
#[derive(Clone, Debug)]
pub enum TermStructureMethod {
    Bootstrapped,
    PolicyPlusTermPremium {
        base_bps: f64,
        slope_bps_per_year: f64,
    },
}
impl Default for TermStructureMethod {
    fn default() -> Self {
        TermStructureMethod::Bootstrapped
    }
}
#[derive(Clone, Debug)]
pub struct GovTermStructurePricer {
    spec: BondDetails,
    method: TermStructureMethod,
    feeds: PricingFeeds,
}

impl GovTermStructurePricer {
    pub fn new(spec: BondDetails, method: TermStructureMethod, feeds: PricingFeeds) -> Self {
        Self { spec, method, feeds }
    }
    fn coupon_rate(&self) -> Decimal {
        let coupon_rate = self.spec.coupon_rate_bps.to_f64().unwrap_or(0.0);
        Decimal::from_f64(coupon_rate / 10_000.0).unwrap_or(Decimal::ZERO)
    }

    fn price_from_yield_inner(&self, y_annual: f64, as_of: NaiveDate) -> Option<Money> {
        let freq = self.spec.frequency.max(1) as i32;
        let n = (self.spec.remaining_tenor_years(as_of) * self.spec.frequency as f64).ceil().max(0.0) as i32;

        const MAX_PERIODS: i32 = 4000;
        if n > MAX_PERIODS {
            return None;
        }

        let y = Decimal::from_f64(y_annual).unwrap_or(Decimal::ZERO) / Decimal::from(freq);
        if y <= dec!(-1.0) {
            return None;
        }

        let c = self.spec.face_value * (self.coupon_rate() / Decimal::from(self.spec.frequency));
        let mut pv = dec!(0);

        let v = dec!(1) / (dec!(1) + y);

        let mut v_n = Decimal::ONE;
        for _ in 0..n {
            v_n *= v;
            pv += c.0 * v_n;
        }

        pv += self.spec.face_value.0 * v_n;
        Some(Money(pv))
    }

    fn ytm_from_price_inner(&self, price: Money, as_of: NaiveDate) -> Option<f64> {
        let mut lo = 0.0f64;
        let mut hi = 0.50f64;
        for _ in 0..64 {
            let mid = 0.5 * (lo + hi);
            let pm = self.price_from_yield_inner(mid, as_of)?.0.to_f64().unwrap_or(0.0);
            let p0 = price.to_f64();
            if pm > p0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        Some(0.5 * (lo + hi))
    }
    fn policy_decimal(&self) -> Option<f64> {
        let bps = *self.feeds.policy_rate_bps.read().ok()?;
        Some((bps / 10_000.0).max(0.0))
    }

    fn curve_yield(&self, as_of: NaiveDate) -> Option<f64> {
        let tenor = self.spec.remaining_tenor_years(as_of).max(0.0);
        match &self.method {
            TermStructureMethod::Bootstrapped => {
                if let Some(y) = self.feeds.yield_curve.read().ok().and_then(|yc| interp_linear(&yc.points, tenor)) {
                    return Some(y);
                }
                let policy = self.policy_decimal()?;
                let slope_bps_per_year = 12.5;
                let premium = (slope_bps_per_year * tenor) / 10_000.0;
                Some((policy + premium).max(0.0))
            }
            TermStructureMethod::PolicyPlusTermPremium { base_bps, slope_bps_per_year } => {
                let policy = self.policy_decimal()?;
                let premium = (*base_bps + slope_bps_per_year * tenor) / 10_000.0;
                Some((policy + premium).max(0.0))
            }
        }
    }
}

fn interp_linear(points: &BTreeMap<NotNan<f64>, f64>, x: f64) -> Option<f64> {
    let x_key = NotNan::new(x).ok()?;

    if points.is_empty() {
        return None;
    }

    if let Some((&xx, &yy)) = points.range(..=x_key).next_back() {
        if let Some((&xu, &yu)) = points.range(x_key..).next() {
            if (xu.into_inner() - xx.into_inner()).abs() < 1e-12 {
                return Some(yy);
            }
            let w = (x - xx.into_inner()) / (xu.into_inner() - xx.into_inner());
            return Some((1.0 - w) * yy + w * yu);
        } else {
            return Some(yy);
        }
    } else {
        return points.iter().next().map(|(_, &y)| y);
    }
}

impl Pricer<FinancialProduct> for GovTermStructurePricer {
    fn mid_yield(&self, _key: &InstrumentId) -> Option<f64> {
        let as_of = *self.feeds.current_date.read().ok()?;
        self.curve_yield(as_of)
    }
    fn price_from_yield(&self, _key: &InstrumentId, y: f64) -> Option<Money> {
        let as_of = *self.feeds.current_date.read().ok()?;
        self.price_from_yield_inner(y, as_of)
    }
    fn yield_from_price(&self, _key: &InstrumentId, px: Money) -> Option<f64> {
        let as_of = *self.feeds.current_date.read().ok()?;
        self.ytm_from_price_inner(px, as_of)
    }
}

#[derive(Clone, Debug)]
pub struct CostPlusParams {
    pub wage_pass_through: f64,
    pub inv_target_days: f64,
    pub inv_elasticity: f64,
}

impl Default for CostPlusParams {
    fn default() -> Self {
        Self { wage_pass_through: 0.5, inv_target_days: 14.0, inv_elasticity: 0.75 }
    }
}

#[derive(Clone, Debug)]
pub struct CostPlusGoodsPricer {
    feeds: PricingFeeds,
    params: CostPlusParams,
}

impl CostPlusGoodsPricer {
    pub fn new(feeds: PricingFeeds, params: CostPlusParams) -> Self {
        Self { feeds, params }
    }
    fn fair_price_inner(&self, good_id: &GoodId) -> Option<Money> {
        let gm = self.feeds.goods.read().ok()?;
        let m: &GoodMetric = gm.per_good.get(good_id)?;
        let wg = if gm.last_avg_wage > 0.0 { (gm.avg_wage / gm.last_avg_wage) - 1.0 } else { 0.0 };
        let wage_factor = 1.0 + self.params.wage_pass_through * wg;
        let days_cov = if m.avg_daily_sales > 1e-9 { m.inventory_qty / m.avg_daily_sales } else { f64::INFINITY };
        let scarcity_adj = if days_cov.is_finite() {
            (self.params.inv_target_days / days_cov).powf(self.params.inv_elasticity).max(0.0)
        } else {
            1.0
        };
        let base_markup = m.base_markup.max(0.0);
        let supply = if m.supply_shock > 0.0 { m.supply_shock } else { 1.0 };
        let unit_cost = m.weighted_unit_cost.max(0.0);
        let px = unit_cost * wage_factor * (1.0 + base_markup) * scarcity_adj * supply;

        tracing::debug!(target: "sim.pricer",
        ?good_id, unit_cost, wage_factor, base_markup, scarcity_adj, supply, fair=px,
        "goods_fair_price");

        Money::from_f64(px)
    }
}

impl Pricer<GoodProduct> for CostPlusGoodsPricer {
    fn mid_yield(&self, _key: &GoodId) -> Option<f64> {
        None
    }
    fn price_from_yield(&self, _key: &GoodId, _y: f64) -> Option<Money> {
        None
    }
    fn yield_from_price(&self, _key: &GoodId, _px: Money) -> Option<f64> {
        None
    }
    fn fair_price(&self, key: &GoodId) -> Option<Money> {
        self.fair_price_inner(key)
    }
}
