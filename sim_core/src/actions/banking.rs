use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BankingAction {
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
    Transfer {
        from: AgentId,
        to: AgentId,
        amount: f64,
    },
    PayWages {
        agent_id: AgentId,
        employee: AgentId,
        amount: f64,
    },
    UpdateReserves {
        bank: AgentId,
        amount_change: f64,
    },
    InjectLiquidity,
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
}

impl BankingAction {
    pub fn name(&self) -> &'static str {
        match self {
            BankingAction::Deposit { .. } => "Deposit",
            BankingAction::Withdraw { .. } => "Withdraw", 
            BankingAction::Transfer { .. } => "Transfer",
            BankingAction::PayWages { .. } => "PayWages",
            BankingAction::UpdateReserves { .. } => "UpdateReserves",
            BankingAction::InjectLiquidity => "InjectLiquidity",
            BankingAction::PostInterbankLendingOffer { .. } => "PostInterbankLendingOffer",
            BankingAction::PostInterbankBorrowingRequest { .. } => "PostInterbankBorrowingRequest",
            BankingAction::ExecuteInterbankLoan { .. } => "ExecuteInterbankLoan",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            BankingAction::Deposit { agent_id, .. } => *agent_id,
            BankingAction::Withdraw { agent_id, .. } => *agent_id,
            BankingAction::Transfer { from, .. } => *from,
            BankingAction::PayWages { agent_id, .. } => *agent_id,
            BankingAction::UpdateReserves { bank, .. } => *bank,
            BankingAction::InjectLiquidity => AgentId::default(),
            BankingAction::PostInterbankLendingOffer { lender_id, .. } => *lender_id,
            BankingAction::PostInterbankBorrowingRequest { borrower_id, .. } => *borrower_id,
            BankingAction::ExecuteInterbankLoan { lender_id, .. } => *lender_id,
        }
    }
}
