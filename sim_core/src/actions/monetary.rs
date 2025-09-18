use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MonetaryAction {
    OpenMarketOperation { cb_id: AgentId, operation_type: OMOType, amount: f64 },
    SetPolicyRate { cb_id: AgentId, rate_bps: BasisPoints },
    SetReserveRequirement { cb_id: AgentId, ratio: f64 },
    ProvideLiquidityAssistance { cb_id: AgentId, bank_id: AgentId, amount: f64, collateral: Option<Vec<InstrumentId>> },
}

impl MonetaryAction {
    pub fn name(&self) -> &'static str {
        match self {
            MonetaryAction::OpenMarketOperation { .. } => "OpenMarketOperation",
            MonetaryAction::SetPolicyRate { .. } => "SetPolicyRate",
            MonetaryAction::SetReserveRequirement { .. } => "SetReserveRequirement",
            MonetaryAction::ProvideLiquidityAssistance { .. } => "ProvideLiquidityAssistance",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            MonetaryAction::OpenMarketOperation { cb_id, .. } => *cb_id,
            MonetaryAction::SetPolicyRate { cb_id, .. } => *cb_id,
            MonetaryAction::SetReserveRequirement { cb_id, .. } => *cb_id,
            MonetaryAction::ProvideLiquidityAssistance { cb_id, .. } => *cb_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy, PartialEq)]
pub enum OMOType {
    QuantitativeEasing,     // CB buys securities to expand balance sheet (injects liquidity)
    QuantitativeTightening, // CB sells securities to shrink balance sheet (drains liquidity)
    Repo { term_days: u32, rate_bps: BasisPoints },
    ReverseRepo { term_days: u32, rate_bps: BasisPoints },
}

impl OMOType {
    pub fn description(&self) -> String {
        match self {
            OMOType::QuantitativeEasing => "Open Market Purchase".to_string(),
            OMOType::QuantitativeTightening => "Open Market Sale".to_string(),
            OMOType::Repo { term_days, rate_bps } => {
                format!("Repurchase Agreement: {} days at {} bps", term_days, rate_bps)
            }
            OMOType::ReverseRepo { term_days, rate_bps } => {
                format!("Reverse Repurchase Agreement: {} days at {} bps", term_days, rate_bps)
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MonetaryIntention {
    ConductOMO { cb_id: AgentId, operation_type: OMOType, amount: f64 },
    SetPolicyRate { cb_id: AgentId, new_rate_bps: BasisPoints },
    AdjustReserveRequirement { cb_id: AgentId, new_ratio: f64 },
    ProvideLiquidityFacility { cb_id: AgentId, bank_id: AgentId, amount: f64, collateral: Option<Vec<InstrumentId>> },
}

impl MonetaryIntention {
    pub fn name(&self) -> &'static str {
        match self {
            MonetaryIntention::ConductOMO { .. } => "ConductOMO",
            MonetaryIntention::SetPolicyRate { .. } => "SetPolicyRate",
            MonetaryIntention::AdjustReserveRequirement { .. } => "AdjustReserveRequirement",
            MonetaryIntention::ProvideLiquidityFacility { .. } => "ProvideLiquidityFacility",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            MonetaryIntention::ConductOMO { cb_id, .. } => *cb_id,
            MonetaryIntention::SetPolicyRate { cb_id, .. } => *cb_id,
            MonetaryIntention::AdjustReserveRequirement { cb_id, .. } => *cb_id,
            MonetaryIntention::ProvideLiquidityFacility { cb_id, .. } => *cb_id,
        }
    }
}
