use serde::{Deserialize, Serialize};
use crate::*;
use uuid::Uuid;
use std::collections::HashMap;
use chrono::NaiveDate;

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentRecord {
    pub payment_date: NaiveDate,
    pub due_date: NaiveDate,
    pub amount: f64,
    pub principal_paid: f64,
    pub interest_paid: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoanStatus {
    Current,
    Deliquent30,
    Deliquent60,
    Deliquent90,
    Defaulted,
    Resolved,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CreditRegistry {
    pub applications: HashMap<Uuid, LoanApplication>,
    pub active_loans: HashMap<InstrumentId, ActiveLoan>,
    pub loans_by_borrower: HashMap<AgentId, Vec<InstrumentId>>,
    pub loans_by_lender: HashMap<AgentId, Vec<InstrumentId>>,
    pub applications_by_bank: HashMap<AgentId, Vec<Uuid>>,
    pub credit_histories: HashMap<AgentId, CreditHistory>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActiveLoan {
    pub instrument_id: InstrumentId,
    pub origination_date: NaiveDate,
    pub original_terms: LoanTerms,
    pub outstanding_principal: f64,
    pub payment_history: Vec<PaymentRecord>,
    pub status: LoanStatus,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreditHistory {
    pub total_loans_originated: u32,
    pub total_loans_repaid: u32,
    pub total_defaults: u32,
    pub current_debt_service: f64,
    pub payment_performance: f64,
}