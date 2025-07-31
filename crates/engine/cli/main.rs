use axum::{Json, Router, extract::State, http::Method, routing::get};
#[allow(unused)]
use engine::*;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
mod routes;
use std::path::PathBuf;

fn get_scenario_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut path = PathBuf::from(manifest_dir);
    path.push("src/state/config/config.toml"); // Go up one level from `cli` to `engine` root
    path
}

#[tokio::main]
async fn main() {
    let scenario_path = get_scenario_path();
    let scenario = match Scenario::from_file(scenario_path.to_str().unwrap()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("FATAL: Failed to load scenario from '{:?}': {}", scenario_path, e);
            std::process::exit(1);
        }
    };

    println!("Loaded scenario: {}", scenario.name);

    let ss = initialize_economy_from_scenario(&scenario, &mut StdRng::from_os_rng());

    let state = Arc::new(RwLock::new(ss));

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        .route("/health", get(|| async { "Welcome to the Ravelin Engine!" }))
        .route("/state", get(get_state))
        .route("/clear", get(clear_state))
        .route("/init", get(init))
        .route("/tick", get(routes::tick))
        .route("/inject", get(routes::inject))
        .route("/tick_ret_date", get(routes::tick_ret_date))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8070").await.unwrap();

    println!("Server running on http://127.0.0.1:8070");
    println!("⚠️  CORS is configured to allow ANY origin - development only!");

    axum::serve(listener, app).await.unwrap();
}
async fn init(State(state): State<Arc<RwLock<SimState>>>) -> Json<String> {
    let scenario_path = get_scenario_path();
    let scenario = match state::Scenario::from_file(scenario_path.to_str().unwrap()) {
        Ok(s) => s,
        Err(e) => {
            return Json(format!("Failed to initialize from scenario: {}", e));
        }
    };

    let mut state_guard = state.write().await;
    *state_guard = initialize_economy_from_scenario(&scenario, &mut StdRng::from_os_rng());
    Json(format!("State initialized from scenario: {}", scenario.name))
}
async fn get_state(State(state): State<Arc<RwLock<SimState>>>) -> Json<Res> {
    let state_guard = state.read().await;
    Json(Res {
        stats: Some(Stats {
            m0: state_guard.financial_system.m0(),
            m1: state_guard.financial_system.m1(),
            m2: state_guard.financial_system.m2(),
        }),
        messages: None,
        state: state_guard.clone(),
    })
}

async fn clear_state(State(state): State<Arc<RwLock<SimState>>>) -> Json<String> {
    let mut state_guard = state.write().await;
    *state_guard = SimState::default();
    Json("State cleared".to_string())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Res {
    pub messages: Option<Vec<String>>,
    pub stats: Option<Stats>,
    pub state: SimState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Stats {
    pub m0: f64,
    pub m1: f64,
    pub m2: f64,
}
