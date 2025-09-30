use super::agent_digest::{
    AgentsDigest, compute_agents, credit_rating_label,
    credit_rating_label_opt, sanitize_f64,
};
use super::market_digest::{
    MarketInfrastructureDigest, MarketsDigest, build_market_infrastructure,
    compute_markets,
};
use chrono::{DateTime, Utc};
use engine_v3::SimulationEngine;
#[cfg(target_arch = "wasm32")]
use js_sys::Date;
use metrics::histogram;
use rust_decimal::prelude::ToPrimitive;
use serde::ser::{Serializer};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sim_core::prelude::*;
use sim_core::types::core_utils::time::Session;
use sim_core::types::events::{SimEvent, TickEventSummary};
use sim_core::types::instrument::archetypes::InstrumentMarket;
use sim_core::types::instrument::{
    CashType, CreditState, DerivativeContract, Instrument, InstrumentRuntime, MarketProfile, RealAssetState,
    UnderlyingAsset,
};
use sim_core::types::markets::market::Exchange;
use sim_core::types::system::{balance_sheet::Position, financial_system::FinancialSystem};
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use tracing::instrument;
use uuid::Uuid;

const AGENT_LEADERBOARD_LIMIT: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDigest {
    pub tick: u32,
    pub sim_time: SimTimeDigest,
    pub status: StatusDigest,
    pub agents: AgentsDigest,
    pub markets: MarketsDigest,
    pub risk: RiskDigest,
    pub timings: DigestTimings,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruments: Option<InstrumentRegistryDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MetadataDigest>,
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
            timings: DigestTimings::bootstrap(),
            instruments: None,
            metadata: None,
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
pub struct DigestPhaseTiming {
    pub phase: String,
    pub duration_ms: f64,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labour: Option<LaborStatsDigest>,
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
pub struct LaborStatsDigest {
    pub labour_force: usize,
    pub employment: usize,
    pub unemployment: usize,
    pub unemployment_rate: f64,
    pub participation_rate: f64,
    pub job_openings: f64,
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
}

impl StateSnapshot {
    pub fn from_digest(digest: StateDigest) -> Self {
        Self { digest: Arc::new(digest) }
    }
}

impl Serialize for StateSnapshot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.digest.serialize(serializer)
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstrumentRegistryDigest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instruments: Vec<InstrumentMetaDigest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub goods: Vec<GoodDigest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recipes: Vec<RecipeDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstrumentMetaDigest {
    pub instrument_id: String,
    pub label: String,
    pub instrument_type: String,
    pub market: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub borrower_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterparty_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maturity_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coupon_bps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_par_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub face_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underlying: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GoodDigest {
    pub good_id: String,
    pub name: String,
    pub unit: String,
    pub category: String,
    pub cpi_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RecipeDigest {
    pub recipe_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<RecipeItemDigest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<RecipeItemDigest>,
    pub labour_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RecipeItemDigest {
    pub good_id: String,
    pub quantity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviourDigest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ticks: Vec<BehaviourTickDigest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent_events: Vec<SimEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviourTickDigest {
    pub tick: u32,
    pub date: String,
    pub summary: TickEventSummary,
    pub intention_count: usize,
    pub action_count: usize,
    pub effect_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<TickDetailDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickDetailDigest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub intentions: Vec<SimIntention>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionRecord>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<StateEffect>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<SimEvent>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub action_to_effect_indices: HashMap<usize, Vec<usize>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trades: Vec<Trade>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetadataDigest {
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub agent_names: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub agent_types: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub instrument_labels: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub market_labels: HashMap<String, String>,
}

#[instrument(skip(engine, _events))]
pub fn build_state_digest(engine: &SimulationEngine, _events: &[SimEvent]) -> StateDigest {
    let (digest, _) = build_state_digest_with_metrics(engine, _events);
    digest
}

#[instrument(skip(engine, _events))]
pub fn build_state_digest_with_metrics(
    engine: &SimulationEngine, _events: &[SimEvent],
) -> (StateDigest, Vec<DigestPhaseTiming>) {
    #[cfg(target_arch = "wasm32")]
    let build_start = Date::now();
    #[cfg(not(target_arch = "wasm32"))]
    let build_start = Instant::now();

    let mut profiler = DigestProfiler::new();

    let status = profiler.time("status", || compute_status(engine));
    let agents = profiler.time("agents", || compute_agents(engine, AGENT_LEADERBOARD_LIMIT));
    let mut markets = profiler.time("markets", || compute_markets(&engine.state));
    let instruments =
        profiler.time("instrument_registry", || build_instrument_registry(&engine.state.financial_system));
    let infrastructure =
        profiler.time("market_infrastructure", || build_market_infrastructure(&engine.state, &markets, &instruments));
    if !infrastructure.listings.is_empty() || !infrastructure.omo_actions.is_empty() {
        markets.infrastructure = Some(infrastructure);
    }

    let risk = profiler.time("risk", || compute_risk(&engine.state.financial_system));
    let metadata = profiler
        .time("metadata", || build_metadata_digest(engine, &agents, markets.infrastructure.as_ref(), &instruments));

    #[cfg(target_arch = "wasm32")]
    let build_duration_ms = Date::now() - build_start;
    #[cfg(not(target_arch = "wasm32"))]
    let build_duration_ms = build_start.elapsed().as_secs_f64() * 1_000.0;

    let timings = DigestTimings { generated_at: Utc::now(), build_duration_ms };
    let phases = profiler.finish();

    histogram!("mirror.digest.build_ms", timings.build_duration_ms);
    let agent_count = agents.catalogue.as_ref().map(|c| c.roster.len()).unwrap_or(0);
    histogram!("mirror.digest.snapshot.size", (agent_count + markets.snapshots.len()) as f64);

    let instruments_opt =
        if instruments.instruments.is_empty() && instruments.goods.is_empty() { None } else { Some(instruments) };

    let metadata_opt = if metadata.agent_names.is_empty()
        && metadata.agent_types.is_empty()
        && metadata.instrument_labels.is_empty()
        && metadata.market_labels.is_empty()
    {
        None
    } else {
        Some(metadata)
    };

    let digest = StateDigest {
        tick: engine.state.ticknum,
        sim_time: SimTimeDigest {
            current_date: engine.state.current_date.format("%Y-%m-%d").to_string(),
            session: format_session(engine.state.current_session),
        },
        status,
        agents,
        markets,
        risk,
        timings,
        instruments: instruments_opt,
        metadata: metadata_opt,
    };

    (digest, phases)
}

struct DigestProfiler {
    phases: Vec<DigestPhaseTiming>,
}

impl DigestProfiler {
    fn new() -> Self {
        Self { phases: Vec::new() }
    }

    fn time<T, F>(&mut self, phase: &'static str, op: F) -> T
    where
        F: FnOnce() -> T,
    {
        let timer = PhaseTimer::start();
        let result = op();
        let duration_ms = timer.elapsed_ms();
        self.phases.push(DigestPhaseTiming { phase: phase.to_string(), duration_ms });
        result
    }

    fn finish(self) -> Vec<DigestPhaseTiming> {
        self.phases
    }
}

#[cfg(target_arch = "wasm32")]
struct PhaseTimer {
    start_ms: f64,
}

#[cfg(target_arch = "wasm32")]
impl PhaseTimer {
    fn start() -> Self {
        Self { start_ms: Date::now() }
    }

    fn elapsed_ms(&self) -> f64 {
        Date::now() - self.start_ms
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct PhaseTimer {
    start: Instant,
}

#[cfg(not(target_arch = "wasm32"))]
impl PhaseTimer {
    fn start() -> Self {
        Self { start: Instant::now() }
    }

    fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1_000.0
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
        labour: Some(LaborStatsDigest {
            labour_force: macro_stats.labour_force,
            employment: macro_stats.employment,
            unemployment: macro_stats.unemployment,
            unemployment_rate: macro_stats.unemployment_rate,
            participation_rate: macro_stats.labor_force_participation,
            job_openings: macro_stats.job_openings,
        }),
    }
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


fn build_metadata_digest(
    engine: &SimulationEngine, agents: &AgentsDigest, infrastructure: Option<&MarketInfrastructureDigest>,
    instruments: &InstrumentRegistryDigest,
) -> MetadataDigest {
    let mut agent_names: HashMap<String, String> = HashMap::new();
    let mut agent_types: HashMap<String, String> = HashMap::new();

    if let Some(catalogue) = agents.catalogue.as_ref() {
        for agent in &catalogue.roster {
            let agent_id = agent.agent_id.to_string();
            agent_names.insert(agent_id.clone(), agent.name.clone());
            agent_types.insert(agent_id, agent.agent_type.clone());
        }
    } else {
        for agent_id in engine.state.agents.all_agent_ids() {
            let (agent_type, name) = engine.get_agent_info(&agent_id);
            let uuid = agent_id.0;
            let agent_id_str = uuid.to_string();
            agent_types.insert(agent_id_str.clone(), agent_type);
            if let Some(name) = name {
                agent_names.insert(agent_id_str, name);
            }
        }
    }

    let mut instrument_labels: HashMap<String, String> = HashMap::new();
    for instrument in &instruments.instruments {
        instrument_labels.insert(instrument.instrument_id.clone(), instrument.label.clone());
    }

    let mut market_labels: HashMap<String, String> = HashMap::new();
    if let Some(infra) = infrastructure {
        for listing in &infra.listings {
            if let Some(label) = &listing.label {
                market_labels.insert(listing.symbol.clone(), label.clone());
            }
        }
    }

    MetadataDigest { agent_names, agent_types, instrument_labels, market_labels }
}


fn build_instrument_registry(system: &FinancialSystem) -> InstrumentRegistryDigest {
    let mut instruments: Vec<InstrumentMetaDigest> = Vec::with_capacity(system.instruments.instruments.len());

    for (instrument_id, instrument) in &system.instruments.instruments {
        let mut meta = InstrumentMetaDigest {
            instrument_id: instrument_id.to_string(),
            label: instrument.label().to_string(),
            instrument_type: instrument.type_as_string().to_string(),
            market: instrument_market_label(instrument.market_profile()),
            issuer_id: None,
            borrower_id: None,
            counterparty_id: None,
            currency: None,
            maturity_date: None,
            coupon_bps: None,
            unit_par_value: instrument.unit_par_value().map(|m| sanitize_f64(m.to_f64())),
            face_value: instrument.face_value().map(|m| sanitize_f64(m.to_f64())),
            underlying: None,
            extra: HashMap::new(),
        };

        match instrument.state() {
            InstrumentRuntime::Cash(cash) => {
                meta.issuer_id = Some(cash.issuer.0);
                meta.currency = Some(format!("{:?}", cash.currency));
                meta.coupon_bps = Some(rate_to_f64(cash.interest_bps));
                meta.extra.insert("cash_type".into(), json!(format!("{:?}", cash.cash_type)));
            }
            InstrumentRuntime::Bond(bond) => {
                meta.issuer_id = Some(bond.issuer.0);
                meta.maturity_date = Some(bond.maturity_date.format("%Y-%m-%d").to_string());
                meta.coupon_bps = Some(rate_to_f64(bond.archetype.coupon_rate_bps));
                meta.extra.insert("issue_date".into(), json!(bond.issue_date.format("%Y-%m-%d").to_string()));
                meta.extra.insert("bond_type".into(), json!(format!("{:?}", bond.bond_type())));
                meta.extra.insert("cash_flow_type".into(), json!(format!("{:?}", bond.cash_flow_type())));
                meta.extra.insert("outstanding_units".into(), json!(sanitize_f64(bond.outstanding_units)));
                if let Some(label) = credit_rating_label_opt(bond.rating) {
                    meta.extra.insert("rating".into(), json!(label));
                }
            }
            InstrumentRuntime::Credit(credit) => match credit {
                CreditState::Loan(loan) => {
                    meta.issuer_id = Some(loan.lender.0);
                    meta.borrower_id = Some(loan.borrower.0);
                    meta.maturity_date = Some(loan.maturity_date.format("%Y-%m-%d").to_string());
                    meta.extra.insert("loan_type".into(), json!(format!("{:?}", loan.loan_type)));
                    meta.extra.insert("rate_index".into(), json!(format!("{:?}", loan.rate_index())));
                    meta.extra.insert("spread_bps".into(), json!(rate_to_f64(loan.spread_bps())));
                    if let Some(label) = credit_rating_label_opt(loan.rating) {
                        meta.extra.insert("rating".into(), json!(label));
                    }
                }
                CreditState::ConsumerLoan { category, loan } => {
                    meta.issuer_id = Some(loan.lender.0);
                    meta.borrower_id = Some(loan.borrower.0);
                    meta.maturity_date = Some(loan.maturity_date.format("%Y-%m-%d").to_string());
                    meta.extra.insert("category".into(), json!(format!("{:?}", category)));
                    meta.extra.insert("rate_index".into(), json!(format!("{:?}", loan.rate_index())));
                    meta.extra.insert("spread_bps".into(), json!(rate_to_f64(loan.spread_bps())));
                    if let Some(label) = credit_rating_label_opt(loan.rating) {
                        meta.extra.insert("rating".into(), json!(label));
                    }
                }
                CreditState::ConsumerCreditCard(facility) => {
                    meta.issuer_id = Some(facility.lender.0);
                    meta.borrower_id = Some(facility.borrower.0);
                    meta.maturity_date = Some(facility.expiry_date.format("%Y-%m-%d").to_string());
                    meta.extra
                        .insert("commitment_amount".into(), json!(sanitize_f64(facility.commitment_amount.to_f64())));
                    meta.extra
                        .insert("available_amount".into(), json!(sanitize_f64(facility.available_amount.to_f64())));
                    meta.extra.insert("drawn_amount".into(), json!(sanitize_f64(facility.drawn_amount.to_f64())));
                }
                CreditState::Facility(facility) => {
                    meta.issuer_id = Some(facility.lender.0);
                    meta.borrower_id = Some(facility.borrower.0);
                    meta.maturity_date = Some(facility.expiry_date.format("%Y-%m-%d").to_string());
                    meta.extra.insert("facility_type".into(), json!(format!("{:?}", facility.facility_type)));
                    meta.extra
                        .insert("commitment_amount".into(), json!(sanitize_f64(facility.commitment_amount.to_f64())));
                    meta.extra
                        .insert("available_amount".into(), json!(sanitize_f64(facility.available_amount.to_f64())));
                    meta.extra.insert("drawn_amount".into(), json!(sanitize_f64(facility.drawn_amount.to_f64())));
                    meta.extra.insert("spread_bps".into(), json!(rate_to_f64(facility.spread_bps)));
                }
                CreditState::TradeCredit(trade) => {
                    meta.issuer_id = Some(trade.creditor.0);
                    meta.borrower_id = Some(trade.debtor.0);
                    meta.maturity_date = Some(trade.due_date.format("%Y-%m-%d").to_string());
                    meta.extra.insert("invoice_amount".into(), json!(sanitize_f64(trade.amount.to_f64())));
                }
            },
            InstrumentRuntime::Equity(equity) => {
                meta.issuer_id = Some(equity.profile.issuer.0);
                meta.extra.insert("share_class".into(), json!(format!("{:?}", equity.profile.share_class)));
                meta.extra.insert("outstanding_shares".into(), json!(equity.outstanding_shares));
            }
            InstrumentRuntime::Structured(tranche) => {
                meta.issuer_id = Some(tranche.issuer.0);
                meta.maturity_date = Some(tranche.maturity_date.format("%Y-%m-%d").to_string());
                meta.coupon_bps = Some(rate_to_f64(tranche.coupon_rate_bps));
                meta.extra.insert("tranche_type".into(), json!(format!("{:?}", tranche.tranche_type)));
                meta.extra.insert("rating".into(), json!(credit_rating_label(tranche.rating)));
                meta.extra.insert("attachment_point".into(), json!(rate_to_f64(tranche.attachment_point)));
                meta.extra.insert("detachment_point".into(), json!(rate_to_f64(tranche.detachment_point)));
            }
            InstrumentRuntime::Derivative(derivative) => {
                meta.issuer_id = Some(derivative.issuer.0);
                meta.counterparty_id = derivative.counterparty.map(|id| id.0);
                if let Some(expiry) = derivative.expiry_date {
                    meta.maturity_date = Some(expiry.format("%Y-%m-%d").to_string());
                }
                meta.underlying = Some(match &derivative.underlying {
                    UnderlyingAsset::Instrument(id) => format!("instrument: {}", id),
                    UnderlyingAsset::Good(id) => format!("good: {}", id),
                    UnderlyingAsset::Index(name) => format!("index: {name}"),
                });
                meta.extra.insert(
                    "contract".into(),
                    json!(match &derivative.contract {
                        DerivativeContract::Option(option) => format!("Option {:?}", option.style),
                        DerivativeContract::Future(_) => "Future".to_string(),
                        DerivativeContract::Custom { description } => description.clone(),
                    }),
                );
            }
            InstrumentRuntime::Repo(repo) => {
                meta.issuer_id = Some(repo.lender.0);
                meta.borrower_id = Some(repo.borrower.0);
                meta.counterparty_id = Some(repo.borrower.0);
                meta.maturity_date = Some(repo.end_date.format("%Y-%m-%d").to_string());
                meta.extra.insert("start_date".into(), json!(repo.start_date.format("%Y-%m-%d").to_string()));
                meta.extra.insert("collateral".into(), json!(repo.collateral_id.to_string()));
                meta.extra.insert("collateral_qty".into(), json!(sanitize_f64(repo.collateral_quantity)));
                meta.extra.insert("interest_bps".into(), json!(rate_to_f64(repo.interest_bps)));
            }
            InstrumentRuntime::RealAsset(asset) => match asset {
                RealAssetState::Inventory { owner, goods } => {
                    meta.issuer_id = Some(owner.0);
                    let total_goods: HashMap<String, f64> = goods
                        .iter()
                        .map(|(good_id, item)| (good_id.to_string(), sanitize_f64(item.quantity)))
                        .collect();
                    meta.extra.insert("inventory".into(), json!(total_goods));
                }
                RealAssetState::Property { owner, address, sq_ft, market_value } => {
                    meta.issuer_id = Some(owner.0);
                    meta.extra.insert("address".into(), json!(address));
                    meta.extra.insert("square_feet".into(), json!(sq_ft));
                    meta.extra.insert("market_value".into(), json!(sanitize_f64(market_value.to_f64())));
                }
                RealAssetState::Custom { owner, description, metadata } => {
                    meta.issuer_id = Some(owner.0);
                    meta.extra.insert("description".into(), json!(description));
                    meta.extra.insert("metadata".into(), json!(metadata));
                }
            },
        }

        instruments.push(meta);
    }

    instruments.sort_by(|a, b| a.instrument_id.cmp(&b.instrument_id));

    let mut goods: Vec<GoodDigest> = system
        .goods
        .goods
        .iter()
        .map(|(good_id, good)| GoodDigest {
            good_id: good_id.to_string(),
            name: good.name.clone(),
            unit: good.unit.clone(),
            category: format!("{:?}", good.category),
            cpi_weight: good.cpi_weight,
        })
        .collect();
    goods.sort_by(|a, b| a.good_id.cmp(&b.good_id));

    let mut recipes: Vec<RecipeDigest> = system
        .goods
        .recipes
        .iter()
        .map(|(recipe_id, recipe)| RecipeDigest {
            recipe_id: recipe_id.0.clone(),
            name: recipe.name.clone(),
            inputs: recipe
                .inputs
                .iter()
                .map(|io| RecipeItemDigest { good_id: io.good_id.to_string(), quantity: io.quantity })
                .collect(),
            outputs: recipe
                .outputs
                .iter()
                .map(|io| RecipeItemDigest { good_id: io.good_id.to_string(), quantity: io.quantity })
                .collect(),
            labour_hours: recipe.labour_hours,
        })
        .collect();
    recipes.sort_by(|a, b| a.recipe_id.cmp(&b.recipe_id));

    InstrumentRegistryDigest { instruments, goods, recipes }
}

fn instrument_market_label(profile: &MarketProfile) -> String {
    match profile.market {
        InstrumentMarket::MoneyMarket(segment) => format!("MoneyMarket::{segment:?}"),
        InstrumentMarket::CapitalMarket(segment) => format!("CapitalMarket::{segment:?}"),
        InstrumentMarket::DerivativesMarket(segment) => format!("DerivativesMarket::{segment:?}"),
        InstrumentMarket::Unlisted => "Unlisted".into(),
    }
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

pub(crate) fn rate_to_f64(rate: Rate) -> f64 {
    rate.to_f64().unwrap_or_default()
}

fn format_session(session: Session) -> String {
    match session {
        Session::AM => "AM".into(),
        Session::PM => "PM".into(),
        Session::EOD => "EOD".into(),
    }
}
