use axum::{
    extract::{State},
    response::Json,
};
use engine::*;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn tick(State(state): State<Arc<RwLock<SimState>>>) -> Json<String> {
    let mut state_guard = state.write().await;
    let (_ss, actions) = engine::tick(&mut state_guard);
    Json(json!(actions).to_string())
}