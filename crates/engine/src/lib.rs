#[allow(unused)]
pub mod execution;
pub use execution::*;
pub mod tests;

pub mod state;
pub use state::*;
pub use shared::*;
use rand::prelude::*;

pub fn tick(sim_state: &mut SimState) -> (&mut SimState, Vec<SimAction>, Vec<StateEffect>) {
    sim_state.ticknum += 1;
    let mut rng = StdRng::from_os_rng();
    
    let mut all_sim_actions = Vec::new();
    
    for firm in &sim_state.firms {
        let decisions = firm.decide(&sim_state.financial_system, &mut rng);
        let actions = firm.act(&decisions);
        all_sim_actions.extend(actions);
    }
    
    for consumer in &sim_state.consumers {
        let decisions = consumer.decide(&sim_state.financial_system, &mut rng);
        let actions = consumer.act(&decisions);
        all_sim_actions.extend(actions);
    }
    
    let mut all_effects = Vec::new();
    for action in &all_sim_actions {
        let result = TransactionExecutor::execute_action(action, sim_state);
        if result.success {
            all_effects.extend(result.effects);
        } else {
            for error in result.errors {
                println!("Action failed: {} - {}", action.name(), error);
            }
        }
    }
    
    if let Err(e) = TransactionExecutor::apply_effects(&all_effects, sim_state) {
        println!("Error applying effects: {}", e);
    }
    
    (sim_state, all_sim_actions.clone(), all_effects.clone())
}

pub fn inject_liquidity(ss: &mut SimState) -> &mut SimState {
    let action = SimAction::InjectLiquidity;
    let ns = TransactionExecutor::execute_action(&action, ss);
    println!("Injecting liquidity: {:?}", ns.effects);
    if let Err(e) = TransactionExecutor::apply_effects(&ns.effects, ss) {
        println!("Error applying effects: {}", e);
    }
    ss
}