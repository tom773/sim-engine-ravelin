use crate::{AppState, SimulationEngine, dto::*, market_routes::*};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use rand::rngs::ThreadRng;
use serde::Serialize;
use serde_json::json;
use sim_core::*;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize)]
pub struct Health {
    pub initialized: bool,
    pub epoch: String,
}

#[derive(Serialize)]
pub struct ApiError {
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(Serialize)]
pub struct InitResponse {
    pub epoch: String,
}

#[derive(serde::Deserialize)]
pub struct PaginationQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

pub fn with_epoch_header(mut headers: HeaderMap, epoch: Uuid) -> HeaderMap {
    headers.insert("X-Server-Epoch", epoch.to_string().parse().unwrap());
    headers
}

pub async fn healthz(State(state): State<Arc<AppState>>) -> (HeaderMap, Json<Health>) {
    let initialized = state.sim_engine.read().await.is_some();
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    (headers, Json(Health { initialized, epoch: state.epoch.to_string() }))
}

pub async fn init_sim(State(state): State<Arc<AppState>>) -> (StatusCode, HeaderMap, Json<InitResponse>) {
    let mut guard = state.sim_engine.write().await;
    if guard.is_none() {
        let engine = state.scenario.initialize_engine();
        *guard = Some(engine);
    }
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    (StatusCode::OK, headers, Json(InitResponse { epoch: state.epoch.to_string() }))
}

fn calculate_overnight_rates(engine: &SimulationEngine) -> OvernightRatesDto {
    let fed_funds_rate = engine
        .state
        .financial_system
        .exchange
        .financial_markets
        .get(&FinancialMarketId::FederalFundsOvernight)
        .and_then(|market| market.last_or_mid())
        .map(|price| {
            let daily_rate = FinancialMarketId::FederalFundsOvernight.price_to_daily_rate(price);
            daily_rate * 360.0 * 100.0
        })
        .unwrap_or(engine.state.financial_system.central_bank.policy_rate_bps / 100.0);

    let sofr = engine
        .state
        .financial_system
        .exchange
        .financial_markets
        .get(&FinancialMarketId::TreasuryRepoOvernight)
        .and_then(|market| market.last_or_mid())
        .map(|price| {
            let daily_rate = FinancialMarketId::TreasuryRepoOvernight.price_to_daily_rate(price);
            daily_rate * 360.0 * 100.0
        })
        .unwrap_or(fed_funds_rate - 0.10);

    OvernightRatesDto {
        effr: engine
            .state
            .financial_system
            .exchange
            .financial_markets
            .get(&FinancialMarketId::FederalFundsOvernight)
            .and_then(|market| market.last_or_mid())
            .map(|price| {
                let daily_rate = FinancialMarketId::FederalFundsOvernight.price_to_daily_rate(price);
                daily_rate * 360.0 * 100.0
            }),
        sofr: Some(sofr),
        iorb: Some((engine.state.financial_system.central_bank.policy_rate_bps + 15.0) / 100.0),
        discount_rate: Some((engine.state.financial_system.central_bank.policy_rate_bps + 25.0) / 100.0),
        overnight_RRP: Some((engine.state.financial_system.central_bank.policy_rate_bps).max(0.0) / 100.0),
    }
}

