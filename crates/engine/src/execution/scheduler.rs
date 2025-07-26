use shared::*;
use rand::prelude::*;
use crate::{
    state::SimState,
    execution::*,
};
pub fn collect_actions_firms(
    agents: &[Box<dyn Agent <DecisionType = FirmDecision>>],
    fs: &FinancialSystem,
    rng: &mut StdRng,
) -> Vec<Action> {
    let mut actions = Vec::new();
    
    for agent in agents {
        let decision = agent.decide(fs, rng);
        let agent_actions = agent.act(&decision);
        
        for action in agent_actions {
            actions.push(action);
        }
    }
    
    actions
}

pub fn collect_actions_consumers(
    agents: &[Box<dyn Agent <DecisionType = ConsumerDecision>>],
    fs: &FinancialSystem,
    rng: &mut StdRng,
) -> Vec<Action> {
    let mut actions = Vec::new();
    
    for agent in agents {
        let decision = agent.decide(fs, rng);
        let agent_actions = agent.act(&decision);
        
        for action in agent_actions {
            actions.push(action);
        }
    }
    
    actions
}