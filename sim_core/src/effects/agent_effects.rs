use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentEffect {
    EstablishEmployment { firm_id: AgentId, consumer_id: AgentId, contract: EmploymentContract },
    TerminateEmployment { firm_id: AgentId, consumer_id: AgentId },
    UpdateIncome { id: AgentId, new_income: f64 },
    RecordDividendIncome { recipient: AgentId, amount: f64 },
    Produce { firm: AgentId, good_id: GoodId, amount: f64 },
    UpdateRevenue { id: AgentId, revenue: f64 },
    RecordCogs { id: AgentId, amount: f64 },
    RecordOpEx { id: AgentId, amount: f64 },
}

impl AgentEffect {
    pub fn name(&self) -> &'static str {
        match self {
            AgentEffect::EstablishEmployment { .. } => "EstablishEmployment",
            AgentEffect::TerminateEmployment { .. } => "TerminateEmployment",
            AgentEffect::UpdateIncome { .. } => "UpdateIncome",
            AgentEffect::RecordDividendIncome { .. } => "RecordDividendIncome",
            AgentEffect::UpdateRevenue { .. } => "UpdateRevenue",
            AgentEffect::Produce { .. } => "Produce",
            AgentEffect::RecordCogs { .. } => "RecordCogs",
            AgentEffect::RecordOpEx { .. } => "RecordOpEx",
        }
    }
}

impl StateEffectApplicator {
    pub fn apply_agent_effect(state: &mut SimState, effect: &AgentEffect) -> Result<(), EffectError> {
        match effect {
            AgentEffect::Produce { firm, good_id, amount } => {
                let firm_ref = state.agents.firms.get_mut(firm).ok_or(EffectError::AgentNotFound { id: *firm })?;
                let recipe_id = firm_ref
                    .recipe.as_mut()
                    .ok_or_else(|| EffectError::InvalidState("Firm has no recipe configured".to_string()))?;
                let recipe = state
                    .financial_system
                    .goods
                    .recipes
                    .get(&recipe_id)
                    .ok_or(EffectError::RecipeError { id: recipe_id.clone() })?;

                let out_per_batch =
                    recipe.outputs.iter().find(|o| o.good_id == *good_id).map(|o| o.quantity).ok_or_else(|| {
                        EffectError::InvalidState(format!(
                            "Produced good {:?} not part of firm's recipe {:?}",
                            good_id, recipe_id
                        ))
                    })?;
                let batches_realized = if out_per_batch > 0.0 { (*amount / out_per_batch).max(0.0) } else { 0.0 };

                let hours_available: f64 = firm_ref.employees.values().map(|c| c.hours).sum();
                let batches_theoretical = if recipe.labour_hours > 1e-9 {
                    hours_available / recipe.labour_hours
                } else {
                    batches_realized.max(1.0)
                };

                let realized = if batches_theoretical > 1e-9 {
                    (batches_realized / batches_theoretical).clamp(0.0, 2.0)
                } else {
                    1.0
                };

                let alpha = 0.20;
                firm_ref.productivity = alpha * realized + (1.0 - alpha) * firm_ref.productivity;

                Ok(())
            }

            AgentEffect::EstablishEmployment { firm_id, consumer_id, contract } => {
                let prev_firm_id = if let Some(consumer) = state.agents.consumers.get(consumer_id) {
                    consumer.employed_by
                } else {
                    return Err(EffectError::AgentNotFound { id: *consumer_id });
                };

                if let Some(prev_id) = prev_firm_id {
                    if prev_id != *firm_id {
                        if let Some(prev_firm) = state.agents.firms.get_mut(&prev_id) {
                            prev_firm.employees.remove(consumer_id);
                        }
                    }
                }

                let agents = &mut state.agents;
                let firm = agents.firms.get_mut(firm_id).ok_or(EffectError::AgentNotFound { id: *firm_id })?;
                let consumer =
                    agents.consumers.get_mut(consumer_id).ok_or(EffectError::AgentNotFound { id: *consumer_id })?;

                firm.employees.insert(*consumer_id, contract.clone());
                consumer.employed_by = Some(*firm_id);
                consumer.hours_worked = contract.hours;
                consumer.income = contract.wage_rate * contract.hours;

                Ok(())
            }
            AgentEffect::TerminateEmployment { firm_id, consumer_id } => {
                let firm = state.agents.firms.get_mut(firm_id);
                let consumer = state.agents.consumers.get_mut(consumer_id);

                match (firm, consumer) {
                    (Some(firm), Some(consumer)) => {
                        if firm.employees.contains_key(consumer_id) && consumer.employed_by == Some(*firm_id) {
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
            AgentEffect::UpdateRevenue { id, revenue } => {
                if let Some(bs) = state.financial_system.balance_sheets.get_mut(id) {
                    bs.income_statement.add_revenue(*revenue);
                    Ok(())
                } else {
                    Err(EffectError::AgentNotFound { id: *id })
                }
            }
            AgentEffect::RecordCogs { id, amount } => {
                if let Some(bs) = state.financial_system.balance_sheets.get_mut(id) {
                    bs.income_statement.add_cogs(*amount);
                    Ok(())
                } else {
                    Err(EffectError::AgentNotFound { id: *id })
                }
            }
            AgentEffect::RecordOpEx { id, amount } => {
                if let Some(bs) = state.financial_system.balance_sheets.get_mut(id) {
                    bs.income_statement.add_opex(*amount);
                    Ok(())
                } else {
                    Err(EffectError::AgentNotFound { id: *id })
                }
            }
        }
    }
}