pub async fn get_dashboard(State(state): State<Arc<AppState>>) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let guard = state.sim_engine.read().await;

    if let Some(engine) = guard.as_ref() {
        let stats = engine.state.macro_stats();

        let banks = engine.state.agents.banks.values().cloned().collect::<Vec<_>>();

        let agent_counts = AgentCounts {
            banks: engine.state.agents.banks.len(),
            firms: engine.state.agents.firms.len(),
            consumers: engine.state.agents.consumers.len(),
            total: engine.state.agents.banks.len()
                + engine.state.agents.firms.len()
                + engine.state.agents.consumers.len(),
        };

        let monetary_stats = MonetaryStats {
            velocity_m1: Some(50_000_000_000.0 / stats.m1),
            velocity_m2: Some(50_000_000_000.0 / stats.m2),
            m0: stats.m0,
            monetary_base: stats.m0,
            m1: stats.m1,
            m2: stats.m2,
            bank_reserves: engine.state.financial_system.all_bank_reserves(&banks.iter().map(|b| b.id).collect()),
            currency_in_circulation: engine
                .state
                .financial_system
                .currency_in_circulation(engine.state.financial_system.central_bank.id),
        };

        let core_stats = calculate_enhanced_core_stats(engine, &stats);

        let central_bank_policy = PolicyRates {
            policy_rate: engine.state.financial_system.central_bank.policy_rate_bps / 100.0,
            reserve_requirement: engine.state.financial_system.central_bank.reserve_requirement,
        };

        let overnight_rates = calculate_overnight_rates(engine);

        let dashboard = DashboardDto {
            current_date: engine.state.current_date.format("%Y-%m-%d").to_string(),
            tick_number: engine.state.ticknum as u64,
            total_iterations: engine.state.config.iterations as u64,
            agent_counts,
            economic_stats: EconomicStats {
                core_stats,
                monetary_policy: central_bank_policy,
                monetary_stats,
                overnight_rates,
            },
        };

        (StatusCode::OK, headers, Json(serde_json::to_value(dashboard).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}

fn calculate_enhanced_core_stats(engine: &SimulationEngine, _base_stats: &MacroStats) -> CoreStats {
    let total_firm_production = engine.state.agents.firms.values().map(|_firm| 1000.0).sum::<f64>();

    let total_consumer_spending = engine.state.agents.consumers.values().map(|_consumer| 0.0).sum::<f64>();

    let employed_agents =
        engine.state.agents.consumers.values().filter(|consumer| consumer.employed_by.is_some()).count();
    let bank_liabilities = engine
        .state
        .agents
        .banks
        .values()
        .map(|bank| {
            if let Some(bs) = engine.state.financial_system.get_bs_by_id(&bank.id) {
                bs.total_liabilities()
            } else {
                0.0
            }
        })
        .sum::<f64>();
    let labor_force = engine.state.agents.consumers.len();
    let unemployment_rate = if labor_force > 0 { 1.0 - (employed_agents as f64 / labor_force as f64) } else { 0.0 };

    let trade_balance = calculate_trade_balance(engine);

    let capacity_utilization = calculate_capacity_utilization(engine);

    CoreStats {
        gdp: total_firm_production + total_consumer_spending,
        cpi: calculate_cpi(engine),
        ppi: calculate_ppi(engine),
        unemployment_rate: unemployment_rate * 100.0,
        labor_force_participation: 62.5,
        job_openings: calculate_job_openings(engine),
        capacity_utilization,
        industrial_production: total_firm_production,
        housing_starts: 150_000.0,
        retail_sales: total_consumer_spending * 0.6,
        consumer_spending: total_consumer_spending,
        trade_balance,
        credit_growth: calculate_credit_growth(engine),
        household_debt: calculate_household_debt(engine),
        corporate_debt: calculate_corporate_debt(engine),
        government_debt: calculate_government_debt(engine),
        bank_liabilities,
    }
}

// TODO

fn calculate_trade_balance(_engine: &SimulationEngine) -> f64 {
    -50_000_000_000.0
}

fn calculate_capacity_utilization(_engine: &SimulationEngine) -> f64 {
    87.2
}

fn calculate_cpi(_engine: &SimulationEngine) -> f64 {
    245.0
}

fn calculate_ppi(_engine: &SimulationEngine) -> f64 {
    250.0
}

fn calculate_job_openings(engine: &SimulationEngine) -> f64 {
    engine.state.agents.firms.len() as f64
}

fn calculate_credit_growth(_engine: &SimulationEngine) -> f64 {
    0.05
}

fn calculate_household_debt(engine: &SimulationEngine) -> f64 {
    engine
        .state
        .agents
        .consumers
        .values()
        .map(|consumer| {
            if let Some(bs) = engine.state.financial_system.get_bs_by_id(&consumer.id) {
                bs.total_liabilities()
            } else {
                0.0
            }
        })
        .sum()
}

fn calculate_corporate_debt(engine: &SimulationEngine) -> f64 {
    engine
        .state
        .agents
        .firms
        .values()
        .map(|firm| {
            if let Some(bs) = engine.state.financial_system.get_bs_by_id(&firm.id) {
                bs.total_liabilities()
            } else {
                0.0
            }
        })
        .sum()
}

fn calculate_government_debt(engine: &SimulationEngine) -> f64 {
    let gid = engine.state.financial_system.government.get_id();
    if let Some(bs) = engine.state.financial_system.get_bs_by_id(&gid) { bs.total_liabilities() } else { 0.0 }
}

pub async fn get_agents_summary(
    Path(agent_type): Path<String>, Query(pagination): Query<PaginationQuery>, State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let guard = state.sim_engine.read().await;

    if let Some(engine) = guard.as_ref() {
        let page = pagination.page.unwrap_or(1).max(1);
        let page_size = pagination.page_size.unwrap_or(20).min(100).max(1);
        let skip = ((page - 1) * page_size) as usize;

        let (items, total_items) = match agent_type.as_str() {
            "banks" => {
                let all_banks: Vec<_> = engine.state.agents.banks.values().collect();
                let total = all_banks.len();
                let banks: Vec<AgentSummaryDto> = all_banks
                    .into_iter()
                    .skip(skip)
                    .take(page_size as usize)
                    .map(|bank| {
                        let bs = engine.state.financial_system.get_bs_by_id(&bank.id);
                        let balance_sheet = if let Some(bs) = bs {
                            BalanceSheetSummary {
                                assets: bs.total_assets(),
                                liabilities: bs.total_liabilities(),
                                equity: bs.net_worth(),
                            }
                        } else {
                            BalanceSheetSummary { assets: 0.0, liabilities: 0.0, equity: 0.0 }
                        };

                        AgentSummaryDto {
                            id: bank.id.to_string(),
                            name: bank.name.clone(),
                            agent_type: "Bank".to_string(),
                            balance_sheet,
                        }
                    })
                    .collect();
                (banks, total as u64)
            }
            "firms" => {
                let all_firms: Vec<_> = engine.state.agents.firms.values().collect();
                let total = all_firms.len();
                let firms: Vec<AgentSummaryDto> = all_firms
                    .into_iter()
                    .skip(skip)
                    .take(page_size as usize)
                    .map(|firm| {
                        let bs = engine.state.financial_system.get_bs_by_id(&firm.id);
                        let balance_sheet = if let Some(bs) = bs {
                            BalanceSheetSummary {
                                assets: bs.total_assets(),
                                liabilities: bs.total_liabilities(),
                                equity: bs.net_worth(),
                            }
                        } else {
                            BalanceSheetSummary { assets: 0.0, liabilities: 0.0, equity: 0.0 }
                        };

                        AgentSummaryDto {
                            id: firm.id.to_string(),
                            name: firm.name.clone(),
                            agent_type: "Firm".to_string(),
                            balance_sheet,
                        }
                    })
                    .collect();
                (firms, total as u64)
            }
            "consumers" => {
                let all_consumers: Vec<_> = engine.state.agents.consumers.values().collect();
                let total = all_consumers.len();
                let consumers: Vec<AgentSummaryDto> = all_consumers
                    .into_iter()
                    .skip(skip)
                    .take(page_size as usize)
                    .map(|consumer| {
                        let bs = engine.state.financial_system.get_bs_by_id(&consumer.id);
                        let balance_sheet = if let Some(bs) = bs {
                            BalanceSheetSummary {
                                assets: bs.total_assets(),
                                liabilities: bs.total_liabilities(),
                                equity: bs.net_worth(),
                            }
                        } else {
                            BalanceSheetSummary { assets: 0.0, liabilities: 0.0, equity: 0.0 }
                        };

                        AgentSummaryDto {
                            id: consumer.id.to_string(),
                            name: format!("Consumer {}", &consumer.id.to_string()[..8]),
                            agent_type: "Consumer".to_string(),
                            balance_sheet,
                        }
                    })
                    .collect();
                (consumers, total as u64)
            }
            _ => {
                let err = json!({ "error": format!("Unknown agent type: {}", agent_type) });
                return (StatusCode::BAD_REQUEST, headers, Json(err));
            }
        };

        let paginated = Paginated { items, total_items, page, page_size };

        (StatusCode::OK, headers, Json(serde_json::to_value(paginated).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}

pub async fn get_agents(
    Path(kind): Path<String>, State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let guard = state.sim_engine.read().await;
    if let Some(engine) = guard.as_ref() {
        let body = match kind.as_str() {
            "banks" => json!({ "banks": engine.state.agents.banks.values().cloned().collect::<Vec<_>>() }),
            "firms" => json!({ "firms": engine.state.agents.firms.values().cloned().collect::<Vec<_>>() }),
            "consumers" => json!({ "consumers": engine.state.agents.consumers.values().cloned().collect::<Vec<_>>() }),
            _ => json!({ "error": format!("Unknown agent type: {}", kind) }),
        };
        (StatusCode::OK, headers, Json(body))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}

pub async fn tick(State(state): State<Arc<AppState>>) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let mut engine_guard = state.sim_engine.write().await;
    if let Some(engine) = engine_guard.as_mut() {
        let mut rng = ThreadRng::default();
        let result = engine.tick(&mut rng);
        (StatusCode::OK, headers, Json(json!({ "status": "Tick completed", "tick_number": result.tick_number })))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}

pub async fn query_market_snapshot(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;
    if let Some(engine) = engine_guard.as_ref() {
        let snapshot = engine.state.all_market_views();
        (StatusCode::OK, headers, Json(json!({ "market_snapshot": snapshot })))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(serde_json::json!({ "error": err })))
    }
}

pub async fn query_stats(State(state): State<Arc<AppState>>) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;
    if let Some(engine) = engine_guard.as_ref() {
        let stats = engine.state.macro_stats().clone();
        (StatusCode::OK, headers, Json(serde_json::to_value(stats).unwrap_or(serde_json::json!({}))))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(serde_json::json!({ "error": err })))
    }
}

pub fn http_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/init", post(init_sim))
        .route("/dashboard", get(get_dashboard))
        .route("/sim/control/tick", post(tick))
        .route("/sim/analysis/stats", get(query_stats))
        .route("/agents/{agent_type}", get(get_agents))
        .route("/agents/{agent_type}/summary", get(get_agents_summary))
        .route("/api/markets/overview", get(get_markets_overview))
        .route("/api/markets/goods/cat", get(get_goods_catalogue))
        .route("/api/markets/goods/overview", get(get_market_goods_overview))
        .route("/api/markets/goods/{good_id}/orderbook", get(get_market_goods_orderbook))
        .route("/api/markets/goods/{good_id}/history", get(get_market_goods_history))
        .route("/api/markets/financial/overview", get(get_market_financial_overview))
        .route("/api/markets/financial/{instrument_id}/orderbook", get(get_market_financial_orderbook))
        .route("/api/markets/financial/{instrument_id}/history", get(get_market_financial_history))
        .with_state(state)
}
