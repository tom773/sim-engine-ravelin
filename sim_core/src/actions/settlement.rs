use serde::{Deserialize, Serialize};
use crate::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SettlementAction {
    AccrueInterest { instrument_id: InstrumentId },
    PayInterest { instrument_id: InstrumentId },
    ProcessCouponPayment { instrument_id: InstrumentId },
}

impl SettlementAction {
    pub fn name(&self) -> &'static str {
        match self {
            SettlementAction::AccrueInterest { .. } => "AccrueInterest",
            SettlementAction::PayInterest { .. } => "PayInterest",
            SettlementAction::ProcessCouponPayment { .. } => "ProcessCouponPayment",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        AgentId::default()
    }
}