use crate::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CreditAction {
    CreateLoanApplication { application: LoanApplication },

    ProcessLoanApplication { application_id: Uuid, decision: LoanDecision },

    OriginateLoan { loan: Loan, funding_amount: Money },

    OpenCreditFacility { facility: CreditFacilityState },

    FundDrawdown { facility_id: Uuid, loan: Loan, amount: Money },

    AccrueInterest { loan_id: Uuid },

    ProcessScheduledPayment { loan_id: Uuid, payment: ScheduledPayment },

    ApplyPayment { loan_id: Uuid, amount: Money, payment_type: PaymentType },

    CreateLien { loan_id: Uuid, lien: Lien },

    ReleaseLien { lien_id: LienId },

    EnforceLien { lien_id: LienId },

    MarkImpairment { loan_id: Uuid, new_stage: ImpairmentStage, provision: Money },

    WriteOffLoan { loan_id: Uuid, amount: Money },

    RecoverLoan { loan_id: Uuid, recovered_amount: Money },

    TestCovenant { loan_id: Uuid, covenant_index: usize, result: CovenantTestResult },
}
impl CreditAction {
    pub fn name(&self) -> String {
        match self {
            CreditAction::CreateLoanApplication { .. } => "CreateLoanApplication".to_string(),
            CreditAction::ProcessLoanApplication { .. } => "ProcessLoanApplication".to_string(),
            CreditAction::OriginateLoan { .. } => "OriginateLoan".to_string(),
            CreditAction::OpenCreditFacility { .. } => "OpenCreditFacility".to_string(),
            CreditAction::FundDrawdown { .. } => "FundDrawdown".to_string(),
            CreditAction::AccrueInterest { .. } => "AccrueInterest".to_string(),
            CreditAction::ProcessScheduledPayment { .. } => "ProcessScheduledPayment".to_string(),
            CreditAction::ApplyPayment { .. } => "ApplyPayment".to_string(),
            CreditAction::CreateLien { .. } => "CreateLien".to_string(),
            CreditAction::ReleaseLien { .. } => "ReleaseLien".to_string(),
            CreditAction::EnforceLien { .. } => "EnforceLien".to_string(),
            CreditAction::MarkImpairment { .. } => "MarkImpairment".to_string(),
            CreditAction::WriteOffLoan { .. } => "WriteOffLoan".to_string(),
            CreditAction::RecoverLoan { .. } => "RecoverLoan".to_string(),
            CreditAction::TestCovenant { .. } => "TestCovenant".to_string(),
        }
    }
    // TODO
    pub fn agent_id(&self) -> AgentId {
        match self {
            CreditAction::CreateLoanApplication { application } => application.borrower_id,
            CreditAction::ProcessLoanApplication { .. } => AgentId::default(), // Bank agent would be needed here
            CreditAction::OriginateLoan { loan, .. } => loan.state.lender,
            CreditAction::OpenCreditFacility { facility } => facility.borrower,
            CreditAction::FundDrawdown { loan, .. } => loan.state.lender,
            CreditAction::AccrueInterest { .. } => AgentId::default(),
            CreditAction::ProcessScheduledPayment { .. } => AgentId::default(),
            CreditAction::ApplyPayment { .. } => AgentId::default(),
            CreditAction::CreateLien { .. } => AgentId::default(),
            CreditAction::ReleaseLien { .. } => AgentId::default(),
            CreditAction::EnforceLien { .. } => AgentId::default(),
            CreditAction::MarkImpairment { .. } => AgentId::default(),
            CreditAction::WriteOffLoan { .. } => AgentId::default(),
            CreditAction::RecoverLoan { .. } => AgentId::default(),
            CreditAction::TestCovenant { .. } => AgentId::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CovenantTestResult {
    pub passed: bool,
    pub actual_value: f64,
    pub required_value: f64,
    pub test_date: chrono::NaiveDate,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CreditIntention {
    RequestLoan {
        amount: Money,
        purpose: LoanPurpose,
        loan_type: LoanType,
        term_months: u32,
        collateral: Option<Vec<InstrumentId>>,
    },

    BankDecision {
        application_id: Uuid,
        decision: LoanDecision,
    },

    RequestCreditLine {
        amount: Money,
        facility_type: FacilityType,
        purpose: LoanPurpose,
    },

    DrawFromFacility {
        facility_id: Uuid,
        amount: Money,
        term_months: Option<u32>, // None = revolving draw
    },

    MakePayment {
        loan_id: Uuid,
        amount: Money,
        payment_type: PaymentType,
    },

    Prepay {
        loan_id: Uuid,
        amount: Money,
    },

    ModifyTerms {
        loan_id: Uuid,
        modification: LoanModification,
    },

    PledgeCollateral {
        loan_id: Uuid,
        collateral: Vec<InstrumentId>,
    },

    ReleaseCollateral {
        lien_id: LienId,
    },
}

impl CreditIntention {
    pub fn name(&self) -> String {
        match self {
            CreditIntention::RequestLoan { .. } => "RequestLoan".to_string(),
            CreditIntention::BankDecision { .. } => "BankDecision".to_string(),
            CreditIntention::RequestCreditLine { .. } => "RequestCreditLine".to_string(),
            CreditIntention::DrawFromFacility { .. } => "DrawFromFacility".to_string(),
            CreditIntention::MakePayment { .. } => "MakePayment".to_string(),
            CreditIntention::Prepay { .. } => "Prepay".to_string(),
            CreditIntention::ModifyTerms { .. } => "ModifyTerms".to_string(),
            CreditIntention::PledgeCollateral { .. } => "PledgeCollateral".to_string(),
            CreditIntention::ReleaseCollateral { .. } => "ReleaseCollateral".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PaymentType {
    Scheduled,
    Principal,
    Interest,
    Fee,
    Payoff,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoanModification {
    ExtendMaturity { new_date: chrono::NaiveDate },
    ChangeRate { new_spread_bps: BasisPoints },
    Restructure { new_terms: LoanTerms },
    Forbearance { until_date: chrono::NaiveDate },
}
