use axum::{Json, Router, extract::State, http::Method, routing::get};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
mod routes;
use std::path::PathBuf;
use ravelin_traits::*;

// This path is now specific to the core implementation being run.
#[cfg(feature = "cli_ravelin_core")]
fn get_scenario_path() -> PathBuf {
    // This is a bit of a hack. A better solution would be a config file.
    // We assume the binary is run from the workspace root.
    PathBuf::from("crates/ravelin_core/src/state/config/config.toml")
}

#[tokio::main]
async fn main() {
    // We explicitly choose which core implementation to run via features.
    #[cfg(feature = "cli_ravelin_core")]
    {
        println!("Starting server with RavelinCore implementation...");
        run_server::<ravelin_core::RavelinCore>().await;
    }
    #[cfg(not(feature = "cli_ravelin_core"))]
    {
        panic!("No core implementation selected. Please enable a feature like 'cli_ravelin_core'.");
    }
}

async fn run_server<C: Core>()
where
    // Add trait bounds required by Axum state
    C::State: 'static,
    C::Action: 'static,
    C::Effect: 'static,
    C::Scenario: 'static,
    C::DomainRegistry: 'static,
{
    let scenario_path = get_scenario_path();
    let scenario = match C::Scenario::from_file(scenario_path.to_str().unwrap()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("FATAL: Failed to load scenario from '{:?}': {}", scenario_path, e);
            std::process::exit(1);
        }
    };

    println!("Loaded scenario: {}", scenario.name());

    // Correctly create a concrete RNG to pass as a trait object
    let mut rng = StdRng::from_os_rng();
    let ss = C::State::initialize_from_scenario(&scenario, &mut rng);

    let state = Arc::new(RwLock::new(ss));

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        .route("/health", get(|| async { "Welcome to the Ravelin Engine!" }))
        .route("/state", get(get_state::<C>))
        .route("/clear", get(clear_state::<C>))
        .route("/init", get(init::<C>))
        .route("/tick", get(routes::tick::<C>))
        .route("/inject", get(routes::inject::<C>))
        // This endpoint is too specific to ravelin_core, so we remove it from the generic engine
        // .route("/tick_ret_date", get(routes::tick_ret_date::<C>))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8070").await.unwrap();
    println!("Server running on http://127.0.0.1:8070");
    axum::serve(listener, app).await.unwrap();
}

// All handlers are now generic over C: Core
async fn init<C: Core>(State(state): State<Arc<RwLock<C::State>>>) -> Json<String> {
    let scenario_path = get_scenario_path();
    let scenario = match C::Scenario::from_file(scenario_path.to_str().unwrap()) {
        Ok(s) => s,
        Err(e) => return Json(format!("Failed to initialize from scenario: {}", e)),
    };

    let mut state_guard = state.write().await;
    let mut rng = StdRng::from_os_rng();
    *state_guard = C::State::initialize_from_scenario(&scenario, &mut rng);
    Json(format!("State initialized from scenario: {}", scenario.name()))
}

async fn get_state<C: Core>(State(state): State<Arc<RwLock<C::State>>>) -> Json<Res<C::State>> {
    let state_guard = state.read().await;
    Json(Res {
        stats: Some(state_guard.get_stats_json()),
        messages: None,
        state: state_guard.clone(),
    })
}

async fn clear_state<C: Core>(State(state): State<Arc<RwLock<C::State>>>) -> Json<String> {
    let mut state_guard = state.write().await;
    *state_guard = C::State::default();
    Json("State cleared".to_string())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Res<S> {
    pub messages: Option<Vec<String>>,
    pub stats: Option<serde_json::Value>, // Stats are now generic JSON
    pub state: S,
}