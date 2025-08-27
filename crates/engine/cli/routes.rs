use crate::{AppState,  market_routes::*, sim_history_routes::*};
use engine::dto::{*, query_dto::*};
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

    let query_service_guard = state.query_service.read().await;
    let query_service = match query_service_guard.as_ref() {
        Some(qs) => qs,
        None => {
            let err = ApiError { code: "SERVICE_UNAVAILABLE", message: "QueryService not available." };
            return (StatusCode::SERVICE_UNAVAILABLE, headers, Json(json!({ "error": err })));
        }
    };

    match query_service.get_dashboard_data().await {
        Ok(Some(dashboard_data)) => {
            let total_iterations = state.scenario.config.iterations as u64;
            let dashboard_dto = map_query_data_to_dashboard_dto(dashboard_data, total_iterations);
            (StatusCode::OK, headers, Json(serde_json::to_value(dashboard_dto).unwrap()))
        }
        Ok(None) => {
            let err = ApiError { code: "NOT_FOUND", message: "No dashboard data available." };
            (StatusCode::NOT_FOUND, headers, Json(json!({ "error": err })))
        }
        Err(e) => {
            let err = json!({ "error": format!("Database error: {}", e) });
            (StatusCode::INTERNAL_SERVER_ERROR, headers, Json(err))
        }
    }
}

pub async fn query_stats(State(state): State<Arc<AppState>>) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    
    let query_service_guard = state.query_service.read().await;
    let query_service = match query_service_guard.as_ref() {
        Some(qs) => qs,
        None => {
            let err = ApiError { code: "SERVICE_UNAVAILABLE", message: "QueryService not available." };
            return (StatusCode::SERVICE_UNAVAILABLE, headers, Json(json!({ "error": err })));
        }
    };

    match query_service.get_stats_data().await {
        Ok(Some(stats_data)) => {
            let response_json = map_query_stats_to_macro_stats(stats_data);
            (StatusCode::OK, headers, Json(response_json))
        }
        Ok(None) => {
            let err = ApiError { code: "NOT_FOUND", message: "No stats data available." };
            (StatusCode::NOT_FOUND, headers, Json(json!({ "error": err })))
        }
        Err(e) => {
            let err = json!({ "error": format!("Database error: {}", e) });
            (StatusCode::INTERNAL_SERVER_ERROR, headers, Json(err))
        }
    }
}

