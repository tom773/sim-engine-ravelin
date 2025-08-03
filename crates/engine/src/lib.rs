use rand::prelude::*;
use ravelin_traits::*;

pub fn tick<C: Core>(sim_state: &mut C::State) -> (&mut C::State, Vec<C::Action>, Vec<C::Effect>) {
    sim_state.advance_time();
    let mut rng = StdRng::from_os_rng();

    let agents = sim_state.get_agents();
    let all_sim_actions: Vec<_> = agents
        .iter()
        .flat_map(|agent| agent.decide_and_act(sim_state, &mut rng))
        .collect();

    let mut all_effects = Vec::new();
    let registry = sim_state.get_domain_registry().clone();
    for action in &all_sim_actions {
        let result = registry.execute(action, sim_state);
        if result.success {
            all_effects.extend(result.effects);
        } else {
            for error in result.errors {
                println!("Action failed: {} - {}", action.name(), error);
            }
        }
    }

    let settlement_effects = sim_state.clear_markets_and_settle();
    all_effects.extend(settlement_effects);

    if let Err(e) = sim_state.apply_effects(&all_effects) {
        println!("CRITICAL: Error applying effects: {}", e);
    }

    (sim_state, all_sim_actions, all_effects)
}