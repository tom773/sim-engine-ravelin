use crate::actions::credit_actions::CreditIntention;
use crate::effects::market_effects::LabourMarketUpdate;
use crate::prelude::*;
use crate::types::instrument::archetypes::CollateralType;
use crate::types::instrument::credit::{CreditFacility, Lien, Loan, LoanApplication, LoanPurpose};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum SimEventKind {
    MatchedTrade,
    CashFlow,
    InstrumentLifecycle,
    TransactionRecord,
    BalanceSheetUpdate,
    FinancialTransaction,
    Intention,
    Action,
    Effect,
    PaymentLifecycle,
    BalanceChange,
    BankingSystem,
    MarketActivity,
    EconomicIndicator,
    MoneyFlowChain,
}

impl SimEvent {
    pub fn kind(&self) -> SimEventKind {
        match self {
            SimEvent::MatchedTrade(_) => SimEventKind::MatchedTrade,
            SimEvent::CashFlow(_) => SimEventKind::CashFlow,
            SimEvent::InstrumentLifecycle(_) => SimEventKind::InstrumentLifecycle,
            SimEvent::TransactionRecord(_) => SimEventKind::TransactionRecord,
            SimEvent::BalanceSheetUpdate(_) => SimEventKind::BalanceSheetUpdate,
            SimEvent::FinancialTransaction { .. } => SimEventKind::FinancialTransaction,
            SimEvent::Intention(_) => SimEventKind::Intention,
            SimEvent::Action(_) => SimEventKind::Action,
            SimEvent::Effect(_) => SimEventKind::Effect,
            SimEvent::PaymentLifecycle(_) => SimEventKind::PaymentLifecycle,
            SimEvent::BalanceChange(_) => SimEventKind::BalanceChange,
            SimEvent::BankingSystem(_) => SimEventKind::BankingSystem,
            SimEvent::MarketActivity(_) => SimEventKind::MarketActivity,
            SimEvent::EconomicIndicator(_) => SimEventKind::EconomicIndicator,
            SimEvent::MoneyFlowChain(_) => SimEventKind::MoneyFlowChain,
        }
    }

