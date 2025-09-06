use crate::*;
use rust_decimal::prelude::*;
use rust_decimal::MathematicalOps;
fn bond_price_f64(face_value: f64, coupon_rate: f64, ytm: f64, years_to_maturity: f64, frequency: usize) -> f64 {
    if years_to_maturity <= 0.0 || frequency == 0 {
        return face_value;
    }
    
    let coupon_payment = face_value * coupon_rate / frequency as f64;
    let periodic_rate = ytm / frequency as f64;
    let total_periods = years_to_maturity * frequency as f64;
    
    let mut price = 0.0;
    
    for period in 1..=(total_periods as i32) {
        price += coupon_payment / (1.0 + periodic_rate).powi(period);
    }
    
    price += face_value / (1.0 + periodic_rate).powf(total_periods);
    
    price
}


pub fn ytm_bond(price: Money, face_value: Money, coupon_rate: Rate, years_to_maturity: f64, frequency: usize) -> Rate {
    if years_to_maturity <= 0.0 || frequency == 0 {
        return Rate::ZERO;
    }
    
    // Convert to f64 for iterative solver
    let price = price.to_f64();
    let face_value = face_value.to_f64();
    let coupon_rate = coupon_rate.to_f64().unwrap_or(0.0);

    let coupon_payment = face_value * coupon_rate / frequency as f64;
    let total_periods = years_to_maturity * frequency as f64;
    
    let mut ytm_guess = coupon_rate;
    
    for _ in 0..100 {
        let periodic_rate = ytm_guess / frequency as f64;
        
        let mut calculated_price = 0.0;
        
        for period in 1..=(total_periods as i32) {
            calculated_price += coupon_payment / (1.0 + periodic_rate).powi(period);
        }
        
        calculated_price += face_value / (1.0 + periodic_rate).powf(total_periods);
        
        let price_diff = calculated_price - price;
        
        if price_diff.abs() < 0.01 {
            return Decimal::from_f64(ytm_guess).unwrap_or_default();
        }
        
        let mut derivative = 0.0;
        for period in 1..=(total_periods as i32) {
            derivative -= period as f64 * coupon_payment / frequency as f64 / (1.0 + periodic_rate).powi(period + 1);
        }
        derivative -= total_periods * face_value / frequency as f64 / (1.0 + periodic_rate).powf(total_periods + 1.0);
        
        if derivative.abs() < 1e-10 {
            break;
        }
        
        ytm_guess = ytm_guess - price_diff / derivative;
        
        ytm_guess = ytm_guess.max(-0.5).min(2.0);
    }
    
    Decimal::from_f64(ytm_guess).unwrap_or_default()
}

pub fn bond_price(face_value: Money, coupon_rate: Rate, ytm: Rate, years_to_maturity: f64, frequency: usize) -> Money {
    if years_to_maturity <= 0.0 || frequency == 0 {
        return face_value;
    }
    
    let freq_dec = Decimal::from(frequency);
    let coupon_payment = face_value * coupon_rate / freq_dec;
    let periodic_rate = ytm / freq_dec;
    let total_periods = years_to_maturity * frequency as f64;
    
    let mut price = Money::ZERO;
    
    for period in 1..=(total_periods as i32) {
        let denominator = (Decimal::ONE + periodic_rate).powi(period as i64);
        if !denominator.is_zero() {
            price += coupon_payment / denominator;
        }
    }
    
    let final_denominator = (Decimal::ONE + periodic_rate).powf(total_periods);
    if !final_denominator.is_zero() {
        price += face_value / final_denominator;
    }
    
    price
}

pub fn duration(face_value: Money, coupon_rate: Rate, ytm: Rate, years_to_maturity: f64, frequency: usize) -> f64 {
    if years_to_maturity <= 0.0 || frequency == 0 {
        return 0.0;
    }

    // Convert to f64 for calculation
    let face_value = face_value.to_f64();
    let coupon_rate = coupon_rate.to_f64().unwrap_or(0.0);
    let ytm = ytm.to_f64().unwrap_or(0.0);

    let coupon_payment = face_value * coupon_rate / frequency as f64;
    let periodic_rate = ytm / frequency as f64;
    let total_periods = years_to_maturity * frequency as f64;
    let bond_price = bond_price_f64(face_value, coupon_rate, ytm, years_to_maturity, frequency);
    
    if bond_price <= 0.0 {
        return 0.0;
    }
    
    let mut weighted_cash_flows = 0.0;
    
    for period in 1..=(total_periods as i32) {
        let pv_coupon = coupon_payment / (1.0 + periodic_rate).powi(period);
        let time_weight = period as f64 / frequency as f64;
        weighted_cash_flows += pv_coupon * time_weight;
    }
    
    let pv_face_value = face_value / (1.0 + periodic_rate).powf(total_periods);
    weighted_cash_flows += pv_face_value * years_to_maturity;
    
    weighted_cash_flows / bond_price
}

