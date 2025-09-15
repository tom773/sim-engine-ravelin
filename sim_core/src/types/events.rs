use crate::prelude::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typetag::serde;
use uuid::Uuid;

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
    FinancialTransaction { context: ActionContext, effects: Vec<FinancialEffect> },
    Intention(SimIntention),
    Action(ActionRecord),
    Effect(StateEffect),
    PaymentLifecycle(PaymentLifecycleEvent),
    BalanceChange(BalanceChangeEvent),
    BankingSystem(BankingSystemEvent),
    MarketActivity(MarketActivityEvent),
    EconomicIndicator(EconomicIndicatorEvent),
    MoneyFlowChain(MoneyFlowChainEvent),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchedTradeEvent {
    pub trade_id: Uuid,
    pub market_id: Symbol,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionEventContext {
    pub purpose: String,
    pub chain_id: Option<Uuid>,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentLifecycleEvent {
    pub payment_id: Uuid,
    pub stage: PaymentStage,
    pub from_agent: AgentId,
    pub to_agent: AgentId,
    pub amount: Money,
    pub context: TransactionEventContext,
    pub timestamp: DateTime<Utc>,
    pub queue_position: Option<usize>,
    pub failure_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PaymentStage {
    Initiated,
    Queued { position: usize },
    Processing,
    Settled,
    Failed { reason: String },
    Rejected { reason: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceChangeEvent {
    pub agent_id: AgentId,
    pub account_type: String,
    pub bank_id: Option<AgentId>,
    pub previous_balance: Money,
    pub new_balance: Money,
    pub change_amount: Money,
    pub change_reason: ChangeReason,
    pub related_transaction_id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ChangeReason {
    PaymentReceived { from: AgentId },
    PaymentSent { to: AgentId },
    InterestAccrual,
    TradeSettlement { trade_id: Uuid },
    LoanDisbursement { loan_id: InstrumentId },
    WagePayment { employer: AgentId },
    DepositCreation,
    TaxPayment { government: AgentId },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankingSystemEvent {
    pub event_type: BankingEventType,
    pub bank_id: AgentId,
    pub amount: Money,
    pub reserves_before: Money,
    pub reserves_after: Money,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BankingEventType {
    ReserveMovement { to_bank: Option<AgentId>, reason: String },
    DepositCreation { for_agent: AgentId, account_id: String },
    LiquidityInjection { from_central_bank: bool },
    InterbankTransfer { to_bank: AgentId, reference: String },
    CreditCreation { to_agent: AgentId, instrument_id: InstrumentId },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketActivityEvent {
    pub market_id: Symbol,
    pub activity_type: MarketActivityType,
    pub agent_id: AgentId,
    pub instrument_id: InstrumentId,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MarketActivityType {
    OrderPlaced { order_id: Uuid, side: OrderSide, quantity: f64, price: Money },
    OrderCancelled { order_id: Uuid },
    OrderPartiallyFilled { order_id: Uuid, filled_quantity: f64, remaining_quantity: f64 },
    PriceUpdate { new_bid: Option<Money>, new_ask: Option<Money> },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EconomicIndicatorEvent {
    pub indicator: IndicatorType,
    pub value: f64,
    pub previous_value: Option<f64>,
    pub change_percent: Option<f64>,
    pub measurement_period: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum IndicatorType {
    TotalMoneySupplyM1,
    InflationRateCPI,
    UnemploymentRate,
    BankReserveRatio,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoneyFlowChainEvent {
    pub chain_id: Uuid,
    pub sequence_number: u32,
    pub event_type: ChainEventType,
    pub agent_id: AgentId,
    pub amount: Money,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ChainEventType {
    ChainStart { initiator: AgentId, purpose: String },
    ChainLink { from: AgentId, to: AgentId, step: String },
    ChainEnd { final_recipient: AgentId, outcome: String },
}
