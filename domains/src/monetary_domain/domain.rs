// monetary_domain/domain.rs
use crate::{Any, Domain, DomainResult, ResolutionContext, ResolutionPhase, ResolutionResult, inventory};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use sim_core::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonetaryDomain {}

impl MonetaryDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for MonetaryDomain {
    fn name(&self) -> &'static str {
        "Monetary"
    }

    fn resolve_intention(&self, intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::Monetary(MonetaryIntention::ConductOMO { cb_id, operation_type, amount }) => {
                vec![SimAction::Monetary(MonetaryAction::OpenMarketOperation {
                    cb_id: *cb_id,
                    operation_type: *operation_type,
                    amount: *amount,
                })]
            }
            SimIntention::Monetary(MonetaryIntention::SetPolicyRate { cb_id, new_rate_bps }) => {
                vec![SimAction::Monetary(MonetaryAction::SetPolicyRate { cb_id: *cb_id, rate_bps: *new_rate_bps })]
            }
            SimIntention::Monetary(MonetaryIntention::AdjustReserveRequirement { cb_id, new_ratio }) => {
                vec![SimAction::Monetary(MonetaryAction::SetReserveRequirement { cb_id: *cb_id, ratio: *new_ratio })]
            }
            SimIntention::Monetary(MonetaryIntention::ProvideLiquidityFacility {
                cb_id,
                bank_id,
                amount,
                collateral,
            }) => {
                vec![SimAction::Monetary(MonetaryAction::ProvideLiquidityAssistance {
                    cb_id: *cb_id,
                    bank_id: *bank_id,
                    amount: *amount,
                    collateral: collateral.clone(),
                })]
            }
            _ => return None,
        };
        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::Monetary(MonetaryIntention::ConductOMO { .. }) => Some(ResolutionPhase::Market),
            SimIntention::Monetary(MonetaryIntention::SetPolicyRate { .. })
            | SimIntention::Monetary(MonetaryIntention::AdjustReserveRequirement { .. }) => {
                Some(ResolutionPhase::Independent)
            }
            SimIntention::Monetary(MonetaryIntention::ProvideLiquidityFacility { .. }) => {
                Some(ResolutionPhase::Independent)
            }
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let monetary_action = match action {
            SimAction::Monetary(action) => action,
            _ => return DomainResult::failure(vec!["Not a monetary action".to_string()]),
        };

        if let Err(error) = self.validate(monetary_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match monetary_action {
            MonetaryAction::OpenMarketOperation { cb_id, operation_type, amount } => {
                self.execute_omo(*cb_id, *operation_type, *amount, state)
            }
            MonetaryAction::SetPolicyRate { cb_id, rate_bps } => self.execute_rate_change(*cb_id, *rate_bps, state),
            MonetaryAction::SetReserveRequirement { cb_id, ratio } => {
                self.execute_reserve_requirement_change(*cb_id, *ratio, state)
            }
            MonetaryAction::ProvideLiquidityAssistance { cb_id, bank_id, amount, collateral } => {
                self.execute_liquidity_assistance(*cb_id, *bank_id, *amount, collateral, state)
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl MonetaryDomain {
    fn validate(&self, action: &MonetaryAction, state: &SimState) -> Result<(), String> {
        match action {
            MonetaryAction::OpenMarketOperation { cb_id, amount, .. } => {
                if *cb_id != state.financial_system.central_bank.id {
                    return Err("Invalid central bank ID".to_string());
                }
                Validator::positive_amount(*amount)
            }
            MonetaryAction::SetPolicyRate { cb_id, rate_bps } => {
                if *cb_id != state.financial_system.central_bank.id {
                    return Err("Invalid central bank ID".to_string());
                }
                if *rate_bps < dec!(0) || *rate_bps > dec!(10000) {
                    return Err("Policy rate must be between 0 and 10000 bps".to_string());
                }
                Ok(())
            }
            MonetaryAction::SetReserveRequirement { cb_id, ratio } => {
                if *cb_id != state.financial_system.central_bank.id {
                    return Err("Invalid central bank ID".to_string());
                }
                if *ratio < 0.0 || *ratio > 1.0 {
                    return Err("Reserve requirement must be between 0 and 1".to_string());
                }
                Ok(())
            }
            MonetaryAction::ProvideLiquidityAssistance { cb_id, bank_id, amount, .. } => {
                if *cb_id != state.financial_system.central_bank.id {
                    return Err("Invalid central bank ID".to_string());
                }
                Validator::positive_amount(*amount)?;
                Validator::bank_exists(*bank_id, state)
            }
        }
    }

    fn execute_omo(&self, _cb_id: AgentId, operation_type: OMOType, amount: f64, _state: &SimState) -> DomainResult {
        match operation_type {
            OMOType::QuantitativeEasing => {
                tracing::warn!("Quantitative Easing not yet implemented, amount: {}", amount);
            }
            OMOType::QuantitativeTightening => {
                tracing::warn!("Quantitative Tightening not yet implemented, amount: {}", amount);
            }
            OMOType::Repo { term_days, rate_bps } => {
                tracing::warn!("Repo operations not yet implemented (Args: {} days at {} bps)", term_days, rate_bps);
            }
            OMOType::ReverseRepo { term_days, rate_bps } => {
                tracing::warn!(
                    "Reverse Repo operations not yet implemented (Args: {} days at {} bps)",
                    term_days,
                    rate_bps
                );
            }
        }
        // Return success with no effects (no-op) to maintain deterministic flow
        DomainResult::success(vec![])
    }

    fn execute_rate_change(&self, _cb_id: AgentId, rate_bps: BasisPoints, _state: &SimState) -> DomainResult {
        let effects = vec![StateEffect::Monetary(MonetaryEffect::SetPolicyRate { rate_bps })];
        DomainResult::success(effects)
    }

    fn execute_reserve_requirement_change(&self, _cb_id: AgentId, ratio: f64, _state: &SimState) -> DomainResult {
        let effects = vec![StateEffect::Monetary(MonetaryEffect::SetReserveRequirement { ratio })];
        DomainResult::success(effects)
    }

    fn execute_liquidity_assistance(
        &self, _cb_id: AgentId, bank_id: AgentId, amount: f64, collateral: &Option<Vec<InstrumentId>>,
        _state: &SimState,
    ) -> DomainResult {
        let effects = vec![StateEffect::Monetary(MonetaryEffect::ProvideLiquidityAssistance {
            bank_id,
            amount,
            collateral: collateral.clone(),
        })];
        DomainResult::success(effects)
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Monetary",
        constructor: || Box::new(MonetaryDomain::new()),
    }
}