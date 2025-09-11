use crate::*;
use chrono::NaiveDate;
use rand::prelude::*;
use rust_decimal::prelude::*;

pub fn quote_treasury_yields(
    tenor_years: f64,
    policy_bps: BasisPoints,
    rng: &mut dyn RngCore,
) -> (BasisPoints, BasisPoints) {
    let tp_bps = if tenor_years <= 0.083 { 2.0 }
    else if tenor_years <= 0.25 { 7.0 }
    else if tenor_years <= 1.0 { 12.0 }
    else if tenor_years <= 5.0 { 35.0 }
    else if tenor_years <= 10.0 { 50.0 }
    else { 65.0 };

    let jitter: f64 = rng.random_range(0.95..1.05);
    let spread_bps: f64 = rng.random_range(10.0..25.0);

    let mid_bps = policy_bps + Rate::from_f64(tp_bps * jitter).unwrap_or(Rate::ZERO);
    let half = Rate::from_f64(spread_bps * 0.5).unwrap_or(Rate::ZERO);

    let bid = mid_bps + half;
    let ask = mid_bps - half;
    (bid, ask)
}

pub fn auction_bid_price(
    bond: &BondDetails,
    y_bps: BasisPoints,
    instrument_id: &InstrumentId,
    feeds: &PricingFeeds,
    as_of: NaiveDate,
) -> Money {
    let local_feeds = feeds.with_date(as_of);

    let pricer = GovTermStructurePricer::new(
        bond.clone(),
        TermStructureMethod::default(),
        local_feeds,
    );

    let y_decimal = bps_to_decimal(y_bps).to_f64().unwrap_or(0.0);

    pricer.price_from_yield(instrument_id, y_decimal).unwrap_or(Money::ZERO)
}
