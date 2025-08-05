use crate::{routes, AppState};
use async_nats::connect;
use futures::stream::StreamExt;
use std::sync::Arc;

pub async fn run_nats_bridge(state: Arc<AppState>) -> Result<(), Box<dyn std::error::Error>> {
    const NATS_URL: &str = "ws://127.0.0.1:8070";

    let client = connect(NATS_URL).await?;
    println!("[NATS] Connected successfully!");

    let mut subscriber = client.subscribe("sim.control.>").await?;
    println!("\n[NATS] Subscribed to 'sim.control.>'");
    println!();
    println!("[NATS] Available commands:\n");
    println!(" > nats req sim.control.init - [Initialize simulation]");
    println!(" > nats req sim.control.tick - [Advance simulation by one tick]");
    println!(" > nats req sim.control.query.state - [Request current simulation state]");
    println!();

    while let Some(msg) = subscriber.next().await {
        let client_clone = client.clone();
        let state_clone = state.clone(); // Clone the Arc, not the state itself
        tokio::spawn(async move {
            routes::handle_message(msg, client_clone, state_clone).await;
        });
    }

    Ok(())
}