pub fn modified_duration(face_value: Money, coupon_rate: Rate, ytm: Rate, years_to_maturity: f64, frequency: usize) -> f64 {
    let dur = duration(face_value, coupon_rate, ytm, years_to_maturity, frequency);
    let ytm_f64 = ytm.to_f64().unwrap_or_default();
    dur / (1.0 + ytm_f64 / frequency as f64)
}

pub fn convexity(face_value: Money, coupon_rate: Rate, ytm: Rate, years_to_maturity: f64, frequency: usize) -> f64 {
    if years_to_maturity <= 0.0 || frequency == 0 {
        return 0.0;
    }
    
    // Convert to f64 for calculation
    let face_value = face_value.to_f64();
    let coupon_rate = coupon_rate.to_f64().unwrap_or(0.0);
    let ytm = ytm.to_f64().unwrap_or(0.0);

    let coupon_payment = face_value * coupon_rate / frequency as f64;
    let periodic_rate = ytm / frequency as f64;
    let total_periods = years_to_maturity * frequency as f64;
    let bond_price = bond_price_f64(face_value, coupon_rate, ytm, years_to_maturity, frequency);
    
    if bond_price <= 0.0 {
        return 0.0;
    }
    
    let mut convexity_sum = 0.0;
    
    for period in 1..=(total_periods as i32) {
        let pv_coupon = coupon_payment / (1.0 + periodic_rate).powi(period);
        let time_factor = period as f64 * (period as f64 + 1.0) / (frequency as f64).powi(2);
        convexity_sum += pv_coupon * time_factor;
    }
    
    let pv_face_value = face_value / (1.0 + periodic_rate).powf(total_periods);
    let time_factor = total_periods * (total_periods + 1.0) / (frequency as f64).powi(2);
    convexity_sum += pv_face_value * time_factor;
    
    convexity_sum / bond_price / (1.0 + periodic_rate).powi(2)
}

pub fn years_to_maturity(current_date: chrono::NaiveDate, maturity_date: chrono::NaiveDate) -> f64 {
    let days_diff = (maturity_date - current_date).num_days();
    if days_diff <= 0 {
        0.0
    } else {
        days_diff as f64 / 365.25
    }
}

// === New Option Pricing Functions ===

// Helper function: Standard normal cumulative distribution function (CDF) approximation
// Uses the error function (erf), available in the standard library.
pub fn norm_cdf(x: f64) -> f64 {
    0.5 * (1.0 + (x / std::f64::consts::SQRT_2).erf())
}

/// Basic Black-Scholes model for European options
pub fn black_scholes(
    s: Money, // Spot price of the underlying asset
    k: Money, // Strike price
    t: f64, // Time to maturity (in years)
    r: Rate, // Risk-free interest rate (annualized)
    sigma: Rate, // Volatility of the underlying asset (annualized)
    is_call: bool,
) -> Money {
    if t <= 0.0 {
        // Option expired, return intrinsic value
        return if is_call {
            if s > k { s - k } else { Money::ZERO }
        } else {
            if k > s { k - s } else { Money::ZERO }
        };
    }

    let s = s.to_f64();
    let k = k.to_f64();
    let r = r.to_f64().unwrap_or(0.0);
    let sigma = sigma.to_f64().unwrap_or(0.0);

    if sigma <= 0.0 {
        // Handle zero volatility case
        let discount_factor = (-r * t).exp();
        let price_f64 = if is_call {
            (s - k * discount_factor).max(0.0)
        } else {
            (k * discount_factor - s).max(0.0)
        };
        return Money::from_f64(price_f64).unwrap_or_default();
    }

    let d1 = (s / k).ln() + (r + 0.5 * sigma.powi(2)) * t;
    let d1 = d1 / (sigma * t.sqrt());
    let d2 = d1 - sigma * t.sqrt();

    let discount_factor = (-r * t).exp();

    let price_f64 = if is_call {
        s * norm_cdf(d1) - k * discount_factor * norm_cdf(d2)
    } else {
        // Put formula (or Put-Call Parity)
        k * discount_factor * norm_cdf(-d2) - s * norm_cdf(-d1)
    };

    Money::from_f64(price_f64).unwrap_or_default()
}