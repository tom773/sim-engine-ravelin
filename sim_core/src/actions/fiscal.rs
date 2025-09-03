use crate::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FiscalAction {
    ChangeTaxRate {
        government_id: AgentId,
        tax_type: TaxType,
        new_rate: f64,
    },
    SetSpendingTarget {
        government_id: AgentId,
        target: SpendingTargets,
        new_level: f64,
    },
    AnnounceDebtAuction {
        government_id: AgentId,
        maturity: NaiveDate,
        quantity: u32,
        coupon_rate: BasisPoints,
    },
    BidInDebtAuction {
        agent_id: AgentId,
        auction_id: Uuid, // To identify which auction
        quantity: u32,
        bid_price: Money, // Or bid_yield
    }
}

impl FiscalAction {
    pub fn name(&self) -> &'static str {
        match self {
            FiscalAction::ChangeTaxRate { .. } => "ChangeTaxRate",
            FiscalAction::SetSpendingTarget { .. } => "SetSpendingTarget",
            FiscalAction::AnnounceDebtAuction { .. } => "AnnounceDebtAuction",
            FiscalAction::BidInDebtAuction { .. } => "BidInDebtAuction",
        }
    }
    pub fn agent_id(&self) -> AgentId {
        match self {
            FiscalAction::ChangeTaxRate { government_id, .. } => *government_id,
            FiscalAction::SetSpendingTarget { government_id, .. } => *government_id,
            FiscalAction::AnnounceDebtAuction { government_id, .. } => *government_id,
            FiscalAction::BidInDebtAuction { agent_id, .. } => *agent_id,
        }
    }
}