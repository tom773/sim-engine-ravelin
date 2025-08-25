use serde::{Deserialize, Serialize};
use sim_core::*;
use crate::{Any, inventory, Domain, DomainResult, DomainValidator, ResolutionContext, ResolutionResult, ResolutionPhase};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionDomain {}

impl ProductionDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for ProductionDomain {
    fn name(&self) -> &'static str { 
        "Production" 
    }

    fn resolve_intention(&self, intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::Produce { agent_id, recipe_id, batches } => {
                vec![SimAction::Production(ProductionAction::Produce { 
                    agent_id: *agent_id, recipe_id: *recipe_id, batches: *batches 
                })]
            },
            
            SimIntention::HireWorkers { agent_id, count, wage_rate } => {
                vec![SimAction::Labour(LabourAction::PostJobOffer { 
                    market_id: LabourMarketId::GeneralLabour, 
                    offer: JobOffer {
                        offer_id: Uuid::new_v4(), 
                        firm_id: *agent_id,
                        quantity: *count,
                        wage_rate: *wage_rate,
                        hours_required: 40.0,
                    } 
                })]
            },
            
            _ => return None,
        };
        
        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::Produce { .. } => Some(ResolutionPhase::Independent),
            SimIntention::HireWorkers { .. } => Some(ResolutionPhase::Independent),
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
            ProductionAction::Hire { agent_id, count } => {
                self.execute_hire(*agent_id, *count)
            },
            ProductionAction::Produce { agent_id, recipe_id, batches } => {
                self.execute_produce(*agent_id, *recipe_id, *batches, state)
            },
            ProductionAction::Fire { agent_id, employee_id } => {
                self.execute_fire(*agent_id, *employee_id)
            },
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ProductionDomain {
    fn validate(&self, action: &ProductionAction, state: &SimState) -> Result<(), String> {
        match action {
            ProductionAction::Hire { agent_id, count } => {
                if *count == 0 {
                    Err("Cannot hire zero workers".to_string())
                } else {
                    DomainValidator::firm_exists(*agent_id, state)
                }
            },
            ProductionAction::Produce { agent_id, recipe_id, batches } => {
                if *batches == 0 {
                    return Err("Cannot produce zero batches".to_string());
                }
                
                DomainValidator::firm_exists(*agent_id, state)?;
                
                if !state.financial_system.goods.recipes.contains_key(recipe_id) {
                    return Err("Recipe not found".to_string());
                }
                
                if let Some(recipe) = state.financial_system.goods.recipes.get(recipe_id) {
                    if let Some(inventory) = state.financial_system.get_bs_by_id(agent_id)
                        .and_then(|bs| bs.get_inventory()) {
                        
                        for input in &recipe.inputs {
                            let available = inventory.get(&input.good_id)
                                .map_or(0.0, |item| item.quantity);
                            let required = input.quantity * (*batches as f64);
                            
                            if available < required {
                                return Err(format!(
                                    "Insufficient {} for production: need {:.2}, have {:.2}",
                                    input.good_id, required, available
                                ));
                            }
                        }
                    } else {
                        return Err("Firm has no inventory".to_string());
                    }
                }
                
                Ok(())
            },
            ProductionAction::Fire { agent_id, employee_id } => {
                if let Some(firm) = state.agents.firms.get(agent_id) {
                    if firm.employees.iter().any(|(id, _)| id == employee_id) {
                        Ok(())
                    } else {
                        Err("Employee not found in firm's employee list".to_string())
                    }
                } else {
                    Err("Firm not found".to_string())
                }
            }
        }
    }

    fn execute_hire(&self, agent_id: AgentId, count: u32) -> DomainResult {
        let effects = vec![
            StateEffect::Market(MarketEffect::UpdateLabourMarket {
                market_id: LabourMarketId::GeneralLabour,
                update: LabourMarketUpdate::AddOffer(JobOffer {
                    offer_id: Uuid::new_v4(),
                    firm_id: agent_id,
                    quantity: count,
                    wage_rate: 25.0,
                    hours_required: 40.0,
                }),
            })
        ];
        
        DomainResult::success(effects)
    }

    fn execute_produce(&self, agent_id: AgentId, recipe_id: RecipeId, batches: u32, state: &SimState) -> DomainResult {
        if let Some(recipe) = state.financial_system.goods.recipes.get(&recipe_id) {
            let mut effects = Vec::new();
            
            for input in &recipe.inputs {
                effects.push(StateEffect::Inventory(InventoryEffect::RemoveInventory {
                    owner: agent_id,
                    good_id: input.good_id,
                    quantity: input.quantity * batches as f64,
                }));
            }
            
            for output in &recipe.outputs {
                let input_cost: f64 = recipe.inputs.iter()
                    .map(|input| {
                        state.financial_system.get_bs_by_id(&agent_id)
                            .and_then(|bs| bs.get_inventory())
                            .and_then(|inv| inv.get(&input.good_id))
                            .map_or(1.0, |item| item.unit_cost) * input.quantity
                    })
                    .sum();
                
                let labor_cost = recipe.labour_hours * 25.0;
                let total_output_quantity: f64 = recipe.outputs.iter().map(|o| o.quantity).sum();
                let unit_cost = (input_cost + labor_cost) / total_output_quantity;
                
                effects.push(StateEffect::Inventory(InventoryEffect::AddInventory {
                    owner: agent_id,
                    good_id: output.good_id,
                    quantity: output.quantity * batches as f64,
                    unit_cost,
                }));
            }
            
            DomainResult::success(effects)
        } else {
            DomainResult::failure(vec!["Recipe not found".to_string()])
        }
    }

    fn execute_fire(&self, firm_id: AgentId, employee_id: AgentId) -> DomainResult {
        let effect = StateEffect::Agent(AgentEffect::TerminateEmployment {
            firm_id,
            consumer_id: employee_id,
        });
        
        DomainResult::success(vec![effect])
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Production",
        constructor: || Box::new(ProductionDomain::new()),
    }
}