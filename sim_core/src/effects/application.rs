use crate::*;
use crate::types::money::Money;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum EffectError {
    #[error("Instrument not found: {id:?}")]
    InstrumentNotFound { id: InstrumentId },
    #[error("Agent not found: {id:?}")]
    AgentNotFound { id: AgentId },
    #[error("Firm not found: {id:?}")]
    FirmNotFound { id: AgentId },
    #[error("Market not found: {market:?}")]
    MarketNotFound { market: String },
    #[error("Insufficient inventory for {good:?}: have {have}, need {need}")]
    InsufficientInventory {
        good: GoodId,
        have: f64,
        need: f64,
    },
    #[error("Financial system error: {0}")]
    FinancialSystemError(String),
    #[error("Invalid state: {0}")]
    InvalidState(String),
    #[error("Invalid recipe: {id:?}")]
    RecipeError { id: RecipeId },
    #[error("Unimplemented action: {0}")]
    UnimplementedAction(String),
    #[error("Unhandled action: {0}")]
    Unhandled(String),
    #[error("Bank transaction failed: Action {0}, reason {1}")]
    TransactionFailure(String, String),
}

pub trait EffectApplicator {
    fn apply_effect(&mut self, effect: &StateEffect) -> Result<(), EffectError>;

    fn apply_effects(&mut self, effects: &[StateEffect]) -> Result<(), EffectError> {
        for effect in effects.iter() {
            self.apply_effect(effect)?
        }
        Ok(())
    }
}

pub struct StateEffectApplicator;

impl StateEffectApplicator {
    pub fn apply_to_state(state: &mut SimState, effect: &StateEffect) -> Result<(), EffectError> {
        match effect {
            StateEffect::Financial(financial_effect) => {
                Self::apply_financial_effect(state, financial_effect)
            }
            StateEffect::Inventory(inventory_effect) => {
                Self::apply_inventory_effect(state, inventory_effect)
            }
            StateEffect::Market(market_effect) => Self::apply_market_effect(state, market_effect),
            StateEffect::Agent(agent_effect) => Self::apply_agent_effect(state, agent_effect),
        }
    }

    pub fn apply_inventory_effect(
        state: &mut SimState,
        effect: &InventoryEffect,
    ) -> Result<(), EffectError> {
        match effect {
            InventoryEffect::AddInventory {
                owner,
                good_id,
                quantity,
                unit_cost,
            } => {
                let money_unit_cost = Money::from_f64(*unit_cost).unwrap_or(Money::ZERO);
                state
                    .financial_system
                    .add_to_inventory(owner, good_id, *quantity, money_unit_cost);
                Ok(())
            }
            InventoryEffect::RemoveInventory {
                owner,
                good_id,
                quantity,
            } => state
                .financial_system
                .remove_from_inventory(owner, good_id, *quantity)
                .map_err(EffectError::FinancialSystemError),
        }
    }

    fn apply_agent_effect(state: &mut SimState, effect: &AgentEffect) -> Result<(), EffectError> {
        match effect {
            AgentEffect::UpdateRevenue { id: _, revenue: _ } => {
                // TODO implement revenue tracking
                Ok(())
            }
            AgentEffect::Produce {
                firm: _,
                good_id: _,
                amount: _,
            } => Ok(()),
            AgentEffect::EstablishEmployment {
                firm_id,
                consumer_id,
                contract,
            } => {
                let firm = state.agents.firms.get_mut(firm_id);
                let consumer = state.agents.consumers.get_mut(consumer_id);

                match (firm, consumer) {
                    (Some(firm), Some(consumer)) => {
                        firm.employees.insert(*consumer_id, contract.clone());
                        consumer.employed_by = Some(*firm_id);
                        consumer.hours_worked = contract.hours;
                        consumer.income = contract.wage_rate * contract.hours;
                        Ok(())
                    }
                    (None, _) => Err(EffectError::AgentNotFound { id: *firm_id }),
                    (_, None) => Err(EffectError::AgentNotFound { id: *consumer_id }),
                }
            }
            AgentEffect::TerminateEmployment {
                firm_id,
                consumer_id,
            } => {
                let firm = state.agents.firms.get_mut(firm_id);
                let consumer = state.agents.consumers.get_mut(consumer_id);

                match (firm, consumer) {
                    (Some(firm), Some(consumer)) => {
                        if firm.employees.contains_key(consumer_id)
                            && consumer.employed_by == Some(*firm_id)
                        {
                            firm.employees.remove(consumer_id);
                            consumer.employed_by = None;
                            consumer.income = 0.0;
                            consumer.hours_worked = 0.0;
                            Ok(())
                        } else {
                            Err(EffectError::InvalidState(format!(
                                "Employment relationship mismatch for termination between firm {} and consumer {}.",
                                firm_id, consumer_id
                            )))
                        }
                    }
                    (None, _) => Err(EffectError::AgentNotFound { id: *firm_id }),
                    (_, None) => Err(EffectError::AgentNotFound { id: *consumer_id }),
                }
            }
            AgentEffect::UpdateIncome { id, new_income } => {
                if let Some(consumer) = state.agents.consumers.get_mut(id) {
                    consumer.income = *new_income;
                    Ok(())
                } else {
                    Err(EffectError::AgentNotFound { id: *id })
                }
            }
            AgentEffect::RecordDividendIncome { recipient, amount } => {
                if let Some(consumer) = state.agents.consumers.get_mut(recipient) {
                    consumer.income += *amount;
                    Ok(())
                } else if let Some(_firm) = state.agents.firms.get_mut(recipient) {
                    Ok(())
                } else {
                    Err(EffectError::AgentNotFound { id: *recipient })
                }
            }
        }
    }
}

impl EffectApplicator for SimState {
    fn apply_effect(&mut self, effect: &StateEffect) -> Result<(), EffectError> {
        StateEffectApplicator::apply_to_state(self, effect)
    }
}

impl From<String> for EffectError {
    fn from(err: String) -> Self {
        EffectError::FinancialSystemError(err)
    }
}