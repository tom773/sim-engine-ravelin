pub fn ytm_bond(price: f64, face_value: f64, coupon_rate: f64, years_to_maturity: f64, frequency: usize) -> f64 {
    if years_to_maturity <= 0.0 || frequency == 0 {
        return 0.0;
    }
    
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
            return ytm_guess;
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
    
    ytm_guess
}

pub fn ytm_approximation(price: f64, face_value: f64, coupon_rate: f64, years_to_maturity: f64) -> f64 {
    if years_to_maturity <= 0.0 {
        return 0.0;
    }
    
    let annual_coupon = face_value * coupon_rate;
    let numerator = annual_coupon + (face_value - price) / years_to_maturity;
    let denominator = (face_value + price) / 2.0;
    
    numerator / denominator
}

pub fn bond_price(face_value: f64, coupon_rate: f64, ytm: f64, years_to_maturity: f64, frequency: usize) -> f64 {
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

pub fn duration(face_value: f64, coupon_rate: f64, ytm: f64, years_to_maturity: f64, frequency: usize) -> f64 {
    if years_to_maturity <= 0.0 || frequency == 0 {
        return 0.0;
    }
    
    let coupon_payment = face_value * coupon_rate / frequency as f64;
    let periodic_rate = ytm / frequency as f64;
    let total_periods = years_to_maturity * frequency as f64;
    let bond_price = bond_price(face_value, coupon_rate, ytm, years_to_maturity, frequency);
    
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

pub fn modified_duration(face_value: f64, coupon_rate: f64, ytm: f64, years_to_maturity: f64, frequency: usize) -> f64 {
    let dur = duration(face_value, coupon_rate, ytm, years_to_maturity, frequency);
    dur / (1.0 + ytm / frequency as f64)
}

pub fn convexity(face_value: f64, coupon_rate: f64, ytm: f64, years_to_maturity: f64, frequency: usize) -> f64 {
    if years_to_maturity <= 0.0 || frequency == 0 {
        return 0.0;
    }
    
    let coupon_payment = face_value * coupon_rate / frequency as f64;
    let periodic_rate = ytm / frequency as f64;
    let total_periods = years_to_maturity * frequency as f64;
    let bond_price = bond_price(face_value, coupon_rate, ytm, years_to_maturity, frequency);
    
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