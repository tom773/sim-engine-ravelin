use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BankDecision {
    BorrowOvernight { amount_dollars: f64, max_annual_rate_bps: f64 },
    LendOvernight { amount_dollars: f64, min_annual_rate_bps: f64 },
    SetDepositRate { rate: f64 },
    SetLendingRate { rate: f64 },
    ManageReserves { target_level: f64 },
}

impl BankDecision {
    pub fn name(&self) -> &'static str {
        match self {
            BankDecision::BorrowOvernight { .. } => "BorrowOvernight",
            BankDecision::LendOvernight { .. } => "LendOvernight",
            BankDecision::SetDepositRate { .. } => "SetDepositRate",
            BankDecision::SetLendingRate { .. } => "SetLendingRate",
            BankDecision::ManageReserves { .. } => "ManageReserves",
        }
    }
}