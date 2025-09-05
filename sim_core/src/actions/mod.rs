pub mod banking;
pub mod consumption;
pub mod fiscal;
pub mod production;
pub mod validation;
pub mod transaction;
pub mod monetary;


pub use consumption::*;
pub use fiscal::*;
pub use production::*;
pub use banking::*;
pub use validation::*;
pub use transaction::*;
pub use monetary::*;

use crate::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimAction {
    Banking(BankingAction),
    Consumption(ConsumptionAction),
    Fiscal(FiscalAction),
    Production(ProductionAction),
    Transaction(TransactionAction),
    Monetary(MonetaryAction),
}

impl SimAction {
    pub fn name(&self) -> String {
        match self {
            SimAction::Banking(action) => format!("Banking::{}", action.name()),
            SimAction::Consumption(action) => format!("Consumption::{}", action.name()),
            SimAction::Fiscal(action) => format!("Fiscal::{}", action.name()),
            SimAction::Production(action) => format!("Production::{}", action.name()),
            SimAction::Transaction(action) => format!("Transaction::{}", action.name()),
            SimAction::Monetary(action) => format!("Monetary::{}", action.name()),
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            SimAction::Banking(action) => action.agent_id(),
            SimAction::Consumption(action) => action.agent_id(),
            SimAction::Fiscal(action) => action.agent_id(),
            SimAction::Production(action) => action.agent_id(),
            SimAction::Transaction(action) => action.agent_id(),
            SimAction::Monetary(action) => action.agent_id(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimIntention {
    SpendOnGood {
        agent_id: AgentId,
        good_id: GoodId,
        max_notional: f64,
    },
    PurchaseInputs {
        agent_id: AgentId,
        good_id: GoodId,
        quantity: f64,
        max_price: f64,
    },
    ConsumeGood {
        agent_id: AgentId,
        good_id: GoodId,
        quantity: f64,
    },
    PostGoodToMarket {
        agent_id: AgentId,
        good_id: GoodId,
        quantity: f64,
        ask_price: f64,
    },
    Produce {
        agent_id: AgentId,
        recipe_id: RecipeId,
        batches: u32,
    },

    ApplyForJob {
        agent_id: AgentId,
        market_id: LabourMarketId,
        application: JobApplication,
    },
    HireWorkers {
        agent_id: AgentId,
        count: u32,
        wage_rate: f64,
    },

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
    PayWages {
        employer: AgentId,
        employee: AgentId,
        amount: f64,
    },

    InjectLiquidity,
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
        ConductOMO {
        cb_id: AgentId,
        operation_type: OMOType,
        amount: f64,
    },
    SetPolicyRate {
        cb_id: AgentId,
        new_rate_bps: BasisPoints,
    },
    AdjustReserveRequirement {
        cb_id: AgentId,
        new_ratio: f64,
    },
    ProvideLiquidityFacility {
        cb_id: AgentId,
        bank_id: AgentId,
        amount: f64,
        collateral: Option<Vec<InstrumentId>>,
    },
}

impl SimIntention {
    pub fn name(&self) -> String {
        match self {
            SimIntention::SpendOnGood { .. } => "SpendOnGood".to_string(),
            SimIntention::PurchaseInputs { .. } => "PurchaseInputs".to_string(),
            SimIntention::ConsumeGood { .. } => "ConsumeGood".to_string(),
            SimIntention::PostGoodToMarket { .. } => "PostGoodToMarket".to_string(),
            SimIntention::Produce { .. } => "Produce".to_string(),
            SimIntention::ApplyForJob { .. } => "ApplyForJob".to_string(),
            SimIntention::HireWorkers { .. } => "HireWorkers".to_string(),
            SimIntention::IssueDebtToRaise { .. } => "IssueDebtToRaise".to_string(),
            SimIntention::CollectTaxes { .. } => "CollectTaxes".to_string(),
            SimIntention::LendExcessReserves { .. } => "LendExcessReserves".to_string(),
            SimIntention::BorrowReserves { .. } => "BorrowReserves".to_string(),
            SimIntention::MarketMakeTreasuries { .. } => "MarketMakeTreasuries".to_string(),
            SimIntention::DepositFunds { .. } => "DepositFunds".to_string(),
            SimIntention::WithdrawFunds { .. } => "WithdrawFunds".to_string(),
            SimIntention::PayWages { .. } => "PayWages".to_string(),
            SimIntention::InjectLiquidity => "InjectLiquidity".to_string(),
            SimIntention::AnnounceDebtAuction { .. } => "AnnounceDebtAuction".to_string(),
            SimIntention::BidInDebtAuction { .. } => "BidInDebtAuction".to_string(),
            SimIntention::ConductOMO { .. } => "ConductOMO".to_string(),
            SimIntention::SetPolicyRate { .. } => "SetPolicyRate".to_string(),
            SimIntention::AdjustReserveRequirement { .. } => "AdjustReserveRequirement".to_string(),
            SimIntention::ProvideLiquidityFacility { .. } => "ProvideLiquidityFacility".to_string()
        }
    }
    pub fn agent_id(&self) -> AgentId {
        match self {
            SimIntention::SpendOnGood { agent_id, .. } => *agent_id,
            SimIntention::PurchaseInputs { agent_id, .. } => *agent_id,
            SimIntention::ConsumeGood { agent_id, .. } => *agent_id,
            SimIntention::PostGoodToMarket { agent_id, .. } => *agent_id,
            SimIntention::Produce { agent_id, .. } => *agent_id,
            SimIntention::ApplyForJob { agent_id, .. } => *agent_id,
            SimIntention::HireWorkers { agent_id, .. } => *agent_id,
            SimIntention::IssueDebtToRaise { government_id, .. } => *government_id,
            SimIntention::CollectTaxes { government_id, .. } => *government_id,
            SimIntention::LendExcessReserves { agent_id, .. } => *agent_id,
            SimIntention::BorrowReserves { agent_id, .. } => *agent_id,
            SimIntention::MarketMakeTreasuries { agent_id, .. } => *agent_id,
            SimIntention::DepositFunds { agent_id, .. } => *agent_id,
            SimIntention::WithdrawFunds { agent_id, .. } => *agent_id,
            SimIntention::PayWages { employer, .. } => *employer,
            SimIntention::InjectLiquidity => AgentId::default(),
            SimIntention::AnnounceDebtAuction { government_id, .. } => *government_id,
            SimIntention::BidInDebtAuction { agent_id, .. } => *agent_id,
            SimIntention::ConductOMO { cb_id, .. } => *cb_id,
            SimIntention::SetPolicyRate { cb_id, .. } => *cb_id,
            SimIntention::AdjustReserveRequirement { cb_id, .. } => *cb_id,
            SimIntention::ProvideLiquidityFacility { cb_id, .. } => *cb_id,
        }
    }
}
