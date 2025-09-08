use serde::{Deserialize, Serialize};
use crate::*;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoanPurpose {
    BusinessExpansion,
    WorkingCapital,
    Equipment,
    RealEstate,
    PersonalConsumption,
    Refinancing,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoanTerms {
    pub principal: f64,
    pub annual_rate_bps: BasisPoints,
    pub term_months: u32,
    pub payment_frequency: PaymentFrequency,
    pub collateral_required: bool,
    pub loan_to_value_ratio: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PaymentFrequency {
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
    InterestOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoanApplication {
    pub application_id: Uuid,
    pub borrower_id: AgentId,
    pub requested_amount: f64,
    pub purpose: LoanPurpose,
    pub proposed_collateral: Option<Vec<InstrumentId>>,
    pub borrower_income: Option<f64>,
    pub debt_to_income_ratio: Option<f64>,
    pub application_date: chrono::NaiveDate,
    pub status: ApplicationStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ApplicationStatus {
    Pending,
    UnderReview,
    Approved,
    Rejected,
    Withdrawn,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoanDecision {
    Approve { terms: LoanTerms },
    Reject { reason: String },
    CounterOffer { alternative_terms: LoanTerms },
}