use crate::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BankingAction {
    PostInterbankLendingOffer { lender_id: AgentId, amount: f64, rate_bps: BasisPoints },
    PostInterbankBorrowingRequest { borrower_id: AgentId, amount: f64, rate_bps: BasisPoints },
    ExecuteInterbankLoan { lender_id: AgentId, borrower_id: AgentId, amount: f64, rate_bps: BasisPoints },
    OriginateLoan { lender_id: AgentId, borrower_id: AgentId, loan_terms: LoanTerms, application_id: Uuid },
    CreateLoanApplication { bank_id: AgentId, application: LoanApplication },
    ProcessLoanApplication { bank_id: AgentId, application_id: Uuid, decision: LoanDecision },
}

impl BankingAction {
    pub fn name(&self) -> &'static str {
        match self {
            BankingAction::PostInterbankLendingOffer { .. } => "PostInterbankLendingOffer",
            BankingAction::PostInterbankBorrowingRequest { .. } => "PostInterbankBorrowingRequest",
            BankingAction::ExecuteInterbankLoan { .. } => "ExecuteInterbankLoan",
            BankingAction::OriginateLoan { .. } => "OriginateLoan",
            BankingAction::CreateLoanApplication { .. } => "CreateLoanApplication",
            BankingAction::ProcessLoanApplication { .. } => "ProcessLoanApplication",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            BankingAction::PostInterbankLendingOffer { lender_id, .. } => *lender_id,
            BankingAction::PostInterbankBorrowingRequest { borrower_id, .. } => *borrower_id,
            BankingAction::ExecuteInterbankLoan { lender_id, .. } => *lender_id,
            BankingAction::OriginateLoan { lender_id, .. } => *lender_id,
            BankingAction::CreateLoanApplication { bank_id, .. } => *bank_id,
            BankingAction::ProcessLoanApplication { bank_id, .. } => *bank_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionContext {
    GoodsPurchase { market_id: MarketId },
    TradeSettlement { trade_id: Uuid },
    WagePayment { employer: AgentId, employee: AgentId, amount: f64 },
    TaxPayment { payer: AgentId, amount: f64 },
    GovTranseferPayment { recipient: AgentId, amount: f64 },
    CouponPayment { instrument_id: InstrumentId },
    PrincipalRepayment { instrument_id: InstrumentId },
    GenericTransfer { from: AgentId, to: AgentId, amount: f64 },
}

impl TransactionContext {
    pub fn name(&self) -> &'static str {
        match self {
            TransactionContext::GoodsPurchase { .. } => "GoodsPurchase",
            TransactionContext::TradeSettlement { .. } => "TradeSettlement",
            TransactionContext::WagePayment { .. } => "WagePayment",
            TransactionContext::TaxPayment { .. } => "TaxPayment",
            TransactionContext::GovTranseferPayment { .. } => "GovTranseferPayment",
            TransactionContext::CouponPayment { .. } => "CouponPayment",
            TransactionContext::PrincipalRepayment { .. } => "PrincipalRepayment",
            TransactionContext::GenericTransfer { .. } => "GenericTransfer",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BankingIntention {
    LendExcessReserves {
        agent_id: AgentId,
        amount: f64,
        target_rate_bps: BasisPoints,
    },
    BorrowReserves {
        agent_id: AgentId,
        amount: f64,
        target_rate_bps: BasisPoints,
    },
    MarketMakeTreasuries {
        agent_id: AgentId,
        maturity_date: NaiveDate,
        quantity: f64,
        bid_yield_bps: BasisPoints,
        ask_yield_bps: BasisPoints,
    },
    DepositFunds {
        agent_id: AgentId,
        bank: AgentId,
        amount: f64,
    },
    WithdrawFunds {
        agent_id: AgentId,
        bank: AgentId,
        amount: f64,
    },
    RequestLoan {
        agent_id: AgentId,
        bank_id: AgentId,
        amount: f64,
        purpose: LoanPurpose,
        collateral: Option<Vec<InstrumentId>>,
    },

    ApproveLoan {
        bank_id: AgentId,
        borrower_id: AgentId,
        amount: f64,
        terms: LoanTerms,
    },

    RejectLoan {
        bank_id: AgentId,
        borrower_id: AgentId,
        application_id: Uuid,
        reason: String,
    },
}

impl BankingIntention {
    pub fn name(&self) -> &'static str {
        match self {
            BankingIntention::LendExcessReserves { .. } => "LendExcessReserves",
            BankingIntention::BorrowReserves { .. } => "BorrowReserves",
            BankingIntention::MarketMakeTreasuries { .. } => "MarketMakeTreasuries",
            BankingIntention::DepositFunds { .. } => "DepositFunds",
            BankingIntention::WithdrawFunds { .. } => "WithdrawFunds",
            BankingIntention::RequestLoan { .. } => "RequestLoan",
            BankingIntention::ApproveLoan { .. } => "ApproveLoan",
            BankingIntention::RejectLoan { .. } => "RejectLoan",
        }
    }
    pub fn agent_id(&self) -> AgentId {
        match self {
            BankingIntention::LendExcessReserves { agent_id, .. } => *agent_id,
            BankingIntention::BorrowReserves { agent_id, .. } => *agent_id,
            BankingIntention::MarketMakeTreasuries { agent_id, .. } => *agent_id,
            BankingIntention::DepositFunds { agent_id, .. } => *agent_id,
            BankingIntention::WithdrawFunds { agent_id, .. } => *agent_id,
            BankingIntention::RequestLoan { agent_id, .. } => *agent_id,
            BankingIntention::ApproveLoan { bank_id, .. } => *bank_id,
            BankingIntention::RejectLoan { bank_id, .. } => *bank_id,
        }
    }
}
