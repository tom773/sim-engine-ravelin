use crate::prelude::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use typetag::serde;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionContext {
    pub action_instance_id: Uuid,
    pub action_name: String,
    pub agent_id: AgentId,
    pub tick: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SimEvent {
    MatchedTrade(MatchedTradeEvent),
    CashFlow(CashFlowEvent),
    InstrumentLifecycle(InstrumentLifecycleEvent),
    TransactionRecord(Transaction),
    BalanceSheetUpdate(BalanceSheetUpdateEvent),
    FinancialTransaction {
        context: ActionContext,
        effects: Vec<FinancialEffect>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchedTradeEvent {
    pub trade_id: Uuid,
    pub market_id: MarketId,
    pub buyer_id: AgentId,
    pub seller_id: AgentId,
    pub quantity: f64,
    pub price: Money,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub ts: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CashFlowEvent {
    pub from_agent_id: AgentId,
    pub to_agent_id: AgentId,
    pub amount: f64,
    pub reason: String,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub ts: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "lifecycle_event")]
pub enum InstrumentLifecycleEvent {
    Created {
        instrument_id: InstrumentId,
        creditor_id: AgentId,
        debtor_id: AgentId,
        quantity: f64,
        instrument_type: String,
    },
    Removed {
        instrument_id: InstrumentId,
    },
}

impl InstrumentLifecycleEvent {
    pub fn ts(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceSheetUpdateEvent {
    pub owner_id: AgentId,
    pub instrument_id: InstrumentId,
    pub quantity_change: f64,
    pub new_total_quantity: f64,
    pub side: PositionSide,
}