pub async fn get_agents_summary(
    Path(agent_type): Path<String>, 
    Query(pagination): Query<PaginationQuery>, 
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let page = pagination.page.unwrap_or(1).max(1);
    let page_size = pagination.page_size.unwrap_or(20).min(100).max(1);

    let query_service_guard = state.query_service.read().await;
    let query_service = match query_service_guard.as_ref() {
        Some(qs) => qs,
        None => {
            let err = ApiError { code: "SERVICE_UNAVAILABLE", message: "QueryService not available." };
            return (StatusCode::SERVICE_UNAVAILABLE, headers, Json(json!({ "error": err })));
        }
    };

    let db_agent_type = match agent_type.as_str() {
        "banks" => "Bank",
        "firms" => "Firm", 
        "consumers" => "Consumer",
        _ => {
            let err = json!({ "error": format!("Unknown agent type: {}", agent_type) });
            return (StatusCode::BAD_REQUEST, headers, Json(err));
        }
    };

    match query_service.get_agent_summaries(db_agent_type, page, page_size).await {
        Ok((summaries, total_count)) => {
            let items: Vec<AgentSummaryDto> = summaries
                .into_iter()
                .map(|summary| AgentSummaryDto {
                    id: summary.id.clone(),
                    name: match db_agent_type {
                        "Consumer" => format!("Consumer {}", &summary.id[..8]),
                        "Bank" => format!("Bank {}", &summary.id[..8]),
                        "Firm" => format!("Firm {}", &summary.id[..8]),
                        _ => summary.id.clone(),
                    },
                    agent_type: summary.agent_type,
                    balance_sheet: BalanceSheetSummary {
                        assets: summary.total_assets,
                        liabilities: summary.total_liabilities,
                        equity: summary.net_worth,
                    },
                    decision_model: "QueryService".to_string(),
                })
                .collect();

            let paginated = Paginated { 
                items, 
                total_items: total_count, 
                page, 
                page_size 
            };
            
            (StatusCode::OK, headers, Json(serde_json::to_value(paginated).unwrap()))
        }
        Err(e) => {
            let err = json!({ "error": format!("Database error: {}", e) });
            (StatusCode::INTERNAL_SERVER_ERROR, headers, Json(err))
        }
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

pub async fn get_agent_balance_sheet(
    Path(agent_id): Path<AgentId>, State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let guard = state.sim_engine.read().await;
    if let Some(engine) = guard.as_ref() {
        if !engine.state.agents.agent_exists(&agent_id) {
            return (StatusCode::NOT_FOUND, headers, Json(json!({ "error": "Agent not found" })));
        }
        if let Some(bs) = engine.state.financial_system.get_bs_by_id(&agent_id) {
            #[derive(Serialize)]
            struct BalanceSheetResponse<'a> {
                agent_id: &'a AgentId,
                balance_sheet: &'a BalanceSheet,
            }
            let bs_dto = BalanceSheetResponse { agent_id: &agent_id, balance_sheet: bs };
            (StatusCode::OK, headers, Json(serde_json::to_value(bs_dto).unwrap()))
        } else {
            (StatusCode::OK, headers, Json(json!({ "message": "No balance sheet found for the agent" })))
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
        let contracts: Vec<EmploymentContractDto> = engine
            .state
            .agents
            .firms
            .values()
            .flat_map(|firm| {
                firm.employees.iter().map(|emp| EmploymentContractDto {
                    employee_id: emp.0.to_string(),
                    firm_id: firm.id.to_string(),
                    wage_rate: emp.1.wage_rate,
                    hours: emp.1.hours,
                })
            })
            .collect();
        (StatusCode::OK, headers, Json(serde_json::to_value(contracts).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}

pub async fn get_non_agent_balance_sheets(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let guard = state.sim_engine.read().await;
    if let Some(engine) = guard.as_ref() {
        let central_bank_bs = engine.state.financial_system.get_bs_by_id(&engine.state.financial_system.central_bank.id);
        let government_bs = engine.state.financial_system.get_bs_by_id(&engine.state.financial_system.government.id);

        #[derive(Serialize)]
        struct NonAgentBalanceSheets<'a> {
            central_bank: Option<&'a BalanceSheet>,
            government: Option<&'a BalanceSheet>,
        }

        let response = NonAgentBalanceSheets { central_bank: central_bank_bs, government: government_bs };
        (StatusCode::OK, headers, Json(serde_json::to_value(response).unwrap()))
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
        .route("/sim/analysis/stats", get(query_stats))
        .route("/agents/{agent_type}/summary", get(get_agents_summary))
        
        .route("/sim/control/tick", post(tick))
        .route("/sim/analysis/market-snapshot", get(query_market_snapshot))
        .route("/agents/{agent_type}", get(get_agents))
        .route("/agents/{agent_id}/balance_sheet", get(get_agent_balance_sheet))
        .route("/sim/non_agent_balance_sheets", get(get_non_agent_balance_sheets))
        .route("/api/markets/labour/contracts", get(get_employment_contracts))
        
        .route("/sim/analysis/history", get(get_simulation_history))
        .route("/sim/analysis/actions", get(get_actions_history))
        .route("/sim/analysis/effects", get(get_effects_history))
        .route("/sim/analysis/a2e/{tick_number}", get(get_actions_to_effects))
        .route("/sim/analysis/tick/{tick_number}", get(get_tick_details))
        .route("/api/markets/overview", get(get_markets_overview))
        .route("/api/markets/goods/cat", get(get_goods_catalogue))
        .route("/api/markets/goods/overview", get(get_market_goods_overview))
        .route("/api/markets/goods/{good_id}/orderbook", get(get_market_goods_orderbook))
        .route("/api/markets/goods/{good_id}/history", get(get_market_goods_history))
        .route("/api/markets/financial/overview", get(get_market_financial_overview))
        .route("/api/markets/financial/{instrument_id}/orderbook", get(get_market_financial_orderbook))
        .route("/api/markets/financial/{instrument_id}/history", get(get_market_financial_history))
        .route("/api/markets/labour/overview", get(get_market_labour_overview))
        .with_state(state)
}