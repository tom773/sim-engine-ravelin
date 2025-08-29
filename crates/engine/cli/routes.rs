use crate::{AppState, market_routes::*, sim_history_routes::*};
use engine::{*, dto::{query_dto::*}};
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

pub async fn get_qs(state: &Arc<AppState>) -> Result<Arc<QueryService>, (StatusCode, HeaderMap, Json<serde_json::Value>)> {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let qs_opt = state.query_service.lock().await.clone(); 
    
    match qs_opt {
        Some(qs) => Ok(qs),
        None => {
            let err = ApiError { code: "SERVICE_UNAVAILABLE", message: "QueryService not initialized." };
            Err((StatusCode::SERVICE_UNAVAILABLE, headers, Json(json!({ "error": err }))))
        }
    }
}

pub fn with_epoch_header(mut headers: HeaderMap, epoch: Uuid) -> HeaderMap {
    headers.insert("X-Server-Epoch", epoch.to_string().parse().unwrap());
    headers
}

pub async fn healthz(State(state): State<Arc<AppState>>) -> (HeaderMap, Json<Health>) {
    let initialized = state.sim_engine.try_lock().map(|guard| guard.is_some()).unwrap_or(true);
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    (headers, Json(Health { initialized, epoch: state.epoch.to_string() }))
}

pub async fn init_sim(State(state): State<Arc<AppState>>) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let mut guard = state.sim_engine.lock().await;
    if guard.is_none() {
        let mut engine = state.scenario.initialize_engine();
        
        if let Some(qs) = state.query_service.lock().await.as_ref() {
             engine.set_db_writer(qs.get_writer());
        } else {
             let err = ApiError { code: "SERVICE_UNAVAILABLE", message: "QueryService is not available to link to the new engine." };
             return (StatusCode::SERVICE_UNAVAILABLE, HeaderMap::new(), Json(json!({ "error": err })));
        }
        *guard = Some(engine);
    }
    println!("Simulation initialized with epoch: {}", state.epoch);
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    (StatusCode::OK, headers, Json(json!( { "epoch": state.epoch.to_string() })))
}

pub async fn get_dashboard(State(state): State<Arc<AppState>>) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);

    let query_service = match get_qs(&state).await {
        Ok(qs) => qs,
        Err(response) => return response,
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
    
    let query_service = match get_qs(&state).await {
        Ok(qs) => qs,
        Err(response) => return response,
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

// in routes.rs

pub async fn get_agents_summary(
    Path(agent_type): Path<String>,
    Query(pagination): Query<PaginationQuery>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let page = pagination.page.unwrap_or(1).max(1);
    let page_size = pagination.page_size.unwrap_or(20).min(100).max(1);

    let query_service = match get_qs(&state).await {
        Ok(qs) => qs,
        Err(response) => return response,
    };

    // [MODIFIED] Added "government" and "central_bank" to the match statement.
    let db_agent_type = match agent_type.as_str() {
        "banks" => "Bank",
        "firms" => "Firm",
        "consumers" => "Consumer",
        "government" => "Government",
        "central_bank" => "CentralBank",
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
                        "Government" => "Government".to_string(),
                        "CentralBank" => "Central Bank".to_string(),
                        _ => summary.id.clone(),
                    },
                    agent_type: summary.agent_type,
                    balance_sheet: summary.balance_sheet.unwrap(),
                    decision_model: "QueryService".to_string(),
                })
                .collect();

            let paginated = Paginated {
                items,
                total_items: total_count,
                page,
                page_size,
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
    let mut engine_guard = state.sim_engine.lock().await;
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
    let err = ApiError { code: "DEPRECATED", message: "Use specific market overview endpoints like /api/markets/goods/overview." };
    (StatusCode::GONE, headers, Json(json!({ "error": err })))
}

pub async fn get_agents(
    Path(_kind): Path<String>, State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let err = ApiError { code: "DEPRECATED", message: "Use /agents/{agent_type}/summary instead." };
    (StatusCode::GONE, headers, Json(json!({ "error": err })))
}

pub async fn get_agent_balance_sheet(
    Path(agent_id_str): Path<String>, State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    
    let query_service = match get_qs(&state).await {
        Ok(qs) => qs,
        Err(response) => return response,
    };

    match query_service.get_agent_balance_sheet(&agent_id_str).await {
        Ok(Some(bs)) => {
            #[derive(Serialize)]
            struct BalanceSheetResponse {
                agent_id: String,
                balance_sheet: BalanceSheet,
            }
            let bs_dto = BalanceSheetResponse { agent_id: agent_id_str, balance_sheet: bs };
            (StatusCode::OK, headers, Json(serde_json::to_value(bs_dto).unwrap()))
        }
        Ok(None) => {
            let err = ApiError { code: "NOT_FOUND", message: "Balance sheet not found for this agent at the latest tick." };
            (StatusCode::NOT_FOUND, headers, Json(json!({ "error": err })))
        }
        Err(e) => {
            let err = json!({ "error": format!("Database error: {}", e) });
            (StatusCode::INTERNAL_SERVER_ERROR, headers, Json(err))
        }
    }
}

pub async fn get_employment_contracts(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let err = ApiError { code: "NOT_IMPLEMENTED_CQRS", message: "This query is pending migration to QueryService." };
    (StatusCode::NOT_IMPLEMENTED, headers, Json(json!({ "error": err })))
}

pub async fn get_non_agent_balance_sheets(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let err = ApiError { code: "NOT_IMPLEMENTED_CQRS", message: "This query is pending migration to QueryService." };
    (StatusCode::NOT_IMPLEMENTED, headers, Json(json!({ "error": err })))
}

pub fn http_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/init", post(init_sim))
        .route("/sim/control/tick", post(tick))
        
        .route("/healthz", get(healthz))
        .route("/dashboard", get(get_dashboard))
        .route("/sim/analysis/stats", get(query_stats))
        .route("/agents/{agent_type}/summary", get(get_agents_summary)) // Using :param syntax for clarity
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

        .route("/sim/analysis/history", get(get_simulation_history))
        .route("/sim/analysis/actions", get(get_actions_history))
        .route("/sim/analysis/effects", get(get_effects_history))
        .route("/sim/analysis/a2e/{tick_number}", get(get_actions_to_effects))
        .route("/sim/analysis/tick/{tick_number}", get(get_tick_details))

        .route("/sim/analysis/market-snapshot", get(query_market_snapshot))
        .route("/agents/{agent_type}", get(get_agents))
        .route("/sim/non_agent_balance_sheets", get(get_non_agent_balance_sheets))
        
        .with_state(state)
}