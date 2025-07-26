// In crates/engine/src/lib.rs

#[allow(unused)]
pub mod execution;
pub use execution::*;

pub mod state;
pub use state::*;
use serde::{Deserialize, Serialize};
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
                let sim_actions = agent_action_to_sim_actions(&action, &sim_state);
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
        
    }
    
    sim_state
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TickActions {
    pub stage1: AgentActions,
    pub stage2: SimActions,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct AgentActions {
    pub firm_actions: Vec<Action>,
    pub consumer_actions: Vec<Action>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SimActions {
    pub firm_actions: Vec<SimAction>,
    pub consumer_actions: Vec<SimAction>,
}
pub fn tick(sim_state: &mut SimState) -> (&mut SimState, TickActions) {
    sim_state.ticknum += 1;
    let mut rng = StdRng::from_os_rng();
    let firm_actions = sim_state.firms.iter()
        .flat_map(|firm| {
            let decision = firm.decide(&sim_state.financial_system, &mut rng);
            firm.act(&decision)
        })
        .collect::<Vec<_>>();
    let consumer_actions = sim_state.consumers.iter()
        .flat_map(|consumer| {
            let decision = consumer.decide(&sim_state.financial_system, &mut rng);
            consumer.act(&decision)
        })
        .collect::<Vec<_>>();
    let agent_actions = AgentActions {
        firm_actions,
        consumer_actions,
    };
    let firm_sim_actions = agent_actions.firm_actions.iter()
        .flat_map(|action| agent_action_to_sim_actions(action, sim_state))
        .collect::<Vec<_>>();
    let consumer_sim_actions = agent_actions.consumer_actions.iter()
        .flat_map(|action| agent_action_to_sim_actions(action, sim_state))
        .collect::<Vec<_>>();
    let sim_actions = SimActions { firm_actions: firm_sim_actions, consumer_actions: consumer_sim_actions };
     
    return (sim_state, TickActions {
        stage1: agent_actions,
        stage2: sim_actions,
    });
}