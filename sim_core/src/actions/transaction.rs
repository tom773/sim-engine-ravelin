use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionAction {
    InitiatePayment {
        from: AgentId,
        to: AgentId,
        amount: f64,
        context: TransactionContext,
    },
    PostMarketOrder {
        agent_id: AgentId,
        market_id: Symbol,
        side: Side,
        quantity: f64,
        price: Option<Money>,
        order_type: OrderType,
    },
    PostJobApplication {
        market_id: LabourMarketId,
        application: JobApplication,
    },
    PostJobOffer {
        market_id: LabourMarketId,
        offer: JobOffer,
    },
}

impl TransactionAction {
    pub fn name(&self) -> String {
        match self {
            TransactionAction::InitiatePayment { .. } => "InitiatePayment".to_string(),
            TransactionAction::PostMarketOrder { .. } => "PostMarketOrder".to_string(),
            TransactionAction::PostJobApplication { .. } => "PostJobApplication".to_string(),
            TransactionAction::PostJobOffer { .. } => "PostJobOffer".to_string(),
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            TransactionAction::InitiatePayment { from, .. } => *from,
            TransactionAction::PostMarketOrder { agent_id, .. } => *agent_id,
            TransactionAction::PostJobApplication { application, .. } => application.consumer_id,
            TransactionAction::PostJobOffer { offer, .. } => offer.firm_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionIntention {
    PayWages {
        employer: AgentId,
        employee: AgentId,
        amount: f64,
    },
}

impl TransactionIntention {
    pub fn name(&self) -> &'static str {
        match self {
            TransactionIntention::PayWages { .. } => "PayWages",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            TransactionIntention::PayWages { employer, .. } => *employer,
        }
    }
}