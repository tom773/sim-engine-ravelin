use crate::*;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sim_prelude::*;
use std::collections::HashMap;

pub struct SimulationEngine {
    pub state: SimState,
    pub domain_registry: DomainRegistry,
    pub decision_models: HashMap<AgentId, Box<dyn DecisionModel>>,
}

impl SimulationEngine {
    pub fn new(state: SimState) -> Self {
        Self {
            state,
            domain_registry: DomainRegistry::new(),
            decision_models: HashMap::new(),
        }
    }

    fn collect_actions(&self, rng: &mut dyn RngCore) -> Vec<SimAction> {
        let mut all_actions = Vec::new();

        for agent_id in self.state.agents.all_agent_ids() {
            if let Some(model) = self.decision_models.get(&agent_id) {
                if let Some(agent) = self.state.agents.get_agent_as_any(&agent_id) {
                    all_actions.extend(model.decide(agent, &self.state, rng));
                }
            }
        }
        all_actions
    }

    pub fn tick(&mut self, rng: &mut dyn RngCore) -> TickResult {
        let actions = self.collect_actions(rng);

        let effects = self.execute_actions(&actions);
        if let Err(e) = self.state.apply_effects(&effects) {
            println!("[ERROR] applying action effects: {}", e);
        }

        let trades = self.state.financial_system.exchange.clear_markets();

        let settlement_effects = self.settle_trades(&trades);
        if let Err(e) = self.state.apply_effects(&settlement_effects) {
            println!("[ERROR] applying settlement effects: {}", e);
        }

        self.state.advance_time();

        TickResult {
            tick_number: self.state.ticknum,
            actions_count: actions.len(),
            effects_count: effects.len() + settlement_effects.len(),
            trades_count: trades.len(),
        }
    }

    fn execute_actions(&self, actions: &[SimAction]) -> Vec<StateEffect> {
        let mut all_effects = Vec::new();
        for action in actions {
            let effects = self.domain_registry.execute(action, &self.state);
            all_effects.extend(effects);
        }
        all_effects
    }

    fn settle_trades(&self, trades: &[Trade]) -> Vec<StateEffect> {
        let mut effects = Vec::new();
        for trade in trades {
            effects.push(StateEffect::Market(MarketEffect::ExecuteTrade(trade.clone())));
        }
        effects
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TickResult {
    pub tick_number: u32,
    pub actions_count: usize,
    pub effects_count: usize,
    pub trades_count: usize,
}
