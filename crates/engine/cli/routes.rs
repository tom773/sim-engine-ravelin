use crate::{
    AppState, 
    dto::*, 
    market_routes::*,
    sim_history_routes::*,
};
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
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;
use sim_core::traits::{EconomicAnalytics, FinancialStatistics};

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
        let mut engine = state.scenario.initialize_engine();
        if let Err(e) = engine.connect_to_db().await {
            println!("[ERROR] Failed to connect to SurrealDB: {}", e);
        }
        *guard = Some(engine);
    }
    println!("Simulation initialized with epoch: {}", state.epoch);
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    (StatusCode::OK, headers, Json(InitResponse { epoch: state.epoch.to_string() }))
}

pub async fn get_dashboard(State(state): State<Arc<AppState>>) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let guard = state.sim_engine.read().await;
    if let Some(engine) = guard.as_ref() {
        let fs = &engine.state.financial_system;
        let agent_counts = AgentCounts {
            banks: engine.state.agents.banks.len(),
            firms: engine.state.agents.firms.len(),
            consumers: engine.state.agents.consumers.len(),
            total: engine.state.agents.banks.len()
                + engine.state.agents.firms.len()
                + engine.state.agents.consumers.len(),
        };

        let core_stats_data = engine.state.calculate_core_stats();
        let stats = engine.state.macro_stats();
        let bank_ids: HashSet<AgentId> = engine.state.agents.banks.keys().cloned().collect();

        let monetary_stats = MonetaryStats {
            velocity_m1: if stats.m1 > 0.0 { Some(core_stats_data.gdp / stats.m1) } else { None },
            velocity_m2: if stats.m2 > 0.0 { Some(core_stats_data.gdp / stats.m2) } else { None },
            m0: stats.m0,
            monetary_base: stats.m0,
            m1: stats.m1,
            m2: stats.m2,
            bank_reserves: fs.all_bank_reserves(&bank_ids),
            currency_in_circulation: fs.currency_in_circulation(fs.central_bank.id),
        };

        let core_stats_dto = CoreStats {
            gdp: core_stats_data.gdp,
            cpi: core_stats_data.cpi,
            ppi: core_stats_data.ppi,
            unemployment_rate: core_stats_data.unemployment_rate,
            labor_force_participation: core_stats_data.labor_force_participation,
            job_openings: core_stats_data.job_openings,
            capacity_utilization: core_stats_data.capacity_utilization,
            industrial_production: core_stats_data.industrial_production,
            housing_starts: core_stats_data.housing_starts,
            retail_sales: core_stats_data.consumption * 0.6,
            consumer_spending: core_stats_data.consumption,
            trade_balance: core_stats_data.trade_balance,
            credit_growth: core_stats_data.credit_growth,
            household_debt: core_stats_data.household_debt,
            corporate_debt: core_stats_data.corporate_debt,
            government_debt: core_stats_data.government_debt,
            bank_liabilities: core_stats_data.bank_liabilities,
        };

        let central_bank_policy = PolicyRates {
            policy_rate: fs.central_bank.policy_rate_bps,
            reserve_requirement: fs.central_bank.reserve_requirement,
        };

        let overnight_rates_data = fs.calculate_overnight_rates();
        let overnight_rates_dto = OvernightRatesDto {
            effr: overnight_rates_data.effr,
            sofr: overnight_rates_data.sofr,
            iorb: Some(overnight_rates_data.iorb),
            discount_rate: Some(overnight_rates_data.discount_rate),
            overnight_RRP: Some(overnight_rates_data.overnight_rrp),
        };

        let dashboard = DashboardDto {
            current_date: engine.state.current_date.format("%Y-%m-%d").to_string(),
            tick_number: engine.state.ticknum as u64,
            total_iterations: engine.state.config.iterations as u64,
            agent_counts,
            economic_stats: EconomicStats {
                core_stats: core_stats_dto,
                monetary_policy: central_bank_policy,
                monetary_stats,
                overnight_rates: overnight_rates_dto,
            },
        };

        (StatusCode::OK, headers, Json(serde_json::to_value(dashboard).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
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

pub async fn get_agent_balance_sheet(
    Path(agent_id): Path<AgentId>, 
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let guard = state.sim_engine.read().await;
    if let Some(engine) = guard.as_ref() {
        if !engine.state.agents.agent_exists(&agent_id) {
            return (StatusCode::NOT_FOUND, headers, Json(json!({ "error": "Agent not found" })));
        }
        if let Some(bs) = engine.state.financial_system.get_bs_by_id(&agent_id) {
            let balance_sheet = BalanceSheetSummary {
                assets: bs.total_assets(),
                liabilities: bs.total_liabilities(),
                equity: bs.net_worth(),
            };
            (StatusCode::OK, headers, Json(serde_json::to_value(balance_sheet).unwrap()))
        } else {
            (StatusCode::NOT_FOUND, headers, Json(json!({ "error": "Balance sheet not found" })))
        }
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}
pub async fn get_employment_contracts(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let guard = state.sim_engine.read().await;
    if let Some(engine) = guard.as_ref() {
        let contracts: Vec<EmploymentContractDto> = engine.state.agents.firms.values()
            .flat_map(|firm| firm.employees.iter().map(|emp| EmploymentContractDto {
                employee_id: emp.0.to_string(),
                firm_id: firm.id.to_string(),
                wage_rate: emp.1.wage_rate,
                hours: emp.1.hours,
            }))
            .collect();
        (StatusCode::OK, headers, Json(serde_json::to_value(contracts).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}
pub fn http_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/init", post(init_sim))
        .route("/dashboard", get(get_dashboard))
        .route("/sim/control/tick", post(tick))
        .route("/sim/analysis/stats", get(query_stats))
        .route("/sim/analysis/history", get(get_simulation_history))
        .route("/sim/analysis/actions", get(get_actions_history))
        .route("/sim/analysis/effects", get(get_effects_history))
        .route("/sim/analysis/a2e/{tick_number}", get(get_actions_to_effects))
        .route("/sim/analysis/tick/{tick_number}", get(get_tick_details))

        .route("/agents/{agent_type}", get(get_agents))
        .route("/agents/{agent_type}/summary", get(get_agents_summary))
        .route("/agents/{agent_id}/balance_sheet", get(get_agent_balance_sheet))
        .route("/api/markets/overview", get(get_markets_overview))
        
        .route("/api/markets/goods/cat", get(get_goods_catalogue))
        .route("/api/markets/goods/overview", get(get_market_goods_overview))
        .route("/api/markets/goods/{good_id}/orderbook", get(get_market_goods_orderbook))
        .route("/api/markets/goods/{good_id}/history", get(get_market_goods_history))
        
        .route("/api/markets/financial/overview", get(get_market_financial_overview))
        .route("/api/markets/financial/{instrument_id}/orderbook", get(get_market_financial_orderbook))
        .route("/api/markets/financial/{instrument_id}/history", get(get_market_financial_history))

        .route("/api/markets/labour/overview", get(get_market_labour_overview))
        .route("/api/markets/labour/contracts", get(get_employment_contracts))
        .with_state(state)
}