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
    },
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FiscalIntention {
    IssueDebtToRaise {
        government_id: AgentId,
        maturity_date: NaiveDate,
        amount_to_raise: f64,
        coupon_rate: BasisPoints,
    },
    CollectTaxes {
        government_id: AgentId,
        target: AgentId,
        amount: f64,
    },
    AnnounceDebtAuction {
        government_id: AgentId,
        maturity_date: NaiveDate,
        coupon_rate: BasisPoints,
        quantity_to_issue: u32,
    },
    BidInDebtAuction {
        agent_id: AgentId,
        auction_id: Uuid,
        quantity: u32,
        bid_price: Money,
    },
}

impl FiscalIntention {
    pub fn name(&self) -> &'static str {
        match self {
            FiscalIntention::IssueDebtToRaise { .. } => "IssueDebtToRaise",
            FiscalIntention::CollectTaxes { .. } => "CollectTaxes",
            FiscalIntention::AnnounceDebtAuction { .. } => "AnnounceDebtAuction",
            FiscalIntention::BidInDebtAuction { .. } => "BidInDebtAuction",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            FiscalIntention::IssueDebtToRaise { government_id, .. } => *government_id,
            FiscalIntention::CollectTaxes { government_id, .. } => *government_id,
            FiscalIntention::AnnounceDebtAuction { government_id, .. } => *government_id,
            FiscalIntention::BidInDebtAuction { agent_id, .. } => *agent_id,
        }
    }
}
