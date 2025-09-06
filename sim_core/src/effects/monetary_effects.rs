pub use serde::{Deserialize, Serialize};
use crate::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MonetaryEffect {
    OpenMarketOperation { operation_type: OMOType, amount: f64 },
    SetPolicyRate { rate_bps: BasisPoints },
    SetReserveRequirement { ratio: f64 },
    ProvideLiquidityAssistance { bank_id: AgentId, amount: f64, collateral: Option<Vec<InstrumentId>> },
}

impl MonetaryEffect {
    pub fn name(&self) -> &'static str {
        match self {
            MonetaryEffect::OpenMarketOperation { .. } => "OpenMarketOperation",
            MonetaryEffect::SetPolicyRate { .. } => "SetPolicyRate",
            MonetaryEffect::SetReserveRequirement { .. } => "SetReserveRequirement",
            MonetaryEffect::ProvideLiquidityAssistance { .. } => "ProvideLiquidityAssistance",
        }
    }
}
impl StateEffectApplicator {
    pub fn apply_central_bank_effect(state: &mut SimState, effect: &MonetaryEffect) -> Result<(), EffectError> {
        match effect {
            MonetaryEffect::SetPolicyRate { rate_bps } => {
                state.financial_system.central_bank.policy_rate_bps = *rate_bps;
                Ok(())
            }
            MonetaryEffect::SetReserveRequirement { ratio } => {
                if *ratio < 0.0 || *ratio > 1.0 {
                    return Err(EffectError::InvalidState(
                        "Reserve requirement must be between 0 and 1".to_string()
                    ));
                }
                state.financial_system.central_bank.reserve_requirement = *ratio;
                Ok(())
            }
            MonetaryEffect::OpenMarketOperation { operation_type, amount } => {
                tracing::event!(
                    tracing::Level::WARN,
                    "UNIMPLEMENTED: Central Bank performing OMO: {:?} for amount: {}",
                    operation_type, amount
                );
                Ok(())
            }
            MonetaryEffect::ProvideLiquidityAssistance { bank_id, amount , collateral} => {
                tracing::event!(
                    tracing::Level::WARN,
                    "UNIMPLEMENTED: Central Bank providing liquidity assistance to bank: {:?} for amount: {}, collateral: {:?}",
                    bank_id, amount, collateral
                );
                Ok(())
            }
        }
    }
}
