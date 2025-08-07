use sim_core::*;
use std::any::Any;
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct BasicFirmDecisionModel;

#[typetag::serde]
impl DecisionModel for BasicFirmDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimAction> {
        let firm = match agent.downcast_ref::<Firm>() {
            Some(f) => f,
            None => return vec![],
        };
        
        let mut actions = Vec::new();
        let fs = &state.financial_system;

        if firm.employees.len() < 5 {
            actions.push(SimAction::Production(ProductionAction::Hire { agent_id: firm.id, count: 1 }));
        }

        if let Some(recipe_id) = firm.recipe {
            if !firm.employees.is_empty() {
                if let Some(recipe) = fs.goods.get_recipe(&recipe_id) {
                    if let Some(inventory) = fs.get_bs_by_id(&firm.id).and_then(|bs| bs.get_inventory()) {
                        let can_produce = recipe.inputs.iter().all(|(good, qty)| {
                            inventory.get(good).map_or(false, |item| item.quantity >= *qty)
                        });
                        if can_produce {
                            actions.push(SimAction::Production(ProductionAction::Produce { 
                                agent_id: firm.id, 
                                recipe_id, 
                                batches: 1 
                            }));
                        }
                    }
                }
            }
        }

        for (employee_id, contract) in &firm.employees {
            let weekly_wage = contract.wage_rate * contract.hours;
            if weekly_wage > 0.0 {
                actions.push(SimAction::Banking(BankingAction::PayWages {
                    agent_id: firm.id,
                    employee: *employee_id,
                    amount: weekly_wage,
                }));
            }
        }

        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = fs.goods.get_recipe(&recipe_id) {
                let weekly_labor_cost: f64 = firm.employees.values().map(|c| c.wage_rate * c.hours).sum();
                let weekly_output = recipe.output.1 * recipe.efficiency * firm.employees.len() as f64;
                
                if weekly_output > 0.0 {
                    let unit_cost = weekly_labor_cost / weekly_output;
                    let target_price = unit_cost * 1.25;

                    let output_good_id = recipe.output.0;
                    if let Some(inventory) = fs.get_bs_by_id(&firm.id).and_then(|bs| bs.get_inventory()) {
                        if let Some(item) = inventory.get(&output_good_id) {
                            if item.quantity > 0.0 {
                                actions.push(SimAction::Trading(TradingAction::PostAsk {
                                    agent_id: firm.id,
                                    market_id: MarketId::Goods(output_good_id),
                                    quantity: item.quantity,
                                    price: target_price,
                                }));
                            }
                        }
                    }
                }
            }
        }
        actions
    }
}