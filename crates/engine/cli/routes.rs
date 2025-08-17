use crate::AppState;
use async_nats::{Client, Message};
use rand::rngs::ThreadRng;
use serde_json::json;
use std::sync::Arc;
use axum::{
    extract::{State, Path},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;
use engine::SimulationEngine;

#[derive(Serialize)]
pub struct Health { initialized: bool, epoch: String }

#[derive(Serialize)]
struct ApiError { code: &'static str, message: &'static str }

#[derive(Serialize)]
pub struct InitResponse { epoch: String }

fn with_epoch_header(mut headers: HeaderMap, epoch: Uuid) -> HeaderMap {
    headers.insert("X-Server-Epoch", epoch.to_string().parse().unwrap());
    headers
}

// GET /healthz
pub async fn healthz(State(state): State<Arc<AppState>>) -> (HeaderMap, Json<Health>) {
    let initialized = state.sim_engine.read().await.is_some();
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    (headers, Json(Health { initialized, epoch: state.epoch.to_string() }))
}

// POST /init  (idempotent)
pub async fn init_sim(State(state): State<Arc<AppState>>)
-> (StatusCode, HeaderMap, Json<InitResponse>) {
    let mut guard = state.sim_engine.write().await;
    if guard.is_none() {
        let engine = state.scenario.initialize_engine();
        *guard = Some(engine);
    }
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    (StatusCode::OK, headers, Json(InitResponse { epoch: state.epoch.to_string() }))
}

pub async fn _require_sim<F, R>(
    State(state): State<Arc<AppState>>,
    handler: F,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>)
where
    F: FnOnce(&SimulationEngine) -> R,
    R: serde::Serialize,
{
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    if let Some(sim) = state.sim_engine.read().await.as_ref() {
        let payload = serde_json::to_value(handler(sim)).unwrap_or(serde_json::json!({}));
        return (StatusCode::OK, headers, Json(payload));
    }
    let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
    (StatusCode::CONFLICT, headers, Json(serde_json::json!({ "error": err })))
}

pub fn http_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/init", post(init_sim))
        .route("/agents/{agent}", get(get_agents))
        .route("/sim/control/tick", post(tick))
        .route("/sim/control/state", get(query_state))
        .route("/sim/control/markets", get(query_market_snapshot))
        .route("/sim/control/fs", get(query_fs))
        .route("/sim/analysis/stats", get(query_stats))
        .with_state(state)
}

pub async fn handle_message(msg: Message, client: Client, state: Arc<AppState>) {
    println!("[NATS] Received message on '{}'", msg.subject);

    let response = match msg.subject.as_ref() {
        //"sim.control.init" => handle_init_sim(&state),
        "sim.control.tick" => handle_tick(&state).await,
        "sim.control.query.state" => handle_req_state(&state).await,
        _ => {
            let error_msg = format!("[NATS] No handler for subject: {}", msg.subject);
            println!("{}", error_msg);
            Err(error_msg)
        }
    };

    if let Some(reply) = msg.reply {
        let payload = match response {
            Ok(data) => data,
            Err(e) => json!({ "status": "error", "message": e }).to_string(),
        };
        client.publish(reply, payload.into()).await.ok();
    }
}

async fn handle_tick(state: &Arc<AppState>) -> Result<String, String> {
    println!("[SIMCTL] Received TICK command.");
    let mut engine_guard = state.sim_engine.write().await;

    if let Some(engine) = engine_guard.as_mut() {
        let mut rng = ThreadRng::default();
        let result = engine.tick(&mut rng);
        println!("[SIMCTL] Tick {} completed.", result.tick_number);
        Ok(serde_json::to_string(&result).map_err(|e| e.to_string())?)
    } else {
        Err("Simulation not initialized. Send 'init' command first.".to_string())
    }
}

async fn handle_req_state(state: &Arc<AppState>) -> Result<String, String> {
    println!("[SIMCTL] Received QUERY STATE command");
    let engine_guard = state.sim_engine.read().await;

    if let Some(engine) = engine_guard.as_ref() {
        let state_json = serde_json::to_string(&engine.state).map_err(|e| e.to_string())?;
        Ok(state_json)
    } else {
        Err("Simulation not initialized. Send 'init' command first.".to_string())
    }
}

pub async fn get_agents(
    Path(kind): Path<String>,
    State(state): State<Arc<AppState>>,
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
        (StatusCode::CONFLICT, headers, Json(serde_json::json!({ "error": err })))
    }
}

pub async fn tick(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let mut engine_guard = state.sim_engine.write().await;

    if let Some(engine) = engine_guard.as_mut() {
        let mut rng = ThreadRng::default();
        let result = engine.tick(&mut rng);
        (StatusCode::OK, headers, Json(json!({ "status": "Tick completed", "tick_number": result.tick_number })))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(serde_json::json!({ "error": err })))
    }
}

pub async fn query_state(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;

    if let Some(engine) = engine_guard.as_ref() {
        (StatusCode::OK, headers, Json(json!({ "state": &engine.state })))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(serde_json::json!({ "error": err })))
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

pub async fn query_fs(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;

    if let Some(engine) = engine_guard.as_ref() {
        let fs_data = engine.state.financial_system.clone();
        (StatusCode::OK, headers, Json(json!({ "fs": fs_data })))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(serde_json::json!({ "error": err })))
    }
}

pub async fn query_stats(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
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