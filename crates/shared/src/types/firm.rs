use crate::*;
use serde::{Serialize, Deserialize};
use rand::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Firm {
    pub id: AgentId,
    pub bank_id: AgentId,
    pub name: String,
}

impl Firm {
    pub fn new(id: AgentId, bank_id: AgentId, name: String) -> Self {
        Self {
            id,
            name,
            bank_id,
        }
    }
}

impl Agent for Firm {
    type DecisionType = FirmDecision;
    fn act(&self, decision: &FirmDecision) -> Vec<Action> {
        let mut actions = Vec::new();
        if 1 == 1 { // Placeholder for actual decision logic
            actions.push(Action::Produce { agent_id: self.id.clone(), amount: decision.production_quantity as f64 });
            actions.push(Action::Hire { agent_id: self.id.clone(), count: decision.hiring_count as u32 });
        }
        actions
    }
    
    fn decide(&self, _fs: &FinancialSystem, _rng: &mut StdRng) -> FirmDecision {
        FirmDecision {
            production_quantity: 100,
            hiring_count: 5,
        }
    }
}