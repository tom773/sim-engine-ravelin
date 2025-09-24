use chrono::{DateTime, Utc};
use metrics::histogram;
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use sim_core::prelude::*;
use sim_core::types::events::{SimEvent, TickEventSummary};
use sim_core::types::instrument::archetypes::CashType;
use sim_core::types::instrument::{Instrument, InstrumentRuntime, RealAssetState};
use sim_core::types::markets::market::{Exchange, FinancialProduct, MarketGeneric, MarketType};
use sim_core::types::markets::orderbook::MarketDepthSummary;
use sim_core::types::system::{
    balance_sheet::{IncomeStatement, Position},
    financial_system::FinancialSystem,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tracing::instrument;
use uuid::Uuid;

use engine_v3::SimulationEngine;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDigest {
    pub tick: u32,
    pub sim_time: SimTimeDigest,
    pub status: StatusDigest,
    pub agents: AgentsDigest,
    pub markets: MarketsDigest,
    pub risk: RiskDigest,
    pub highlights: Vec<DigestEvent>,
    pub timings: DigestTimings,
}

impl StateDigest {
    pub fn bootstrap() -> Self {
        Self {
            tick: 0,
            sim_time: SimTimeDigest { current_date: "bootstrap".into(), session: "bootstrap".into() },
            status: StatusDigest::default(),
            agents: AgentsDigest::default(),
            markets: MarketsDigest::default(),
            risk: RiskDigest::default(),
            highlights: vec![DigestEvent::info("mirror", "mirror cache initialised")],
            timings: DigestTimings::bootstrap(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SimTimeDigest {
    pub current_date: String,
    pub session: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestTimings {
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub generated_at: DateTime<Utc>,
    pub build_duration_ms: f64,
}

impl DigestTimings {
    fn bootstrap() -> Self {
        Self { generated_at: Utc::now(), build_duration_ms: 0.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatusDigest {
    pub tick: u32,
    pub current_date: String,
    pub total_iterations: u32,
    pub agent_counts: AgentCountsDigest,
    pub macro_overview: MacroDigest,
    pub monetary: MonetaryDigest,
    pub money_supply: MoneySupplyDigest,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentCountsDigest {
    pub banks: usize,
    pub firms: usize,
    pub consumers: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacroDigest {
    pub nominal_gdp_proxy: f64,
    pub consumer_spending_daily: f64,
    pub unemployment_rate: f64,
    pub inflation_rate: f64,
    pub cpi: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MonetaryDigest {
    pub policy_rate_bps: f64,
    pub reserve_requirement: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoneySupplyDigest {
    pub m0: f64,
    pub m1: f64,
    pub m2: f64,
    pub bank_reserves: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsDigest {
    pub leaderboard: Vec<AgentBalanceDigest>,
    pub liquidity_leaderboard: Vec<AgentBalanceDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentBalanceDigest {
    pub agent_id: Uuid,
    pub agent_type: String,
    pub name: String,
    pub total_assets: f64,
    pub total_liabilities: f64,
    pub net_worth: f64,
    pub liquidity: f64,
    pub balance_sheet: BalanceSheetDigest,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BalanceSheetDigest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<BalanceEntryDigest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub liabilities: Vec<BalanceEntryDigest>,
    pub income_statement: IncomeStatementDigest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BalanceEntrySource {
    BalanceSheet,
    Custody,
}

impl Default for BalanceEntrySource {
    fn default() -> Self {
        BalanceEntrySource::BalanceSheet
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BalanceEntryDigest {
    pub instrument_id: String,
    pub instrument_type: String,
    pub label: String,
    pub quantity: f64,
    pub mark_to_market_value: f64,
    pub book_value: f64,
    pub cost_basis: f64,
    pub source: BalanceEntrySource,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IncomeStatementDigest {
    pub revenue: f64,
    pub cost_of_goods_sold: f64,
    pub operating_expenses: f64,
    pub interest_income: f64,
    pub interest_expense: f64,
    pub net_income: f64,
}

impl From<&IncomeStatement> for IncomeStatementDigest {
    fn from(src: &IncomeStatement) -> Self {
        Self {
            revenue: money_to_f64(&src.revenue),
            cost_of_goods_sold: money_to_f64(&src.cost_of_goods_sold),
            operating_expenses: money_to_f64(&src.operating_expenses),
            interest_income: money_to_f64(&src.interest_income),
            interest_expense: money_to_f64(&src.interest_expense),
            net_income: money_to_f64(&src.net_income),
        }
    }
}

fn money_to_f64(value: &Money) -> f64 {
    let raw = value.to_f64();
    if raw.is_finite() { raw } else { 0.0 }
}

fn sanitize_f64(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketsDigest {
    pub snapshots: Vec<MarketDigest>,
    pub most_active: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketDigest {
    pub market_id: String,
    pub label: String,
    pub kind: MarketKindDigest,
    pub last_price: Option<f64>,
    pub mid_price: Option<f64>,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub spread: Option<f64>,
    pub volume: f64,
    pub turnover: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<DepthDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_mid_bps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_last_bps: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MarketKindDigest {
    #[default]
    Financial,
    Goods,
    Labour,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DepthDigest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bids: Vec<DepthLevel>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub asks: Vec<DepthLevel>,
    pub bid_size_at_best: f64,
    pub ask_size_at_best: f64,
    pub total_bid_levels: usize,
    pub total_ask_levels: usize,
    pub total_bid_volume: f64,
    pub total_ask_volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DepthLevel {
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RiskDigest {
    pub total_assets: f64,
    pub total_liabilities: f64,
    pub system_liquidity: f64,
    pub total_loans: f64,
    pub total_deposits: f64,
    pub leverage: f64,
    pub capital_ratio: f64,
    pub liquidity_ratio: f64,
    pub loan_to_deposit_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestEvent {
    pub context: String,
    pub message: String,
    pub level: DigestEventLevel,
}

impl DigestEvent {
    pub fn info(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self { context: context.into(), message: message.into(), level: DigestEventLevel::Info }
    }

    pub fn warning(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self { context: context.into(), message: message.into(), level: DigestEventLevel::Warning }
    }

    pub fn error(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self { context: context.into(), message: message.into(), level: DigestEventLevel::Error }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DigestEventLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub digest: Arc<StateDigest>,
    pub delta: Option<StateDelta>,
}

impl StateSnapshot {
    pub fn from_digest(digest: StateDigest) -> Self {
        Self { digest: Arc::new(digest), delta: None }
    }
}

impl Serialize for StateSnapshot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("StateSnapshot", 2)?;
        state.serialize_field("digest", self.digest.as_ref())?;
        if let Some(delta) = &self.delta {
            state.serialize_field("delta", delta)?;
        }
        state.end()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDelta {
    pub tick: u32,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub generated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub agent_changes: Vec<AgentBalanceDelta>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub market_changes: Vec<MarketDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_change: Option<RiskDelta>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub new_highlights: Vec<DigestEvent>,
}

impl StateDelta {
    pub fn between(previous: Option<&StateDigest>, next: &StateDigest) -> Option<Self> {
        let prev = match previous {
            Some(prev) if prev.tick <= next.tick => prev,
            _ => return None,
        };

        let agent_changes = diff_agents(&prev.agents, &next.agents);
        let market_changes = diff_markets(&prev.markets, &next.markets);
        let risk_change = diff_risk(&prev.risk, &next.risk);
        let new_highlights = diff_highlights(&prev.highlights, &next.highlights);

        if agent_changes.is_empty() && market_changes.is_empty() && risk_change.is_none() && new_highlights.is_empty() {
            return None;
        }

        Some(Self {
            tick: next.tick,
            generated_at: next.timings.generated_at,
            agent_changes,
            market_changes,
            risk_change,
            new_highlights,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBalanceDelta {
    pub agent_id: Uuid,
    pub name: String,
    pub net_worth_delta: f64,
    pub liquidity_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDelta {
    pub market_id: String,
    pub mid_price_delta: Option<f64>,
    pub spread_delta: Option<f64>,
    pub volume_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskDelta {
    pub total_assets_delta: f64,
    pub total_liabilities_delta: f64,
    pub liquidity_ratio_delta: f64,
    pub loan_to_deposit_delta: f64,
}

const AGENT_DELTA_EPSILON: f64 = 1.0;
const MARKET_VOLUME_EPSILON: f64 = 1.0;
const MARKET_PRICE_EPSILON: f64 = 1e-6;
const RATIO_EPSILON: f64 = 1e-4;

fn diff_agents(prev: &AgentsDigest, next: &AgentsDigest) -> Vec<AgentBalanceDelta> {
    let mut prev_map: HashMap<Uuid, &AgentBalanceDigest> = HashMap::new();
    for entry in &prev.leaderboard {
        prev_map.insert(entry.agent_id, entry);
    }

    next.leaderboard
        .iter()
        .filter_map(|entry| {
            let prev_entry = prev_map.get(&entry.agent_id);
            let prev_net = prev_entry.map(|p| p.net_worth).unwrap_or(0.0);
            let prev_liq = prev_entry.map(|p| p.liquidity).unwrap_or(0.0);
            let net_delta = entry.net_worth - prev_net;
            let liq_delta = entry.liquidity - prev_liq;
            if net_delta.abs() < AGENT_DELTA_EPSILON && liq_delta.abs() < AGENT_DELTA_EPSILON {
                None
            } else {
                Some(AgentBalanceDelta {
                    agent_id: entry.agent_id,
                    name: entry.name.clone(),
                    net_worth_delta: net_delta,
                    liquidity_delta: liq_delta,
                })
            }
        })
        .collect()
}

fn diff_markets(prev: &MarketsDigest, next: &MarketsDigest) -> Vec<MarketDelta> {
    let prev_map: HashMap<&String, &MarketDigest> = prev.snapshots.iter().map(|m| (&m.market_id, m)).collect();

    next.snapshots
        .iter()
        .filter_map(|market| {
            let prev_market = prev_map.get(&market.market_id);
            let prev_volume = prev_market.map(|m| m.volume).unwrap_or(0.0);
            let volume_delta = market.volume - prev_volume;

            let mid_delta = match (prev_market.and_then(|m| m.mid_price), market.mid_price) {
                (Some(prev_mid), Some(next_mid)) if (next_mid - prev_mid).abs() >= MARKET_PRICE_EPSILON => {
                    Some(next_mid - prev_mid)
                }
                (None, Some(next_mid)) => Some(next_mid),
                _ => None,
            };

            let spread_delta = match (prev_market.and_then(|m| m.spread), market.spread) {
                (Some(prev_spread), Some(next_spread)) if (next_spread - prev_spread).abs() >= MARKET_PRICE_EPSILON => {
                    Some(next_spread - prev_spread)
                }
                (None, Some(next_spread)) => Some(next_spread),
                _ => None,
            };

            if volume_delta.abs() < MARKET_VOLUME_EPSILON && mid_delta.is_none() && spread_delta.is_none() {
                None
            } else {
                Some(MarketDelta {
                    market_id: market.market_id.clone(),
                    mid_price_delta: mid_delta,
                    spread_delta,
                    volume_delta,
                })
            }
        })
        .collect()
}

fn diff_risk(prev: &RiskDigest, next: &RiskDigest) -> Option<RiskDelta> {
    let total_assets_delta = next.total_assets - prev.total_assets;
    let total_liabilities_delta = next.total_liabilities - prev.total_liabilities;
    let liquidity_ratio_delta = next.liquidity_ratio - prev.liquidity_ratio;
    let loan_to_deposit_delta = next.loan_to_deposit_ratio - prev.loan_to_deposit_ratio;

    if total_assets_delta.abs() < AGENT_DELTA_EPSILON
        && total_liabilities_delta.abs() < AGENT_DELTA_EPSILON
        && liquidity_ratio_delta.abs() < RATIO_EPSILON
        && loan_to_deposit_delta.abs() < RATIO_EPSILON
    {
        None
    } else {
        Some(RiskDelta { total_assets_delta, total_liabilities_delta, liquidity_ratio_delta, loan_to_deposit_delta })
    }
}

fn diff_highlights(prev: &[DigestEvent], next: &[DigestEvent]) -> Vec<DigestEvent> {
    let prev_set: HashSet<(&str, &str)> = prev.iter().map(|evt| (evt.context.as_str(), evt.message.as_str())).collect();
    next.iter().filter(|evt| !prev_set.contains(&(evt.context.as_str(), evt.message.as_str()))).cloned().collect()
}

#[instrument(skip(engine, events))]
pub fn build_state_digest(engine: &SimulationEngine, events: &[SimEvent]) -> StateDigest {
    let build_start = Instant::now();
    let status = compute_status(engine);
    let agents = compute_agents(engine, 10);
    let markets = compute_markets(&engine.state, 12);
    let risk = compute_risk(&engine.state.financial_system);
    let highlights = build_highlights(&engine.state, &markets, events);
    let timings =
        DigestTimings { generated_at: Utc::now(), build_duration_ms: build_start.elapsed().as_secs_f64() * 1_000.0 };

    histogram!("mirror.digest.build_ms", timings.build_duration_ms);
    histogram!("mirror.digest.snapshot.size", (agents.leaderboard.len() + markets.snapshots.len()) as f64);

    StateDigest {
        tick: engine.state.ticknum,
        sim_time: SimTimeDigest {
            current_date: engine.state.current_date.format("%Y-%m-%d").to_string(),
            session: format_session(engine.state.current_session).to_string(),
        },
        status,
        agents,
        markets,
        risk,
        highlights,
        timings,
    }
}

fn compute_status(engine: &SimulationEngine) -> StatusDigest {
    let state = &engine.state;
    let macro_stats = state.macro_stats();
    let policy_rate = state.financial_system.central_bank.policy_rate_bps;
    let reserve_requirement = state.financial_system.central_bank.reserve_requirement;

    StatusDigest {
        tick: state.ticknum,
        current_date: state.current_date.format("%Y-%m-%d").to_string(),
        total_iterations: state.config.iterations,
        agent_counts: AgentCountsDigest {
            banks: state.agents.banks.len(),
            firms: state.agents.firms.len(),
            consumers: state.agents.consumers.len(),
            total: state.agents.banks.len() + state.agents.firms.len() + state.agents.consumers.len(),
        },
        macro_overview: MacroDigest {
            nominal_gdp_proxy: macro_stats.nominal_gdp_proxy,
            consumer_spending_daily: macro_stats.consumer_spending_daily,
            unemployment_rate: macro_stats.unemployment_rate,
            inflation_rate: macro_stats.inflation_rate,
            cpi: macro_stats.cpi,
        },
        monetary: MonetaryDigest { policy_rate_bps: policy_rate.to_f64().unwrap_or_default(), reserve_requirement },
        money_supply: MoneySupplyDigest {
            m0: macro_stats.m0,
            m1: macro_stats.m1,
            m2: macro_stats.m2,
            bank_reserves: macro_stats.bank_reserves,
        },
    }
}

fn compute_agents(engine: &SimulationEngine, limit: usize) -> AgentsDigest {
    let state = &engine.state;
    let financial_system = &state.financial_system;

    let estimated_agents = state.agents.banks.len() + state.agents.firms.len() + state.agents.consumers.len() + 2;
    let mut entries: Vec<AgentBalanceDigest> = Vec::with_capacity(estimated_agents);
    let mut seen = HashSet::new();

    for agent_id in state.agents.all_agent_ids() {
        if seen.insert(agent_id) {
            entries.push(build_agent_entry(engine, financial_system, &agent_id));
        }
    }

    let government_id = state.financial_system.government.id;
    if seen.insert(government_id) {
        entries.push(build_agent_entry(engine, financial_system, &government_id));
    }

    let central_bank_id = state.financial_system.central_bank.id;
    if seen.insert(central_bank_id) {
        entries.push(build_agent_entry(engine, financial_system, &central_bank_id));
    }

    let mut leaderboard = entries.clone();
    leaderboard.sort_by(|a, b| b.net_worth.partial_cmp(&a.net_worth).unwrap_or(std::cmp::Ordering::Equal));
    leaderboard.truncate(limit);

    let mut liquidity_leaderboard = entries;
    liquidity_leaderboard.sort_by(|a, b| b.liquidity.partial_cmp(&a.liquidity).unwrap_or(std::cmp::Ordering::Equal));
    liquidity_leaderboard.truncate(limit);

    AgentsDigest { leaderboard, liquidity_leaderboard }
}

fn build_agent_entry(
    engine: &SimulationEngine, financial_system: &FinancialSystem, agent_id: &AgentId,
) -> AgentBalanceDigest {
    let (agent_type, name) = engine.get_agent_info(agent_id);
    let total_assets = financial_system.get_total_assets(agent_id);
    let total_liabilities = financial_system.get_total_liabilities(agent_id);
    let liquidity = financial_system.get_liquid_assets(agent_id);
    let net_worth = total_assets - total_liabilities;
    let balance_sheet = build_balance_sheet(financial_system, agent_id);

    AgentBalanceDigest {
        agent_id: agent_id.0,
        agent_type,
        name: name.unwrap_or_else(|| "N/A".into()),
        total_assets,
        total_liabilities,
        net_worth,
        liquidity,
        balance_sheet,
    }
}

fn build_balance_sheet(financial_system: &FinancialSystem, agent_id: &AgentId) -> BalanceSheetDigest {
    let mut assets: Vec<BalanceEntryDigest> = Vec::new();
    let mut liabilities: Vec<BalanceEntryDigest> = Vec::new();
    let mut income_statement = IncomeStatementDigest::default();

    if let Some(sheet) = financial_system.balance_sheets.get(agent_id) {
        income_statement = IncomeStatementDigest::from(&sheet.income_statement);

        for (instrument_id, position) in &sheet.assets {
            assets.push(balance_entry_from_position(
                financial_system,
                instrument_id,
                position,
                BalanceEntrySource::BalanceSheet,
            ));
        }

        for (instrument_id, position) in &sheet.liabilities {
            liabilities.push(balance_entry_from_position(
                financial_system,
                instrument_id,
                position,
                BalanceEntrySource::BalanceSheet,
            ));
        }
    }

    for (instrument_id, quantity) in financial_system.clearing_house.csd.get_all_positions(agent_id) {
        if quantity <= 1e-9 {
            continue;
        }
        assets.push(balance_entry_from_custody(financial_system, &instrument_id, quantity));
    }

    assets.sort_by(|a, b| {
        b.mark_to_market_value.partial_cmp(&a.mark_to_market_value).unwrap_or(std::cmp::Ordering::Equal)
    });
    liabilities.sort_by(|a, b| b.book_value.partial_cmp(&a.book_value).unwrap_or(std::cmp::Ordering::Equal));

    BalanceSheetDigest { assets, liabilities, income_statement }
}

fn balance_entry_from_position(
    financial_system: &FinancialSystem, instrument_id: &InstrumentId, position: &Position, source: BalanceEntrySource,
) -> BalanceEntryDigest {
    let (label, instrument_type) = instrument_metadata(financial_system, instrument_id);
    let unit_price = resolve_unit_price(financial_system, instrument_id, position.book_value_per_unit);
    let mark_to_market_value = unit_price.to_f64() * position.quantity;
    let book_value = position.book_value_per_unit.to_f64() * position.quantity;
    let cost_basis = position.cost_basis_per_unit.to_f64() * position.quantity;

    BalanceEntryDigest {
        instrument_id: instrument_id.to_string(),
        instrument_type,
        label,
        quantity: position.quantity,
        mark_to_market_value: sanitize_f64(mark_to_market_value),
        book_value: sanitize_f64(book_value),
        cost_basis: sanitize_f64(cost_basis),
        source,
    }
}

fn balance_entry_from_custody(
    financial_system: &FinancialSystem, instrument_id: &InstrumentId, quantity: f64,
) -> BalanceEntryDigest {
    let (label, instrument_type) = instrument_metadata(financial_system, instrument_id);
    let fallback = financial_system
        .instruments
        .instruments
        .get(instrument_id)
        .and_then(|inst| inst.unit_par_value())
        .unwrap_or(Money::ONE);
    let unit_price = resolve_unit_price(financial_system, instrument_id, fallback);
    let mtm = unit_price.to_f64() * quantity;
    let fallback_value = fallback.to_f64() * quantity;

    BalanceEntryDigest {
        instrument_id: instrument_id.to_string(),
        instrument_type,
        label,
        quantity,
        mark_to_market_value: sanitize_f64(mtm),
        book_value: sanitize_f64(fallback_value),
        cost_basis: sanitize_f64(fallback_value),
        source: BalanceEntrySource::Custody,
    }
}

fn resolve_unit_price(financial_system: &FinancialSystem, instrument_id: &InstrumentId, fallback: Money) -> Money {
    financial_system
        .get_market_price(instrument_id)
        .or_else(|| financial_system.instruments.instruments.get(instrument_id).and_then(|inst| inst.unit_par_value()))
        .unwrap_or(fallback)
}

fn instrument_metadata(financial_system: &FinancialSystem, instrument_id: &InstrumentId) -> (String, String) {
    financial_system
        .instruments
        .instruments
        .get(instrument_id)
        .map(|inst| (inst.label().to_string(), inst.type_as_string().to_string()))
        .unwrap_or_else(|| ("Unknown Instrument".into(), "unknown".into()))
}

fn compute_markets(state: &SimState, limit: usize) -> MarketsDigest {
    let mut snapshots: Vec<MarketDigest> = Vec::new();

    for (symbol, market) in &state.financial_system.exchange.markets {
        match market {
            MarketType::Financial(fin_market) => {
                let inst_id = &fin_market.key;
                let view = state.market_view(symbol).unwrap_or_default();
                let (yield_mid, yield_last) = calculate_yields(inst_id, fin_market);
                let depth = depth_from_summary(fin_market.book.depth_summary(), 3);
                let label = state
                    .financial_system
                    .instruments
                    .instruments
                    .get(inst_id)
                    .map(|i| i.type_as_string().to_string())
                    .unwrap_or_else(|| "Financial Market".into());

                snapshots.push(MarketDigest {
                    market_id: symbol.to_string(),
                    label,
                    kind: MarketKindDigest::Financial,
                    last_price: view.last,
                    mid_price: view.mid,
                    best_bid: fin_market.book.best_bid().map(|m| m.to_f64()),
                    best_ask: fin_market.book.best_ask().map(|m| m.to_f64()),
                    spread: fin_market.book.spread().map(|m| m.to_f64()),
                    volume: view.volume,
                    turnover: view.turnover,
                    depth,
                    yield_mid_bps: yield_mid,
                    yield_last_bps: yield_last,
                });
            }
            MarketType::Goods(goods_market) => {
                let view = state.market_view(symbol).unwrap_or_default();
                let depth = depth_from_summary(goods_market.book.depth_summary(), 3);
                let label = state
                    .financial_system
                    .goods
                    .goods
                    .get(&goods_market.key)
                    .map(|g| g.name.clone())
                    .unwrap_or_else(|| "Goods Market".into());

                snapshots.push(MarketDigest {
                    market_id: symbol.to_string(),
                    label,
                    kind: MarketKindDigest::Goods,
                    last_price: view.last,
                    mid_price: view.mid,
                    best_bid: goods_market.book.best_bid().map(|m| m.to_f64()),
                    best_ask: goods_market.book.best_ask().map(|m| m.to_f64()),
                    spread: goods_market.book.spread().map(|m| m.to_f64()),
                    volume: view.volume,
                    turnover: view.turnover,
                    depth,
                    yield_mid_bps: None,
                    yield_last_bps: None,
                });
            }
            MarketType::Labour(_labour_market) => {
                snapshots.push(MarketDigest {
                    market_id: symbol.to_string(),
                    label: "Labour Market".into(),
                    kind: MarketKindDigest::Labour,
                    last_price: None,
                    mid_price: None,
                    best_bid: None,
                    best_ask: None,
                    spread: None,
                    volume: 0.0,
                    turnover: 0.0,
                    depth: None,
                    yield_mid_bps: None,
                    yield_last_bps: None,
                });
            }
        }
    }

    snapshots.sort_by(|a, b| b.volume.partial_cmp(&a.volume).unwrap_or(std::cmp::Ordering::Equal));
    let most_active: Vec<String> = snapshots.iter().take(5).map(|m| m.market_id.clone()).collect();
    snapshots.truncate(limit);

    MarketsDigest { snapshots, most_active }
}

fn calculate_yields(inst_id: &InstrumentId, market: &MarketGeneric<FinancialProduct>) -> (Option<f64>, Option<f64>) {
    let mid = market
        .book
        .mid_price()
        .and_then(|price| market.pricer.yield_from_price(inst_id, price))
        .and_then(|r| r.to_f64());
    let last = market
        .book
        .last_price
        .and_then(|price| market.pricer.yield_from_price(inst_id, price))
        .and_then(|r| r.to_f64());
    (mid, last)
}

fn depth_from_summary(summary: MarketDepthSummary, max_levels: usize) -> Option<DepthDigest> {
    if summary.bid_levels.is_empty() && summary.ask_levels.is_empty() {
        return None;
    }

    fn ordered_levels(levels: &HashMap<Decimal, f64>, descending: bool, limit: usize) -> Vec<DepthLevel> {
        let mut pairs: Vec<_> = levels.iter().map(|(price, qty)| (*price, *qty)).collect();
        if descending {
            pairs.sort_by(|a, b| b.0.cmp(&a.0));
        } else {
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
        }
        pairs
            .into_iter()
            .take(limit)
            .map(|(price, quantity)| DepthLevel { price: price.to_f64().unwrap_or_default(), quantity })
            .collect()
    }

    let MarketDepthSummary { bid_levels, ask_levels, bid_size_at_best, ask_size_at_best, .. } = summary;

    let bids = ordered_levels(&bid_levels, true, max_levels);
    let asks = ordered_levels(&ask_levels, false, max_levels);

    let total_bid_volume: f64 = bid_levels.values().copied().sum();
    let total_ask_volume: f64 = ask_levels.values().copied().sum();

    Some(DepthDigest {
        bids,
        asks,
        bid_size_at_best,
        ask_size_at_best,
        total_bid_levels: bid_levels.len(),
        total_ask_levels: ask_levels.len(),
        total_bid_volume,
        total_ask_volume,
    })
}

fn compute_risk(system: &FinancialSystem) -> RiskDigest {
    let instruments = &system.instruments.instruments;
    let exchange = &system.exchange;

    let mut total_assets = 0.0;
    let mut total_liabilities = 0.0;
    let mut system_liquidity = 0.0;
    let mut total_loans = 0.0;
    let mut total_deposits = 0.0;

    for balance_sheet in system.balance_sheets.values() {
        for (inst_id, position) in &balance_sheet.assets {
            let value = position_value(instruments, exchange, inst_id, position);
            total_assets += value;

            if let Some(inst) = instruments.get(inst_id) {
                match inst.state() {
                    InstrumentRuntime::Cash(_) => {
                        system_liquidity += value;
                    }
                    InstrumentRuntime::Credit(state) => {
                        total_loans += state.exposure().to_f64();
                    }
                    InstrumentRuntime::RealAsset(RealAssetState::Inventory { goods, .. }) => {
                        let goods_total = goods.values().map(|g| g.quantity).sum::<f64>();
                        system_liquidity += goods_total;
                    }
                    _ => {}
                }
            }
        }

        for (inst_id, position) in &balance_sheet.liabilities {
            let value = position_value(instruments, exchange, inst_id, position);
            total_liabilities += value;

            if let Some(inst) = instruments.get(inst_id) {
                if let InstrumentRuntime::Cash(state) = inst.state() {
                    if matches!(
                        state.cash_type,
                        CashType::DemandDeposit | CashType::SavingsDeposit | CashType::TimeDeposit
                    ) {
                        total_deposits += value;
                    }
                }
            }
        }
    }

    let equity = total_assets - total_liabilities;
    let leverage = if equity.abs() > f64::EPSILON { total_assets / equity } else { f64::INFINITY };
    let capital_ratio = if total_assets > 0.0 { equity / total_assets } else { 0.0 };
    let liquidity_ratio = if total_liabilities > 0.0 { system_liquidity / total_liabilities } else { 0.0 };
    let loan_to_deposit_ratio = if total_deposits > 0.0 { total_loans / total_deposits } else { 0.0 };

    RiskDigest {
        total_assets,
        total_liabilities,
        system_liquidity,
        total_loans,
        total_deposits,
        leverage,
        capital_ratio,
        liquidity_ratio,
        loan_to_deposit_ratio,
    }
}

fn build_highlights(state: &SimState, markets: &MarketsDigest, events: &[SimEvent]) -> Vec<DigestEvent> {
    let summary = TickEventSummary::from_events(events);
    let mut highlights = Vec::with_capacity(5);

    highlights.push(DigestEvent::info("tick", format!("tick {} complete", state.ticknum)));
    highlights.push(DigestEvent::info("date", state.current_date.format("%Y-%m-%d").to_string()));
    highlights.push(DigestEvent::info(
        "events",
        format!("{} events ({} kinds)", summary.total_events, summary.by_kind.len()),
    ));

    if let Some(top_market_id) = markets.most_active.first() {
        if let Some(top_market) = markets.snapshots.iter().find(|m| &m.market_id == top_market_id) {
            highlights.push(DigestEvent::info(
                "market",
                format!("Most active: {} (vol {:.0})", top_market.label, top_market.volume),
            ));
        }
    }

    if let Some(top_event) = summary.by_kind.first() {
        highlights
            .push(DigestEvent::info("top_event", format!("{:?}: {} occurrences", top_event.kind, top_event.count)));
    }

    highlights
}

fn position_value(
    instruments: &HashMap<InstrumentId, Instrument>, exchange: &Exchange, inst_id: &InstrumentId, position: &Position,
) -> f64 {
    let price = exchange
        .financial_market(inst_id)
        .and_then(|market| market.representative_price())
        .or_else(|| instruments.get(inst_id).and_then(|inst| inst.unit_par_value()))
        .unwrap_or(position.book_value_per_unit);
    price.to_f64() * position.quantity
}

fn format_session(session: sim_core::types::core_utils::time::Session) -> &'static str {
    use sim_core::types::core_utils::time::Session;
    match session {
        Session::AM => "AM",
        Session::PM => "PM",
        Session::EOD => "EOD",
    }
}