    pub fn compact(&self) -> CompactEvent {
        match self {
            SimEvent::MatchedTrade(evt) => CompactEvent::triplet(
                "matched_trade",
                json!(evt.market_id.to_string()),
                json!({
                    "trade": short_uuid(evt.trade_id),
                    "buyer": short_agent(evt.buyer_id),
                    "seller": short_agent(evt.seller_id),
                    "qty": round_qty(evt.quantity),
                    "price": round_money(evt.price),
                    "ts": evt.ts.timestamp_millis(),
                }),
            ),
            SimEvent::CashFlow(evt) => CompactEvent::triplet(
                "cash_flow",
                json!(format!("{} -> {}", short_agent(evt.from_agent_id), short_agent(evt.to_agent_id))),
                json!({
                    "amount": round_amount(evt.amount),
                    "reason": evt.reason,
                    "ts": evt.ts.timestamp_millis(),
                }),
            ),
            SimEvent::InstrumentLifecycle(evt) => match evt {
                InstrumentLifecycleEvent::Created {
                    instrument_id,
                    creditor_id,
                    debtor_id,
                    quantity,
                    instrument_type,
                } => CompactEvent::triplet(
                    "instrument_lifecycle",
                    json!("created"),
                    json!({
                        "instrument": short_instrument(*instrument_id),
                        "creditor": short_agent(*creditor_id),
                        "debtor": short_agent(*debtor_id),
                        "qty": round_qty(*quantity),
                        "instrument_type": instrument_type,
                    }),
                ),
                InstrumentLifecycleEvent::Removed { instrument_id } => CompactEvent::triplet(
                    "instrument_lifecycle",
                    json!("removed"),
                    json!({ "instrument": short_instrument(*instrument_id) }),
                ),
            },
            SimEvent::TransactionRecord(tx) => CompactEvent::triplet(
                "transaction",
                json!(short_uuid(tx.id)),
                json!({
                    "from": short_agent(tx.from_agent),
                    "to": short_agent(tx.to_agent),
                    "amount": round_amount(tx.amount),
                    "kind": tx.transaction_type,
                    "date": tx.timestamp.to_string(),
                    "instrument": tx.instrument_id.map(short_instrument),
                }),
            ),
            SimEvent::BalanceSheetUpdate(evt) => CompactEvent::triplet(
                "balance_sheet_update",
                json!(short_agent(evt.owner_id)),
                json!({
                    "instrument": short_instrument(evt.instrument_id),
                    "delta": round_qty(evt.quantity_change),
                    "total": round_qty(evt.new_total_quantity),
                    "side": format!("{:?}", evt.side),
                }),
            ),
            SimEvent::FinancialTransaction { context, effects } => CompactEvent::triplet(
                "financial_transaction",
                json!(context.action_name),
                json!({
                    "agent": short_agent(context.agent_id),
                    "tick": context.tick,
                    "action_id": short_uuid(context.action_instance_id),
                    "effects": effects.iter().map(|e| e.name()).collect::<Vec<_>>(),
                }),
            ),
            SimEvent::Intention(intention) => CompactEvent::triplet(
                "intention",
                json!(intention.name()),
                json!({
                    "agent": intention_agent(intention).map(short_agent),
                }),
            ),
            SimEvent::Action(action) => CompactEvent::triplet(
                "action",
                json!(action.action.name()),
                json!({
                    "agent": short_agent(action.agent_id),
                    "agent_type": action.agent_type,
                    "agent_name": action.agent_name,
                    "action_id": short_uuid(action.id),
                }),
            ),
            SimEvent::Effect(effect) => {
                CompactEvent::triplet("effect", json!(effect.name()), compact_state_effect(effect))
            }
            SimEvent::PaymentLifecycle(evt) => CompactEvent::triplet(
                "payment_lifecycle",
                json!(short_uuid(evt.payment_id)),
                json!({
                    "stage": compact_payment_stage(&evt.stage),
                    "from": short_agent(evt.from_agent),
                    "to": short_agent(evt.to_agent),
                    "amount": round_money(evt.amount),
                    "queue_position": evt.queue_position,
                    "reason": evt.failure_reason,
                    "timestamp": evt.timestamp.timestamp_millis(),
                }),
            ),
            SimEvent::BalanceChange(evt) => CompactEvent::triplet(
                "balance_change",
                json!(short_agent(evt.agent_id)),
                json!({
                    "account_type": evt.account_type,
                    "bank": evt.bank_id.map(short_agent),
                    "before": round_money(evt.previous_balance),
                    "after": round_money(evt.new_balance),
                    "change": round_money(evt.change_amount),
                    "reason": compact_change_reason(&evt.change_reason),
                    "related_tx": evt.related_transaction_id.map(short_uuid),
                    "timestamp": evt.timestamp.timestamp_millis(),
                }),
            ),
            SimEvent::BankingSystem(evt) => CompactEvent::triplet(
                "banking_system",
                json!(short_agent(evt.bank_id)),
                json!({
                    "event": compact_banking_event_type(&evt.event_type),
                    "amount": round_money(evt.amount),
                    "reserves_before": round_money(evt.reserves_before),
                    "reserves_after": round_money(evt.reserves_after),
                    "timestamp": evt.timestamp.timestamp_millis(),
                }),
            ),
            SimEvent::MarketActivity(evt) => CompactEvent::triplet(
                "market_activity",
                json!(evt.market_id.to_string()),
                compact_market_activity(&evt.activity_type, evt.agent_id, evt.instrument_id),
            ),
            SimEvent::EconomicIndicator(evt) => CompactEvent::triplet(
                "economic_indicator",
                json!(format!("{:?}", evt.indicator)),
                json!({
                    "value": round_amount(evt.value),
                    "previous": evt.previous_value.map(round_amount),
                    "change": evt.change_percent.map(round_amount),
                    "period": evt.measurement_period,
                    "timestamp": evt.timestamp.timestamp_millis(),
                }),
            ),
            SimEvent::MoneyFlowChain(evt) => CompactEvent::triplet(
                "money_flow_chain",
                json!(short_uuid(evt.chain_id)),
                compact_money_flow_event(evt),
            ),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventKindCount {
    pub kind: SimEventKind,
    pub count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TickEventSummary {
    pub total_events: usize,
    pub by_kind: Vec<EventKindCount>,
}

impl TickEventSummary {
    pub fn from_events(events: &[SimEvent]) -> Self {
        let mut counts: HashMap<SimEventKind, usize> = HashMap::new();
        for event in events {
            *counts.entry(event.kind()).or_insert(0) += 1;
        }

        let mut by_kind: Vec<EventKindCount> =
            counts.into_iter().map(|(kind, count)| EventKindCount { kind, count }).collect();
        by_kind.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.kind.cmp(&b.kind)));

        Self { total_events: events.len(), by_kind }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CompactEvent(pub Vec<Value>);

impl CompactEvent {
    pub fn triplet(kind: &str, label: Value, payload: Value) -> Self {
        CompactEvent(vec![json!(kind), label, payload])
    }
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

fn short_uuid(id: Uuid) -> String {
    let s = id.to_string();
    s.chars().take(8).collect()
}

fn short_agent(id: AgentId) -> String {
    short_uuid(id.0)
}

fn short_instrument(id: InstrumentId) -> String {
    short_uuid(id.0)
}

fn short_good(id: GoodId) -> String {
    short_uuid(id.0)
}

fn short_labour(id: LabourMarketId) -> String {
    short_uuid(id.0)
}

fn round_to(value: f64, decimals: i32) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let factor = 10_f64.powi(decimals);
    (value * factor).round() / factor
}

fn round_amount(value: f64) -> f64 {
    round_to(value, 2)
}
fn round_qty(value: f64) -> f64 {
    round_to(value, 4)
}

fn round_money(money: Money) -> f64 {
    round_amount(money.to_f64())
}

fn intention_agent(intention: &SimIntention) -> Option<AgentId> {
    let id = match intention {
        SimIntention::Credit(credit) => match credit {
            CreditIntention::RequestLoan { .. } => None,
            CreditIntention::BankDecision { .. } => None,
            CreditIntention::RequestCreditLine { .. } => None,
            CreditIntention::DrawFromFacility { .. } => None,
            CreditIntention::MakePayment { .. } => None,
            CreditIntention::Prepay { .. } => None,
            CreditIntention::ModifyTerms { .. } => None,
            CreditIntention::PledgeCollateral { .. } => None,
            CreditIntention::ReleaseCollateral { .. } => None,
        },
        _ => Some(intention.agent_id()),
    };

    id.and_then(|a| if a == AgentId::default() { None } else { Some(a) })
}

fn compact_state_effect(effect: &StateEffect) -> Value {
    match effect {
        StateEffect::Financial(fin) => compact_financial_effect(fin),
        StateEffect::Inventory(inv) => compact_inventory_effect(inv),
        StateEffect::Market(market) => compact_market_effect(market),
        StateEffect::Agent(agent) => compact_agent_effect(agent),
        StateEffect::Monetary(monetary) => compact_monetary_effect(monetary),
        StateEffect::Credit(credit) => compact_credit_effect(credit),
    }
}

fn compact_financial_effect(effect: &FinancialEffect) -> Value {
    match effect {
        FinancialEffect::CreateInstrument { instrument, creditor, debtor, quantity } => json!({
            "instrument": short_instrument(instrument.id),
            "instrument_type": instrument.type_as_string(),
            "creditor": short_agent(*creditor),
            "debtor": short_agent(*debtor),
            "qty": round_qty(*quantity),
        }),
        FinancialEffect::RecordTransaction(tx) => compact_transaction_payload(tx),
        FinancialEffect::RecordSettlementInstruction(instr) => json!({
            "instruction": short_uuid(instr.instruction_id),
            "trade": short_uuid(instr.trade_id),
            "seller": short_agent(instr.seller),
            "buyer": short_agent(instr.buyer),
            "instrument": short_instrument(instr.instrument_id),
            "qty": round_qty(instr.quantity),
            "cash": round_amount(instr.cash_amount),
            "settlement_date": instr.settlement_date.to_string(),
            "status": format!("{:?}", instr.status),
        }),
        FinancialEffect::QueuePayment(pi) => json!({
            "payment": short_uuid(pi.id),
            "from_bank": short_agent(pi.from_bank),
            "to_bank": short_agent(pi.to_bank),
            "payer": short_agent(pi.payer),
            "payee": short_agent(pi.payee),
            "amount": round_amount(pi.amount),
            "priority": format!("{:?}", pi.priority),
        }),
        FinancialEffect::DvPFinalize { trade_id } => json!({ "trade": short_uuid(*trade_id) }),
        FinancialEffect::DvPCancel { trade_id } => json!({ "trade": short_uuid(*trade_id) }),
    }
}

fn compact_inventory_effect(effect: &InventoryEffect) -> Value {
    match effect {
        InventoryEffect::AddInventory { owner, good_id, quantity, unit_cost } => json!({
            "owner": short_agent(*owner),
            "good": short_good(*good_id),
            "qty": round_qty(*quantity),
            "unit_cost": round_amount(*unit_cost),
        }),
        InventoryEffect::RemoveInventory { owner, good_id, quantity } => json!({
            "owner": short_agent(*owner),
            "good": short_good(*good_id),
            "qty": round_qty(*quantity),
        }),
    }
}

fn compact_market_effect(effect: &MarketEffect) -> Value {
    match effect {
        MarketEffect::PlaceOrderInBook { market_id, order } => json!({
            "market": market_id.to_string(),
            "agent": short_agent(order.agent_id),
            "side": format!("{:?}", order.side),
            "qty": round_qty(order.quantity),
            "price": order.price.map(round_money),
            "order_type": format!("{:?}", order.order_type),
        }),
        MarketEffect::ExecuteTrade(trade) => json!({
            "trade": short_uuid(trade.trade_id),
            "market": trade.market_id.to_string(),
            "buyer": short_agent(trade.buyer),
            "seller": short_agent(trade.seller),
            "qty": round_qty(trade.quantity),
            "price": round_money(trade.price),
        }),
        MarketEffect::UpdatePrice { market_id, new_price } => json!({
            "market": market_id.to_string(),
            "price": round_amount(*new_price),
        }),
        MarketEffect::ClearMarket { market_id } => json!({ "market": market_id.to_string() }),
        MarketEffect::UpdateLabourMarket { market_id, update } => match update {
            LabourMarketUpdate::AddApplication(app) => json!({
                "market": short_labour(*market_id),
                "application": short_uuid(app.application_id),
                "consumer": short_agent(app.consumer_id),
                "reservation_wage": round_amount(app.reservation_wage),
                "hours": round_qty(app.hours_desired),
            }),
            LabourMarketUpdate::AddOffer(offer) => json!({
                "market": short_labour(*market_id),
                "offer": short_uuid(offer.offer_id),
                "firm": short_agent(offer.firm_id),
                "wage": round_amount(offer.wage_rate),
                "hours": round_qty(offer.hours_required),
                "quantity": offer.quantity,
            }),
        },
        MarketEffect::ClearLabourMarketOrders { market_id, filled_applications } => json!({
            "market": short_labour(*market_id),
            "cleared": filled_applications.iter().map(|id| short_uuid(*id)).collect::<Vec<_>>(),
        }),
        MarketEffect::OpenDebtAuction { auction } => json!({
            "auction": short_uuid(auction.auction_id),
            "instrument": short_instrument(auction.instrument_id),
            "qty_offered": auction.quantity_offered,
            "status": format!("{:?}", auction.status),
        }),
        MarketEffect::SubmitAuctionBid { auction_id, bid } => json!({
            "auction": short_uuid(*auction_id),
            "agent": short_agent(bid.agent_id),
            "qty": bid.quantity,
            "price": round_money(bid.price),
        }),
        MarketEffect::CloseDebtAuction { auction_id } => json!({ "auction": short_uuid(*auction_id) }),
        MarketEffect::PostOvernightFundingQuote(quote) => json!({
            "venue": format!("{:?}", quote.venue),
            "agent": short_agent(quote.agent),
            "side": format!("{:?}", quote.side),
            "notional": round_amount(quote.notional),
            "rate_bps": quote.limit_rate_bps.to_string(),
        }),
    }
}

fn compact_agent_effect(effect: &AgentEffect) -> Value {
    match effect {
        AgentEffect::EstablishEmployment { firm_id, consumer_id, contract } => json!({
            "firm": short_agent(*firm_id),
            "consumer": short_agent(*consumer_id),
            "wage": round_amount(contract.wage_rate),
            "hours": round_qty(contract.hours),
        }),
        AgentEffect::TerminateEmployment { firm_id, consumer_id } => json!({
            "firm": short_agent(*firm_id),
            "consumer": short_agent(*consumer_id),
        }),
        AgentEffect::UpdateIncome { id, new_income } => json!({
            "agent": short_agent(*id),
            "income": round_amount(*new_income),
        }),
        AgentEffect::RecordDividendIncome { recipient, amount } => json!({
            "agent": short_agent(*recipient),
            "amount": round_amount(*amount),
        }),
        AgentEffect::Produce { firm, good_id, amount } => json!({
            "firm": short_agent(*firm),
            "good": short_good(*good_id),
            "output": round_qty(*amount),
        }),
        AgentEffect::UpdateRevenue { id, revenue } => json!({
            "agent": short_agent(*id),
            "revenue": round_amount(*revenue),
        }),
        AgentEffect::RecordCogs { id, amount } => json!({
            "agent": short_agent(*id),
            "cogs": round_amount(*amount),
        }),
        AgentEffect::RecordOpEx { id, amount } => json!({
            "agent": short_agent(*id),
            "opex": round_amount(*amount),
        }),
    }
}

fn compact_monetary_effect(effect: &MonetaryEffect) -> Value {
    match effect {
        MonetaryEffect::OpenMarketOperation { operation_type, amount } => json!({
            "operation": format!("{:?}", operation_type),
            "amount": round_amount(*amount),
        }),
        MonetaryEffect::SetPolicyRate { rate_bps } => json!({ "rate_bps": rate_bps.to_string() }),
        MonetaryEffect::SetReserveRequirement { ratio } => json!({ "ratio": round_to(*ratio, 4) }),
        MonetaryEffect::ProvideLiquidityAssistance { bank_id, amount, collateral } => json!({
            "bank": short_agent(*bank_id),
            "amount": round_amount(*amount),
            "collateral": collateral
                .as_ref()
                .map(|v| v.iter().map(|id| short_instrument(*id)).collect::<Vec<_>>()),
        }),
    }
}

fn compact_credit_effect(effect: &CreditEffect) -> Value {
    match effect {
        CreditEffect::RegisterFacility { facility } => compact_credit_facility(facility),
        CreditEffect::RegisterLoan { loan, is_consumer, purpose } => compact_credit_loan(loan, *is_consumer, purpose),
        CreditEffect::UpdateFacilityUtilization { facility_id, drawn_amount, available_amount } => json!({
            "facility": short_uuid(*facility_id),
            "drawn": round_money(*drawn_amount),
            "available": round_money(*available_amount),
        }),
        CreditEffect::RecordLoanApplication { bank_id, application } => json!({
            "bank": short_agent(*bank_id),
            "application": compact_loan_application(application),
        }),
        CreditEffect::ProcessLoanPayment { loan_id, principal_paid, interest_paid, fees_paid, payment_date } => json!({
            "loan": short_uuid(*loan_id),
            "principal": round_money(*principal_paid),
            "interest": round_money(*interest_paid),
            "fees": round_money(*fees_paid),
            "date": payment_date.to_string(),
        }),
        CreditEffect::AccrueLoanInterest { loan_id, interest_amount, accrual_date } => json!({
            "loan": short_uuid(*loan_id),
            "interest": round_money(*interest_amount),
            "date": accrual_date.to_string(),
        }),
        CreditEffect::RecordImpairment { loan_id, stage, provision } => json!({
            "loan": short_uuid(*loan_id),
            "stage": format!("{:?}", stage),
            "provision": round_money(*provision),
        }),
        CreditEffect::RecordLien { lien, loan_id } => json!({
            "loan": short_uuid(*loan_id),
            "lien": compact_lien(lien),
        }),
        CreditEffect::UpdateLienStatus { lien_id, new_status } => json!({
            "lien": short_uuid(*lien_id),
            "status": format!("{:?}", new_status),
        }),
    }
}

fn compact_credit_facility(facility: &CreditFacility) -> Value {
    json!({
        "facility": short_uuid(facility.details.facility_id),
        "lender": short_agent(facility.details.lender),
        "borrower": short_agent(facility.details.borrower),
        "commitment": round_money(facility.details.commitment_amount),
        "available": round_money(facility.details.available_amount),
        "drawn": round_money(facility.details.drawn_amount),
        "status": format!("{:?}", facility.status),
    })
}

fn compact_credit_loan(loan: &Loan, is_consumer: bool, purpose: &LoanPurpose) -> Value {
    json!({
        "loan": short_uuid(loan.details.loan_id),
        "lender": short_agent(loan.details.lender),
        "borrower": short_agent(loan.details.borrower),
        "principal": round_money(loan.details.principal),
        "outstanding": round_money(loan.details.outstanding_principal),
        "type": format!("{:?}", loan.details.loan_type),
        "consumer": is_consumer,
        "purpose": format!("{:?}", purpose),
        "status": format!("{:?}", loan.status),
    })
}

fn compact_loan_application(app: &LoanApplication) -> Value {
    json!({
        "application": short_uuid(app.application_id),
        "borrower": short_agent(app.borrower_id),
        "amount": round_amount(app.requested_amount),
        "purpose": format!("{:?}", app.purpose),
        "status": format!("{:?}", app.status),
    })
}

fn compact_lien(lien: &Lien) -> Value {
    json!({
        "lien": short_uuid(lien.lien_id),
        "collateral": compact_collateral(&lien.collateral_type),
        "priority": lien.priority,
        "status": format!("{:?}", lien.status),
    })
}

fn compact_collateral(collateral: &CollateralType) -> Value {
    match collateral {
        CollateralType::Securities { instrument_id, quantity, haircut } => json!({
            "type": "securities",
            "instrument": short_instrument(*instrument_id),
            "qty": round_qty(*quantity),
            "haircut": round_amount(*haircut),
        }),
        CollateralType::RealEstate { property_id, valuation, ltv_limit } => json!({
            "type": "real_estate",
            "property": short_uuid(*property_id),
            "valuation": round_money(*valuation),
            "ltv_limit": round_to(*ltv_limit, 4),
        }),
        CollateralType::Receivables { debtor, amount } => json!({
            "type": "receivables",
            "debtor": short_agent(*debtor),
            "amount": round_money(*amount),
        }),
        CollateralType::Cash { amount } => json!({
            "type": "cash",
            "amount": round_money(*amount),
        }),
        CollateralType::Other { description, value } => json!({
            "type": "other",
            "description": description,
            "value": round_money(*value),
        }),
    }
}

fn compact_payment_stage(stage: &PaymentStage) -> Value {
    match stage {
        PaymentStage::Initiated => json!("initiated"),
        PaymentStage::Queued { position } => json!({ "queued": position }),
        PaymentStage::Processing => json!("processing"),
        PaymentStage::Settled => json!("settled"),
        PaymentStage::Failed { reason } => json!({ "failed": reason }),
        PaymentStage::Rejected { reason } => json!({ "rejected": reason }),
    }
}

fn compact_change_reason(reason: &ChangeReason) -> Value {
    match reason {
        ChangeReason::PaymentReceived { from } => json!({ "payment_received": short_agent(*from) }),
        ChangeReason::PaymentSent { to } => json!({ "payment_sent": short_agent(*to) }),
        ChangeReason::InterestAccrual => json!("interest_accrual"),
        ChangeReason::TradeSettlement { trade_id } => json!({ "trade": short_uuid(*trade_id) }),
        ChangeReason::LoanDisbursement { loan_id } => json!({ "loan": short_instrument(*loan_id) }),
        ChangeReason::WagePayment { employer } => json!({ "wage_from": short_agent(*employer) }),
        ChangeReason::DepositCreation => json!("deposit_creation"),
        ChangeReason::TaxPayment { government } => json!({ "tax_to": short_agent(*government) }),
    }
}

fn compact_banking_event_type(event: &BankingEventType) -> Value {
    match event {
        BankingEventType::ReserveMovement { to_bank, reason } => json!({
            "type": "reserve_movement",
            "to": to_bank.map(short_agent),
            "reason": reason,
        }),
        BankingEventType::DepositCreation { for_agent, account_id } => json!({
            "type": "deposit_creation",
            "agent": short_agent(*for_agent),
            "account": account_id,
        }),
        BankingEventType::LiquidityInjection { from_central_bank } => json!({
            "type": "liquidity_injection",
            "from_cb": from_central_bank,
        }),
        BankingEventType::InterbankTransfer { to_bank, reference } => json!({
            "type": "interbank_transfer",
            "to": short_agent(*to_bank),
            "reference": reference,
        }),
        BankingEventType::CreditCreation { to_agent, instrument_id } => json!({
            "type": "credit_creation",
            "agent": short_agent(*to_agent),
            "instrument": short_instrument(*instrument_id),
        }),
    }
}

fn compact_market_activity(activity: &MarketActivityType, agent: AgentId, instrument: InstrumentId) -> Value {
    match activity {
        MarketActivityType::OrderPlaced { order_id, side, quantity, price } => json!({
            "event": "order_placed",
            "order": short_uuid(*order_id),
            "side": format!("{:?}", side),
            "qty": round_qty(*quantity),
            "price": round_money(*price),
            "agent": short_agent(agent),
            "instrument": short_instrument(instrument),
        }),
        MarketActivityType::OrderCancelled { order_id } => json!({
            "event": "order_cancelled",
            "order": short_uuid(*order_id),
            "agent": short_agent(agent),
            "instrument": short_instrument(instrument),
        }),
        MarketActivityType::OrderPartiallyFilled { order_id, filled_quantity, remaining_quantity } => json!({
            "event": "order_partial_fill",
            "order": short_uuid(*order_id),
            "filled": round_qty(*filled_quantity),
            "remaining": round_qty(*remaining_quantity),
            "agent": short_agent(agent),
            "instrument": short_instrument(instrument),
        }),
        MarketActivityType::PriceUpdate { new_bid, new_ask } => json!({
            "event": "price_update",
            "bid": new_bid.as_ref().map(|b| round_money(*b)),
            "ask": new_ask.as_ref().map(|a| round_money(*a)),
            "instrument": short_instrument(instrument),
        }),
    }
}

fn compact_money_flow_event(evt: &MoneyFlowChainEvent) -> Value {
    match &evt.event_type {
        ChainEventType::ChainStart { initiator, purpose } => json!({
            "stage": "start",
            "agent": short_agent(*initiator),
            "purpose": purpose,
            "amount": round_money(evt.amount),
            "sequence": evt.sequence_number,
        }),
        ChainEventType::ChainLink { from, to, step } => json!({
            "stage": "link",
            "from": short_agent(*from),
            "to": short_agent(*to),
            "step": step,
            "amount": round_money(evt.amount),
            "sequence": evt.sequence_number,
        }),
        ChainEventType::ChainEnd { final_recipient, outcome } => json!({
            "stage": "end",
            "recipient": short_agent(*final_recipient),
            "outcome": outcome,
            "amount": round_money(evt.amount),
            "sequence": evt.sequence_number,
        }),
    }
}

fn compact_transaction_payload(tx: &Transaction) -> Value {
    json!({
        "transaction": short_uuid(tx.id),
        "from": short_agent(tx.from_agent),
        "to": short_agent(tx.to_agent),
        "amount": round_amount(tx.amount),
        "kind": tx.transaction_type,
        "date": tx.timestamp.to_string(),
        "instrument": tx.instrument_id.map(short_instrument),
    })
}
