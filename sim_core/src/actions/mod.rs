pub mod banking;
pub mod consumption;
pub mod fiscal;
pub mod labour_action;
pub mod production;
pub mod settlement;
pub mod trading;
pub mod validation;

use crate::*;
pub use banking::*;
use chrono::NaiveDate;
pub use consumption::*;
pub use fiscal::*;
pub use labour_action::*;
pub use production::*;
use serde::{Deserialize, Serialize};
pub use settlement::*;
pub use trading::*;
pub use validation::*;
use uuid::Uuid;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimAction {
    Banking(BankingAction),
    Consumption(ConsumptionAction),
    Fiscal(FiscalAction),
    Production(ProductionAction),
    Settlement(SettlementAction),
    Trading(TradingAction),
    Labour(LabourAction),
}

impl SimAction {
    pub fn name(&self) -> String {
        match self {
            SimAction::Banking(action) => format!("Banking::{}", action.name()),
            SimAction::Consumption(action) => format!("Consumption::{}", action.name()),
            SimAction::Fiscal(action) => format!("Fiscal::{}", action.name()),
            SimAction::Production(action) => format!("Production::{}", action.name()),
            SimAction::Settlement(action) => format!("Settlement::{}", action.name()),
            SimAction::Trading(action) => format!("Trading::{}", action.name()),
            SimAction::Labour(action) => format!("Labour::{}", action.name()),
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            SimAction::Banking(action) => action.agent_id(),
            SimAction::Consumption(action) => action.agent_id(),
            SimAction::Fiscal(action) => action.agent_id(),
            SimAction::Production(action) => action.agent_id(),
            SimAction::Settlement(action) => action.agent_id(),
            SimAction::Trading(action) => action.agent_id(),
            SimAction::Labour(action) => action.agent_id(),
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
}

impl SimIntention {
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
            SimIntention::BidInDebtAuction { agent_id, .. } => *agent_id
        }
    }
}
