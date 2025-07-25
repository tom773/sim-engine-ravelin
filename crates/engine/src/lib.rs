// In crates/engine/src/lib.rs

#[allow(unused)]
pub mod execution;
pub use execution::{TransactionExecutor, agent_action_to_sim_actions};

pub mod state;
pub use state::*;

pub use shared::*;
use rand::prelude::*;

pub fn run_simulation() -> SimState {
    let mut rng = StdRng::from_os_rng();
    let config = SimConfig::default();
    let mut sim_state = initialize_economy(&config, &mut rng);

    for tick in 0..config.iterations {
        sim_state.ticknum = tick + 1;
        println!("\n=== Tick {} ===", sim_state.ticknum);
        
        // Collect all actions from agents
        let mut all_sim_actions = Vec::new();
        
        // Consumer actions
        for consumer in &sim_state.consumers {
            let decision = consumer.decide(&sim_state.financial_system, &mut rng);
            println!("Consumer {} decided to spend ${:.2} and save ${:.2}", 
                consumer.id.0, decision.spend_amount, decision.save_amount);
            
            let actions = consumer.act(&decision);
            
            // Convert each action to sim actions
            for action in actions {
                let sim_actions = agent_action_to_sim_actions(&consumer.id, &action, &sim_state);
                all_sim_actions.extend(sim_actions);
            }
        }
        
        // Execute all actions
        for action in &all_sim_actions {
            println!("Executing: {:?}", action);
            let result = TransactionExecutor::execute_action(action, &sim_state);
            
            if result.success {
                // Apply effects to state
                if let Err(e) = TransactionExecutor::apply_effects(&result.effects, &mut sim_state) {
                    eprintln!("Error applying effects: {}", e);
                }
            } else {
                eprintln!("Action failed: {:?}", result.errors);
            }
        }
        
        // Print summary
        print_tick_summary(&sim_state);
    }
    
    sim_state
}

fn print_tick_summary(state: &SimState) {
    println!("\n--- End of Tick {} Summary ---", state.ticknum);
    
    // Bank summary
    for (bank_id, bank) in &state.financial_system.commercial_banks {
        let assets = bank.total_assets(&state.financial_system);
        let liabilities = bank.total_liabilities(&state.financial_system);
        
        let reserves = state.financial_system.balance_sheets
            .get(bank_id)
            .map(|bs| bs.assets.values()
                .filter(|inst| matches!(inst.instrument_type, InstrumentType::CentralBankReserves))
                .map(|inst| inst.principal)
                .sum::<f64>()
            )
            .unwrap_or(0.0);
            
        let cash = state.financial_system.balance_sheets
            .get(bank_id)
            .map(|bs| bs.assets.values()
                .filter(|inst| matches!(inst.instrument_type, InstrumentType::Cash))
                .map(|inst| inst.principal)
                .sum::<f64>()
            )
            .unwrap_or(0.0);
            
        println!("{}: Assets=${:.2}, Liabilities=${:.2}, Equity=${:.2}, Cash=${:.2}, CB Reserves=${:.2}",
            bank.name, assets, liabilities, assets - liabilities, cash, reserves);
    }
    
    // Consumer summary (aggregate)
    let total_consumer_cash: f64 = state.consumers.iter()
        .map(|c| c.get_cash_holdings(&state.financial_system))
        .sum();
        
    let total_consumer_deposits: f64 = state.consumers.iter()
        .map(|c| c.get_deposits(&state.financial_system))
        .sum();
        
    let total_consumer_assets: f64 = state.consumers.iter()
        .map(|c| state.financial_system.get_total_assets(&c.id))
        .sum();
        
    let total_consumer_liabilities: f64 = state.consumers.iter()
        .map(|c| state.financial_system.get_total_liabilities(&c.id))
        .sum();
    
    println!("Consumers: Cash=${:.2}, Deposits=${:.2}, Total Assets=${:.2}, Liabilities=${:.2}, Net Worth=${:.2}",
        total_consumer_cash, total_consumer_deposits, total_consumer_assets, total_consumer_liabilities, 
        total_consumer_assets - total_consumer_liabilities);
}