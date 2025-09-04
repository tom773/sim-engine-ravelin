use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BankingAction {
    InitiatePayment {
        from: AgentId,
        to: AgentId,
        amount: f64,
        context: TransactionContext, 
    },

    Deposit {
        agent_id: AgentId,
        bank: AgentId,
        amount: f64,
    },

    Withdraw {
        agent_id: AgentId,
        bank: AgentId,
        amount: f64,
    },

    PostInterbankLendingOffer {
        lender_id: AgentId,
        amount: f64,
        rate_bps: BasisPoints,
    },

    PostInterbankBorrowingRequest {
        borrower_id: AgentId,
        amount: f64,
        rate_bps: BasisPoints,
    },

    ExecuteInterbankLoan {
        lender_id: AgentId,
        borrower_id: AgentId,
        amount: f64,
        rate_bps: BasisPoints,
    },

    InjectLiquidity,
}

impl BankingAction {
    pub fn name(&self) -> &'static str {
        match self {
            BankingAction::Deposit { .. } => "Deposit",
            BankingAction::Withdraw { .. } => "Withdraw", 
            BankingAction::InjectLiquidity => "InjectLiquidity",
            BankingAction::PostInterbankLendingOffer { .. } => "PostInterbankLendingOffer",
            BankingAction::PostInterbankBorrowingRequest { .. } => "PostInterbankBorrowingRequest",
            BankingAction::ExecuteInterbankLoan { .. } => "ExecuteInterbankLoan",
            BankingAction::InitiatePayment { .. } => "InitiatePayment",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            BankingAction::Deposit { agent_id, .. } => *agent_id,
            BankingAction::Withdraw { agent_id, .. } => *agent_id,
            BankingAction::InjectLiquidity => AgentId::default(),
            BankingAction::PostInterbankLendingOffer { lender_id, .. } => *lender_id,
            BankingAction::PostInterbankBorrowingRequest { borrower_id, .. } => *borrower_id,
            BankingAction::ExecuteInterbankLoan { lender_id, .. } => *lender_id,
            BankingAction::InitiatePayment { from, .. } => *from,
        }
    }
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionContext {
    GoodsPurchase {
        market_id: MarketId,
    },
    TradeSettlement {
        trade_id: Uuid,
    },
    WagePayment,
    TaxPayment,
    GovTranseferPayment,
    CouponPayment {
        instrument_id: InstrumentId,
    },
    PrincipalRepayment {
        instrument_id: InstrumentId,
    },
    GenericTransfer,
}

impl TransactionContext {
    pub fn name(&self) -> &'static str {
        match self {
            TransactionContext::GoodsPurchase { .. } => "GoodsPurchase",
            TransactionContext::TradeSettlement { .. } => "TradeSettlement",
            TransactionContext::WagePayment => "WagePayment",
            TransactionContext::TaxPayment => "TaxPayment",
            TransactionContext::GovTranseferPayment => "GovTranseferPayment",
            TransactionContext::CouponPayment { .. } => "CouponPayment",
            TransactionContext::PrincipalRepayment { .. } => "PrincipalRepayment",
            TransactionContext::GenericTransfer => "GenericTransfer",
        }
    }
}