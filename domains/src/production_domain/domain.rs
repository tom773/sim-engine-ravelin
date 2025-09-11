use crate::{Any, Domain, DomainResult, ResolutionContext, ResolutionPhase, ResolutionResult, inventory};
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::collections::HashMap;
use uuid::Uuid;
use tracing::{debug, info, trace, warn};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionDomain {}

impl ProductionDomain {
    pub fn new() -> Self {
        Self {}
    }
}

fn get_unit_cost(inventory: &HashMap<GoodId, InventoryItem>, good_id: &GoodId) -> Money {
    inventory.get(good_id).map_or(Money::ZERO, |item| item.unit_cost)
}

impl Domain for ProductionDomain {
    fn name(&self) -> &'static str {
        "Production"
    }

    fn resolve_intention(&self, intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::Production(ProductionIntention::Produce { agent_id, recipe_id, batches }) => {
                vec![SimAction::Production(ProductionAction::Produce {
                    agent_id: *agent_id,
                    recipe_id: recipe_id.clone(),
                    batches: *batches,
                })]
            }
            SimIntention::Production(ProductionIntention::FireWorkers { agent_id, employee_ids }) => {
                employee_ids
                    .iter()
                    .map(|eid| SimAction::Production(ProductionAction::Fire {
                        agent_id: *agent_id,
                        employee_id: *eid,
                    }))
                    .collect()
            }
            _ => return None,
        };

        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::Production(ProductionIntention::Produce { .. }) => Some(ResolutionPhase::Independent),
            SimIntention::Production(ProductionIntention::FireWorkers { .. }) => Some(ResolutionPhase::Independent),
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let production_action = match action {
            SimAction::Production(action) => action,
            _ => return DomainResult::failure(vec!["Not a production action".to_string()]),
        };

        if let Err(error) = self.validate(production_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match production_action {
            ProductionAction::Hire { agent_id, count } => self.execute_hire(*agent_id, *count, state),
            ProductionAction::Produce { agent_id, recipe_id, batches } => {
                self.execute_produce(*agent_id, recipe_id.clone(), *batches, state)
            }
            ProductionAction::Fire { agent_id, employee_id } => self.execute_fire(*agent_id, *employee_id),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ProductionDomain {
    fn validate(&self, action: &ProductionAction, state: &SimState) -> Result<(), String> {
        let fs = &state.financial_system;
        match action {
            ProductionAction::Hire { agent_id, count } => {
                if *count == 0 {
                    warn!(target: "sim.prod", firm_id=?agent_id, "validate_hire_zero");
                    Err("Cannot hire zero workers".to_string())
                } else {
                    trace!(target: "sim.prod", firm_id=?agent_id, count, "validate_hire_ok");
                    Validator::firm_exists(*agent_id, state)
                }
            }
            ProductionAction::Produce { agent_id, recipe_id, batches } => {
                trace!(target: "sim.prod", firm_id=?agent_id, ?recipe_id, batches, "validate_produce_start");

                if *batches == 0 {
                    warn!(target: "sim.prod", firm_id=?agent_id, ?recipe_id, "validate_produce_zero_batches");
                    return Err("Cannot produce zero batches".to_string());
                }
                Validator::firm_exists(*agent_id, state)?;

                if !fs.goods.recipes.contains_key(recipe_id) {
                    warn!(target: "sim.prod", firm_id=?agent_id, ?recipe_id, "validate_produce_recipe_missing");
                    return Err("Recipe not found".to_string());
                }
                let recipe = fs.goods.recipes.get(recipe_id).unwrap();
                let inventory = fs.get_agent_inventory(agent_id);

                for input in &recipe.inputs {
                    let available = inventory.get(&input.good_id).map_or(0.0, |item| item.quantity);
                    let required = input.quantity * (*batches as f64);
                    if available + 1e-9 < required {
                        warn!(target: "sim.prod",
                            firm_id=?agent_id, ?recipe_id, good=?input.good_id, required, available,
                            "validate_produce_insufficient_input"
                        );
                        return Err(format!(
                            "Insufficient input {:?} for production: need {:.2}, have {:.2}",
                            input.good_id, required, available
                        ));
                    }
                }
                debug!(target: "sim.prod", firm_id=?agent_id, ?recipe_id, batches, "validate_produce_ok");
                Ok(())
            }
            ProductionAction::Fire { agent_id, employee_id } => {
                if let Some(firm) = state.agents.firms.get(agent_id) {
                    if firm.employees.contains_key(employee_id) {
                        trace!(target: "sim.prod", firm_id=?agent_id, employee_id=?employee_id, "validate_fire_ok");
                        Ok(())
                    } else {
                        warn!(target: "sim.prod", firm_id=?agent_id, employee_id=?employee_id, "validate_fire_not_found");
                        Err("Employee not found in firm's employee list".to_string())
                    }
                } else {
                    warn!(target: "sim.prod", firm_id=?agent_id, employee_id=?employee_id, "validate_fire_firm_missing");
                    Err("Firm not found".to_string())
                }
            }
        }
    }

    fn execute_produce(&self, agent_id: AgentId, recipe_id: RecipeId, batches: u32, state: &SimState) -> DomainResult {
        let fs = &state.financial_system;
        let firm_name = state.agents.firms.get(&agent_id).map(|f| f.name.clone()).unwrap_or_else(|| "<unknown>".into());

        trace!(target: "sim.prod", firm_id=?agent_id, firm_name, ?recipe_id, batches, "execute_produce_start");

        if let Some(recipe) = fs.goods.recipes.get(&recipe_id) {
            let mut effects = Vec::new();
            let inventory = fs.get_agent_inventory(&agent_id);

            let input_cost: Money =
                recipe.inputs.iter().map(|input| get_unit_cost(&inventory, &input.good_id) * input.quantity).sum();

            let firm = state.agents.firms.get(&agent_id).unwrap();
            let avg_wage_rate = firm.wage_rate;
            let labor_cost = Money::from_f64(recipe.labour_hours * avg_wage_rate).unwrap_or_default();
            let total_cost_per_batch = input_cost + labor_cost;
            let total_output_quantity_per_batch: f64 = recipe.outputs.iter().map(|o| o.quantity).sum();
            let unit_cost = if total_output_quantity_per_batch > 0.0 {
                total_cost_per_batch / total_output_quantity_per_batch
            } else {
                Money::ZERO
            };

            info!(target: "sim.prod",
                firm_id=?agent_id, firm_name, ?recipe_id, batches,
                input_cost_per_batch = input_cost.to_f64(),
                labour_cost_per_batch = labor_cost.to_f64(),
                total_cost_per_batch = total_cost_per_batch.to_f64(),
                out_qty_per_batch = total_output_quantity_per_batch,
                unit_cost = unit_cost.to_f64(),
                "execute_produce_costs_computed"
            );

            for input in &recipe.inputs {
                let q = input.quantity * batches as f64;
                trace!(target: "sim.prod", firm_id=?agent_id, good=?input.good_id, quantity=q, "execute_produce_remove_input");
                effects.push(StateEffect::Inventory(InventoryEffect::RemoveInventory {
                    owner: agent_id,
                    good_id: input.good_id,
                    quantity: q,
                }));
            }

            for output in &recipe.outputs {
                let q = output.quantity * batches as f64;
                trace!(target: "sim.prod", firm_id=?agent_id, good=?output.good_id, quantity=q, unit_cost = unit_cost.to_f64(), "execute_produce_add_output");
                effects.push(StateEffect::Inventory(InventoryEffect::AddInventory {
                    owner: agent_id,
                    good_id: output.good_id,
                    quantity: q,
                    unit_cost: unit_cost.to_f64(),
                }));
            }

            debug!(target: "sim.prod", firm_id=?agent_id, firm_name, ?recipe_id, batches, "execute_produce_ok");
            DomainResult::success(effects)
        } else {
            warn!(target: "sim.prod", firm_id=?agent_id, firm_name, ?recipe_id, "execute_produce_recipe_missing");
            DomainResult::failure(vec!["Recipe not found".to_string()])
        }
    }

    fn execute_hire(&self, agent_id: AgentId, count: u32, state: &SimState) -> DomainResult {
        let market_id = match state.financial_system.find_general_labour_market() {
            Some(id) => id,
            None => return DomainResult::failure(vec!["No labour market found.".to_string()]),
        };

        let firm = state.agents.firms.get(&agent_id).unwrap(); // Safe due to validation
        let wage_rate = firm.wage_rate;

        let effects = vec![StateEffect::Market(MarketEffect::UpdateLabourMarket {
            market_id,
            update: LabourMarketUpdate::AddOffer(JobOffer {
                offer_id: Uuid::new_v4(),
                firm_id: agent_id,
                quantity: count,
                wage_rate,
                hours_required: 40.0,
            }),
        })];

        DomainResult::success(effects)
    }

    fn execute_fire(&self, firm_id: AgentId, employee_id: AgentId) -> DomainResult {
        let effect = StateEffect::Agent(AgentEffect::TerminateEmployment { firm_id, consumer_id: employee_id });

        DomainResult::success(vec![effect])
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Production",
        constructor: || Box::new(ProductionDomain::new()),
    }
}