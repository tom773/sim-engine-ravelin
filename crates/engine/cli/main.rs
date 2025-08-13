use crate::bridge::{run_http, run_nats_bridge};
use engine::{Scenario, SimulationEngine};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

mod bridge;
mod routes;

pub const SCENARIO_TOML: &str = include_str!("../../../config/config.toml");

pub struct AppState {
    sim_engine: Mutex<Option<SimulationEngine>>,
    scenario: Scenario,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let scenario = Scenario::from_toml_str(SCENARIO_TOML).expect("Failed to parse scenario TOML");

    let state = Arc::new(AppState { sim_engine: Mutex::new(None), scenario });

    let shutdown = CancellationToken::new();

    let http_fut = run_http(state.clone(), shutdown.clone());
    let nats_fut = run_nats_bridge(state.clone(), shutdown.clone());

    tokio::pin!(http_fut);
    tokio::pin!(nats_fut);

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            eprintln!("↪ shutting down (ctrl-c)...");
            shutdown.cancel();
        }
        res = &mut http_fut => {
            eprintln!("http finished: {:?}", res);
            shutdown.cancel();
        }
        res = &mut nats_fut => {
            eprintln!("nats finished: {:?}", res);
            shutdown.cancel();
        }
    }

    Ok(())
}
