use rand::prelude::*;
use shared::*;
use crossbeam_channel::{bounded};

mod command;
use command::{SimCommand, process_command};
pub mod state;
pub use state::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

pub fn run_simulation() -> SimState {
    let (tx, rx) = bounded::<SimCommand>(100);
    let mut rng = StdRng::from_os_rng();
    let config = SimConfig::default();
    let mut sim_state = initialize_economy(&config, &mut rng);

    for tick in 0..config.iterations {
        sim_state.ticknum = tick + 1;
        println!("Tick {}: Running simulation step...\n", sim_state.ticknum);
        
        for (idx, consumer) in sim_state.consumers.iter().enumerate() {
            let decision = consumer.decide(&sim_state.financial_system, &mut rng);
            let action = consumer.act(&decision);
            
            match action {
                Action::Save => {
                    tx.send(SimCommand::SaveMoney {
                        consumer_idx: idx,
                        amount: consumer.income,
                    }).unwrap();
                }
                Action::Buy { good_id, quantity } => {
                    tx.send(SimCommand::Purchase {
                        consumer_idx: idx,
                        firm_idx: 0, // You'd have logic to select firm
                        good_id,
                        quantity,
                    }).unwrap();
                }
                Action::Sell { .. } => {
                }
            }
        }
        
        while let Ok(command) = rx.try_recv() {
            process_command(&mut sim_state, command);
        }
        println!("\n");
    }
    
    sim_state
}

pub async fn start_simulation_loop(engine_state: Arc<RwLock<SimState>>) {
    let mut rng = StdRng::from_os_rng(); 
    let (tx, rx) = bounded::<SimCommand>(100);

    println!("Simulation loop started.");

    loop {
        let mut state_guard = engine_state.write().await;
        
        let tick_duration_ms = (1000.0/1.0) as u64;

        state_guard.ticknum += 1;
        println!("Tick {}: Running simulation step...", state_guard.ticknum);

        for (idx, consumer) in state_guard.consumers.iter().enumerate() {
            let decision = consumer.decide(&state_guard.financial_system, &mut rng);
            let action = consumer.act(&decision);
            
            match action {
                Action::Save => {
                    let _ = tx.send(SimCommand::SaveMoney {
                        consumer_idx: idx,
                        amount: consumer.income,
                    });
                }
                Action::Buy { good_id, quantity } => {
                    let _ = tx.send(SimCommand::Purchase {
                        consumer_idx: idx,
                        firm_idx: 0, 
                        good_id,
                        quantity,
                    });
                }
                Action::Sell { .. } => {}
            }
        }

        while let Ok(command) = rx.try_recv() {
            process_command(&mut *state_guard, command);
        }

        println!("\n");
        
        sleep(Duration::from_millis(tick_duration_ms)).await;
    }
}