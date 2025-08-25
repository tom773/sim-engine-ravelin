use sim_core::*;
use std::any::Any;
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimpleFirmDecisionModel {
    pub target_markup: f64,
    pub base_wage: f64,
    pub target_employees: usize,
}

impl Default for SimpleFirmDecisionModel {
    fn default() -> Self {
        Self {
            target_markup: 1.25,
            base_wage: 25.0,
            target_employees: 3,
        }
    }
}

#[typetag::serde]
impl DecisionModel for SimpleFirmDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let firm = match agent.downcast_ref::<Firm>() {
            Some(f) => f,
            None => return vec![],
        };
        
        let mut intentions = Vec::new();

        self.handle_hiring(firm, &mut intentions);
        self.handle_production(firm, state, &mut intentions);
        self.handle_wages(firm, &mut intentions);
        self.handle_sales(firm, state, &mut intentions);
        self.handle_input_purchases(firm, state, &mut intentions);

        intentions
    }
}

impl SimpleFirmDecisionModel {
    fn handle_hiring(&self, firm: &Firm, intentions: &mut Vec<SimIntention>) {
        let current_employees = firm.employees.len();
        if current_employees < self.target_employees {
            let positions_to_fill = self.target_employees - current_employees;
            intentions.push(SimIntention::HireWorkers { 
                agent_id: firm.id, 
                count: positions_to_fill as u32,
                wage_rate: self.base_wage,
            });
        }
    }

    fn handle_production(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        if firm.employees.is_empty() {
            return;
        }

        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = state.financial_system.goods.get_recipe(&recipe_id) {
                if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id)
                    .and_then(|bs| bs.get_inventory()) {
                    
                    let can_produce = recipe.inputs.iter().all(|input| {
                        inventory.get(&input.good_id)
                            .map_or(false, |item| item.quantity >= input.quantity)
                    });

                    if can_produce {
                        intentions.push(SimIntention::Produce { 
                            agent_id: firm.id, 
                            recipe_id, 
                            batches: 1,
                        });
                    }
                }
            }
        }
    }

    fn handle_wages(&self, firm: &Firm, intentions: &mut Vec<SimIntention>) {
        for (employee_id, contract) in &firm.employees {
            let weekly_wage = contract.wage_rate * contract.hours;
            if weekly_wage > 0.0 {
                intentions.push(SimIntention::PayWages {
                    employer: firm.id,
                    employee: *employee_id,
                    amount: weekly_wage,
                });
            }
        }
    }

    fn handle_sales(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id)
            .and_then(|bs| bs.get_inventory()) {
            
            for (good_id, item) in inventory {
                if item.quantity > 0.1 {
                    intentions.push(SimIntention::SellInventory {
                        agent_id: firm.id,
                        good_id: *good_id,
                        quantity: item.quantity,
                        desired_markup: self.target_markup,
                    });
                }
            }
        }
    }

    fn handle_input_purchases(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = state.financial_system.goods.get_recipe(&recipe_id) {
                if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id)
                    .and_then(|bs| bs.get_inventory()) {
                    
                    for input in &recipe.inputs {
                        let current_qty = inventory.get(&input.good_id)
                            .map_or(0.0, |item| item.quantity);
                        
                        let target_qty = input.quantity * 2.0;
                        if current_qty < target_qty {
                            let buy_qty = target_qty - current_qty;
                            let max_price = 100.0;
                            
                            intentions.push(SimIntention::PurchaseInputs {
                                agent_id: firm.id,
                                good_id: input.good_id,
                                quantity: buy_qty,
                                max_price,
                            });
                        }
                    }
                }
            }
        }
    }
}