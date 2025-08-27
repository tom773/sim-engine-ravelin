use engine::{Scenario, QueryService, SimulationEngine};
use rand::rngs::ThreadRng;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

pub mod debug_ws;
pub use debug_ws::*;
pub mod market_routes;
pub mod sim_history_routes;
pub use market_routes::*;
pub mod routes;
pub use routes::*;
pub use sim_history_routes::*;

pub const SCENARIO_TOML: &str = include_str!("../../../config/config.toml");

pub struct AppState {
    epoch: Uuid,
    sim_engine: RwLock<Option<SimulationEngine>>,
    query_service: RwLock<Option<QueryService>>,
    scenario: Scenario,
}

impl AppState {
    pub fn new(scenario: Scenario) -> Self {
        Self {
            epoch: Uuid::new_v4(),
            sim_engine: RwLock::new(None),
            query_service: RwLock::new(None),
            scenario,
        }
    }

    pub async fn initialize_query_service(&self) -> Result<(), String> {
        match QueryService::connect().await {
            Ok(qs) => {
                match qs.health_check().await {
                    Ok(true) => {
                        *self.query_service.write().await = Some(qs);
                        println!("🗄️ QueryService connected and healthy");
                        Ok(())
                    }
                    Ok(false) => {
                        println!("⚠️ QueryService connected but not healthy");
                        Err("QueryService health check failed".to_string())
                    }
                    Err(e) => {
                        println!("⚠️ QueryService health check error: {}", e);
                        Err(format!("QueryService health check error: {}", e))
                    }
                }
            }
            Err(e) => {
                println!("⚠️ Failed to connect to QueryService: {}", e);
                Err(format!("Failed to connect to QueryService: {}", e))
            }
        }
    }

    pub async fn query_service_status(&self) -> String {
        if let Some(qs) = self.query_service.read().await.as_ref() {
            match qs.health_check().await {
                Ok(true) => "Connected and healthy".to_string(),
                Ok(false) => "Connected but unhealthy".to_string(),
                Err(e) => format!("Connected but error: {}", e),
            }
        } else {
            "Not connected".to_string()
        }
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let scenario = Scenario::from_toml_str(SCENARIO_TOML)
        .expect("Failed to parse scenario TOML");

    let state = Arc::new(AppState::new(scenario));

    let shutdown = CancellationToken::new();

    println!("\n[ENGINE] Initializing simulation engine...");
    {
        let mut engine_guard = state.sim_engine.write().await;
        let mut engine = state.scenario.initialize_engine();

        if let Err(e) = engine.connect_to_db().await {
            println!("[ERROR] Failed to connect to SurrealDB during init: {}", e);
        }

        println!("[ENGINE] Performing initial tick to populate data...");
        let mut rng = ThreadRng::default();
        let result = engine.tick_with_scheduler(&mut rng);
        println!("[ENGINE] Initial tick completed. Tick number: {}\n", result.tick_number);

        *engine_guard = Some(engine);
    }
    println!("[ENGINE] Simulation engine is live.");


    println!("\n[DB] Initializing CQRS Query Service...");
    if let Err(e) = state.initialize_query_service().await {
        println!("[DB] QueryService initialization failed: {}", e);
        println!("[DB] Application will continue with legacy mode only");
    } else {
        println!("[DB] QueryService initialization successful");
    }

    let http_server_fut = run_http(state.clone(), shutdown.clone());

    tokio::spawn(async {
        if let Err(e) = serve_ws("127.0.0.1:8066").await {
            eprintln!("[debug-ws] server died: {e}");
        }
    });

    let monitoring_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let status = monitoring_state.query_service_status().await;
            println!("[MONITOR] QueryService status: {}", status);
        }
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("[BYE] Shutting down (ctrl-c)...");
            shutdown.cancel();
        }
        res = http_server_fut => {
            if let Err(e) = res {
                eprintln!("[WTF] Server error: {:?}", e);
            }
        }
    }

    Ok(())
}

pub async fn run_http(state: Arc<AppState>, shutdown: CancellationToken) -> anyhow::Result<()> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = routes::http_router(state.clone())
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8060").await?;
    println!("[ENGINE] HTTP server listening on http://0.0.0.0:8060");
    println!("[ENGINE] Dashboard: http://localhost:8060/dashboard");
    println!("[DB] QueryService health: http://localhost:8060/health/query-service");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown.cancelled_owned())
        .await?;

    Ok(())
}