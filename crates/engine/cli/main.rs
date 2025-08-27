use axum::routing::get;
use engine::{Scenario, QueryService, SimulationEngine};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;
pub mod debug_ws;
pub mod dto;
pub mod routes;
pub use debug_ws::*;
pub mod market_routes;
pub mod sim_history_routes;
pub use dto::*;
pub use market_routes::*;
pub use routes::*;
pub use sim_history_routes::*;
pub const SCENARIO_TOML: &str = include_str!("../../../config/config.toml");
pub struct AppState {
    epoch: Uuid,
    sim_engine: RwLock<Option<SimulationEngine>>,
    query_service: RwLock<Option<QueryService>>,
    scenario: Scenario,
}
#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let scenario = Scenario::from_toml_str(SCENARIO_TOML).expect("Failed to parse scenario TOML");
    let state = Arc::new(AppState { epoch: Uuid::new_v4(), sim_engine: RwLock::new(None), query_service: RwLock::new(None), scenario });
    let shutdown = CancellationToken::new();
    let http_server_fut = run_http(state.clone(), shutdown.clone());
    tokio::spawn(async {
        if let Err(e) = serve_ws("127.0.0.1:8066").await {
            eprintln!("[debug-ws] server died: {e}");
        }
    });
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("↪ shutting down (ctrl-c)...");
            shutdown.cancel();
        }
        res = http_server_fut => {
            if let Err(e) = res {
                eprintln!("❗️Server error: {:?}", e);
            }
        }
    }
    Ok(())
}
pub async fn run_http(state: Arc<AppState>, shutdown: CancellationToken) -> anyhow::Result<()> {
    if let Ok(query_service) = QueryService::connect().await {
        *state.query_service.write().await = Some(query_service);
        println!("🗄️ Query service connected to SurrealDB");
    } else {
        println!("⚠️ Failed to connect query service to SurrealDB - some endpoints may not work");
    }
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);
    let app = routes::http_router(state).route("/health", get(|| async { "ok" })).layer(cors);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8060").await?;
    println!("🌍 HTTP server listening on http://0.0.0.0:8060");
    axum::serve(listener, app).with_graceful_shutdown(shutdown.cancelled_owned()).await?;
    Ok(())
}
