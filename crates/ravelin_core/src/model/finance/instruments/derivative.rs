use chrono::{Datelike, NaiveDate};
use crate::*;
use serde::{Deserialize, Serialize};
pub trait Derivative {
    fn get_underlying_instrument_id(&self) -> &InstrumentId;
    fn get_notional_amount(&self) -> f64;
    fn get_maturity_date(&self) -> NaiveDate;
    fn get_details(&self) -> &Box<dyn InstrumentDetails>;
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentFrequency {
    Quarterly,
    SemiAnnually,
    Annually,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterestRateSwapDetails {
    pub notional_principal: f64,
    pub fixed_rate: f64, // The rate the fixed-leg payer pays
    pub maturity_date: NaiveDate,
    
    pub fixed_leg_payer: AgentId,
    pub floating_leg_payer: AgentId,
    
    pub next_payment_date: NaiveDate,
    pub payment_frequency: PaymentFrequency,
}

#[typetag::serde]
impl InstrumentDetails for InterestRateSwapDetails {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl InterestRateSwapDetails {
    fn calculate_next_payment_date(&self, from_date: NaiveDate) -> NaiveDate {
        match self.payment_frequency {
            PaymentFrequency::Quarterly => from_date.with_month(from_date.month() + 3).unwrap(),
            PaymentFrequency::SemiAnnually => from_date.with_month(from_date.month() + 6).unwrap(),
            PaymentFrequency::Annually => from_date.with_year(from_date.year() + 1).unwrap(),
        }
    }
}
