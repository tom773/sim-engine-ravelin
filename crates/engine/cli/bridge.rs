// bridge.rs
use crate::{routes, AppState};
use async_nats::connect;
use axum::{routing::get, Router};
use futures::StreamExt;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};

pub async fn run_http(state: Arc<AppState>, shutdown: CancellationToken) -> anyhow::Result<()> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/init", get(routes::handle_init_sim))
        .route("/agents/{agent}", get(routes::get_agents))
        .route("/sim/control/tick", get(routes::tick))
        .route("/sim/control/state", get(routes::query_state))
        .route("/sim/control/markets", get(routes::query_market_snapshot))
        .route("/sim/control/fs", get(routes::query_fs))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8060").await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown.cancelled_owned())
        .await?;
    Ok(())
}

pub async fn run_nats_bridge(state: Arc<AppState>, shutdown: CancellationToken) -> anyhow::Result<()> {
    const NATS_URL: &str = "ws://127.0.0.1:8070"; // requires async-nats "ws" feature
    let client = connect(NATS_URL).await?;
    println!("[NATS] Connected successfully!");

    let mut sub = client.subscribe("sim.control.>").await?;

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                println!("[NATS] shutdown signal received");
                break;
            }
            maybe_msg = sub.next() => {
                match maybe_msg {
                    Some(msg) => {
                        let c = client.clone();
                        let s = state.clone();
                        // don’t .await inside select arm without spawning if it can be slow
                        tokio::spawn(async move {
                            routes::handle_message(msg, c, s).await;
                        });
                    }
                    None => {
                        println!("[NATS] subscription ended");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
