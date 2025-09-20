use crate::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentSeries {
    pub id: SeriesId,
    pub template_id: TemplateId,
    pub issuer: AgentId,
    pub series_key: SeriesKey,
    pub terms: SeriesTerms,
    pub issuance_data: IssuanceData,
    pub outstanding: OutstandingData,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SeriesKey {
    BondIssue {
        issue_date: NaiveDate,
        maturity_date: NaiveDate,
        coupon_bps: BasisPoints,
        currency: Currency,
    },
    
    DepositProduct {
        product_name: String,
        rate_bps: BasisPoints,
        currency: Currency,
    },
    
    LoanDraw {
        facility_id: Uuid,
        draw_number: u32,
        draw_date: NaiveDate,
    },
    
    Generic {
        series_code: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeriesTerms {
    BondTerms {
        face_value: Money,
        coupon_rate_bps: BasisPoints,
        frequency: u32,
        rating: CreditRating,
        covenants: Vec<Covenant>,
    },
    
    DepositTerms {
        interest_rate_bps: BasisPoints,
        minimum_balance: Option<Money>,
        fee_schedule: Vec<FeeScheduleItem>,
    },
    
    LoanTerms {
        principal: Money,
        rate_structure: RateStructure,
        payment_schedule: PaymentSchedule,
        collateral_requirements: Vec<CollateralRequirement>,
    },
    
    GenericTerms {
        parameters: HashMap<String, serde_json::Value>,
    },
}