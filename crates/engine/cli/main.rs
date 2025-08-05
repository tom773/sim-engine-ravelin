use crate::bridge::run_nats_bridge;
use engine::{SimulationEngine, Scenario};
use std::sync::{Arc, Mutex};

mod bridge;
mod routes;

pub const SCENARIO_TOML: &str = include_str!("../../../config/config.toml");

pub struct AppState {
    sim_engine: Mutex<Option<SimulationEngine>>,
    scenario: Scenario,
}

#[tokio::main]
async fn main() {
    let scenario = Scenario::from_toml_str(SCENARIO_TOML)
        .expect("Failed to parse scenario TOML");

    let app_state = Arc::new(AppState {
        sim_engine: Mutex::new(None),
        scenario,
    });
    
    run_nats_bridge(app_state)
        .await
        .expect("[NATS] Failed to run NATS bridge");
}