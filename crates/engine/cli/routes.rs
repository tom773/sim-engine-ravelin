use crate::AppState;
use async_nats::{Client, Message};
use rand::rngs::ThreadRng;
use serde_json::json;
use std::sync::Arc;
use axum::{extract::{State, Path}, Json};

pub async fn handle_message(msg: Message, client: Client, state: Arc<AppState>) {
    println!("[NATS] Received message on '{}'", msg.subject);

    let response = match msg.subject.as_ref() {
        //"sim.control.init" => handle_init_sim(&state),
        "sim.control.tick" => handle_tick(&state),
        "sim.control.query.state" => handle_req_state(&state),
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

fn handle_tick(state: &Arc<AppState>) -> Result<String, String> {
    println!("[SIMCTL] Received TICK command.");
    let mut engine_guard = state.sim_engine.lock().unwrap();

    if let Some(engine) = engine_guard.as_mut() {
        let mut rng = ThreadRng::default();
        let result = engine.tick(&mut rng);
        println!("[SIMCTL] Tick {} completed.", result.tick_number);
        Ok(serde_json::to_string(&result).map_err(|e| e.to_string())?)
    } else {
        Err("Simulation not initialized. Send 'init' command first.".to_string())
    }
}

fn handle_req_state(state: &Arc<AppState>) -> Result<String, String> {
    println!("[SIMCTL] Received QUERY STATE command");
    let engine_guard = state.sim_engine.lock().unwrap();

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
) -> Json<serde_json::Value> {
    let guard = state.sim_engine.lock().unwrap();

    if let Some(engine) = guard.as_ref() {
        let body = match kind.as_str() {
            "banks" => json!({ "banks": engine.state.agents.banks.values().cloned().collect::<Vec<_>>() }),
            "firms" => json!({ "firms": engine.state.agents.firms.values().cloned().collect::<Vec<_>>() }),
            "consumers" => json!({ "consumers": engine.state.agents.consumers.values().cloned().collect::<Vec<_>>() }),
            _ => json!({ "error": format!("Unknown agent type: {}", kind) }),
        };
        Json(body)
    } else {
        Json(json!({ "error": "Simulation not initialized" }))
    }
}

pub async fn handle_init_sim(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let mut engine_guard = state.sim_engine.lock().unwrap();
    
    let engine = state.scenario.initialize_engine();
    *engine_guard = Some(engine);
    Json(json!({ "status": "Simulation initialized successfully" }))
}

pub async fn tick(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let mut engine_guard = state.sim_engine.lock().unwrap();

    if let Some(engine) = engine_guard.as_mut() {
        let mut rng = ThreadRng::default();
        let result = engine.tick(&mut rng);
        Json(json!({ "status": "Tick completed", "tick_number": result.tick_number }))
    } else {
        Json(json!({ "error": "Simulation not initialized. Send 'init' command first." }))
    }
}

pub async fn query_state(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let engine_guard = state.sim_engine.lock().unwrap();

    if let Some(engine) = engine_guard.as_ref() {
        let state_json = serde_json::to_string(&engine.state).unwrap_or_else(|_| "Error serializing state".to_string());
        Json(json!({ "state": state_json }))
    } else {
        Json(json!({ "error": "Simulation not initialized. Send 'init' command first." }))
    }
}

pub async fn query_market_snapshot(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let engine_guard = state.sim_engine.lock().unwrap();

    if let Some(engine) = engine_guard.as_ref() {
        let snapshot = engine.state.all_market_views();
        Json(json!({ "market_snapshot": snapshot }))
    } else {
        Json(json!({ "error": "Simulation not initialized. Send 'init' command first." }))
    }
}

pub async fn query_fs(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let engine_guard = state.sim_engine.lock().unwrap();

    if let Some(engine) = engine_guard.as_ref() {
        let fs_data = engine.state.financial_system.clone();
        Json(json!({ "fs": fs_data }))
    } else {
        Json(json!({ "error": "Simulation not initialized. Send 'init' command first." }))
    }
}