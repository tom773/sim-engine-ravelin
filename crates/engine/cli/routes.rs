use crate::AppState;
use async_nats::{Client, Message};
use engine::*;
use rand::rngs::ThreadRng;
use serde_json::json;
use std::sync::Arc;

pub async fn handle_message(msg: Message, client: Client, state: Arc<AppState>) {
    println!("[NATS] Received message on '{}'", msg.subject);

    let response = match msg.subject.as_ref() {
        "sim.control.init" => handle_init_sim(&state),
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

fn handle_init_sim(state: &Arc<AppState>) -> Result<String, String> {
    println!("[SIMCTL] Received INIT command.");
    let mut engine_guard = state.sim_engine.lock().unwrap();
    
    let sim_state = state.scenario.initialize_state();
    *engine_guard = Some(SimulationEngine::new(sim_state));

    println!("[SIMCTL] Simulation Initialized.");
    Ok(json!({ "status": "ok", "message": "Simulation initialized successfully." }).to_string())
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