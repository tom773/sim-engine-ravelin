use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProductionAction {
    Produce { agent_id: AgentId, recipe_id: RecipeId, batches: u32 },
    Fire { agent_id: AgentId, employee_id: AgentId },
}

impl ProductionAction {
    pub fn name(&self) -> &'static str {
        match self {
            ProductionAction::Produce { .. } => "Produce",
            ProductionAction::Fire { .. } => "Fire",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            ProductionAction::Produce { agent_id, .. } => *agent_id,
            ProductionAction::Fire { agent_id, .. } => *agent_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProductionIntention {
    PurchaseInputs {
        agent_id: AgentId,
        good_id: GoodId,
        quantity: f64,
        max_price: f64,
    },
    PostGoodToMarket {
        agent_id: AgentId,
        good_id: GoodId,
        quantity: f64,
        ask_price: f64,
    },
    Produce {
        agent_id: AgentId,
        recipe_id: RecipeId,
        batches: u32,
    },
    ApplyForJob {
        agent_id: AgentId,
        market_id: LabourMarketId,
        application: JobApplication,
    },
    HireWorkers {
        agent_id: AgentId,
        count: u32,
        wage_rate: f64,
        max_wage: f64,
    },
    FireWorkers {
        agent_id: AgentId,
        employee_ids: Vec<AgentId>,
    },
}

impl ProductionIntention {
    pub fn name(&self) -> &'static str {
        match self {
            ProductionIntention::PurchaseInputs { .. } => "PurchaseInputs",
            ProductionIntention::PostGoodToMarket { .. } => "PostGoodToMarket",
            ProductionIntention::Produce { .. } => "Produce",
            ProductionIntention::ApplyForJob { .. } => "ApplyForJob",
            ProductionIntention::HireWorkers { .. } => "HireWorkers",
            ProductionIntention::FireWorkers { .. } => "FireWorkers",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            ProductionIntention::PurchaseInputs { agent_id, .. } => *agent_id,
            ProductionIntention::PostGoodToMarket { agent_id, .. } => *agent_id,
            ProductionIntention::Produce { agent_id, .. } => *agent_id,
            ProductionIntention::ApplyForJob { agent_id, .. } => *agent_id,
            ProductionIntention::HireWorkers { agent_id, .. } => *agent_id,
            ProductionIntention::FireWorkers { agent_id, .. } => *agent_id,
        }
    }
}