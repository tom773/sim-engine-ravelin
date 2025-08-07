use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LabourAction {
    ApplyForJob {
        market_id: LabourMarketId,
        application: JobApplication,
    },
    PostJobOffer {
        market_id: LabourMarketId,
        offer: JobOffer,
    },
    ClearLabourMarket {
        market_id: LabourMarketId,
    },
    Fire { firm_id: AgentId, employee_id: AgentId },
}

impl LabourAction {
    pub fn name(&self) -> &'static str {
        match self {
            LabourAction::ApplyForJob { .. } => "ApplyForJob",
            LabourAction::PostJobOffer { .. } => "PostJobOffer",
            LabourAction::ClearLabourMarket { .. } => "ClearLabourMarket",
            LabourAction::Fire { .. } => "Fire",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            LabourAction::ApplyForJob { application, .. } => application.consumer_id,
            LabourAction::PostJobOffer { offer, .. } => offer.firm_id,
            LabourAction::ClearLabourMarket { .. } => AgentId::default(), // System action
            LabourAction::Fire { firm_id, .. } => *firm_id,
        }
    }
}