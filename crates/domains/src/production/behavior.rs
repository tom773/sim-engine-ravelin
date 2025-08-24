use sim_core::*;
use std::any::Any;
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimpleFirmDecisionModel {
    pub target_markup: f64,
    pub base_wage: f64,
}

impl Default for SimpleFirmDecisionModel {
    fn default() -> Self {
        Self {
            target_markup: 1.25, // 25% markup
            base_wage: 25.0,     // Base hourly wage
        }
    }
}

#[typetag::serde]
impl DecisionModel for SimpleFirmDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimAction> {
        let firm = match agent.downcast_ref::<Firm>() {
            Some(f) => f,
            None => return vec![],
        };
        
        let mut actions = Vec::new();
        let _fs = &state.financial_system;

        self.handle_hiring(firm, &mut actions);

        self.handle_production(firm, state, &mut actions);

        self.handle_wages(firm, &mut actions);

        self.handle_sales(firm, state, &mut actions);

        self.handle_input_purchases(firm, state, &mut actions);

        actions
    }
}

impl SimpleFirmDecisionModel {
    fn handle_hiring(&self, firm: &Firm, actions: &mut Vec<SimAction>) {
        let current_employees = firm.employees.len();
        let target_employees = 3;
        if current_employees < target_employees {
            let positions_to_fill = target_employees - current_employees;
            actions.push(SimAction::Production(ProductionAction::Hire { 
                agent_id: firm.id, 
                count: positions_to_fill as u32,
            }));
        }
    }

    fn handle_production(&self, firm: &Firm, state: &SimState, actions: &mut Vec<SimAction>) {
        if firm.employees.is_empty() {
            return;
        }

        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = state.financial_system.goods.get_recipe(&recipe_id) {
                if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id)
                    .and_then(|bs| bs.get_inventory()) {
                    
                    let can_produce = recipe.inputs.iter().all(|(good_id, required_qty)| {
                        inventory.get(good_id).map_or(false, |item| item.quantity >= *required_qty)
                    });

                    if can_produce {
                        actions.push(SimAction::Production(ProductionAction::Produce { 
                            agent_id: firm.id, 
                            recipe_id, 
                            batches: 1,
                        }));
                    } else {
                        println!("Firm {} cannot produce - missing inputs", firm.name);
                    }
                }
            }
        }
    }

    fn handle_wages(&self, firm: &Firm, actions: &mut Vec<SimAction>) {
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
    }

    fn handle_sales(&self, firm: &Firm, state: &SimState, actions: &mut Vec<SimAction>) {
        if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id)
            .and_then(|bs| bs.get_inventory()) {
            
            for (good_id, item) in inventory {
                if item.quantity > 0.1 {
                    let base_price = self.calculate_selling_price(firm, *good_id, state);
                    
                    actions.push(SimAction::Trading(TradingAction::PostAsk {
                        agent_id: firm.id,
                        market_id: MarketId::Goods(*good_id),
                        quantity: item.quantity,
                        price: base_price,
                    }));
                }
            }
        }
    }

    fn handle_input_purchases(&self, firm: &Firm, state: &SimState, actions: &mut Vec<SimAction>) {
        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = state.financial_system.goods.get_recipe(&recipe_id) {
                if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id)
                    .and_then(|bs| bs.get_inventory()) {
                    
                    for (input_good_id, required_qty) in &recipe.inputs {
                        let current_qty = inventory.get(input_good_id)
                            .map_or(0.0, |item| item.quantity);
                        
                        let target_qty = required_qty * 2.0;
                        if current_qty < target_qty {
                            let buy_qty = target_qty - current_qty;
                            let max_price = 100.0; // Willing to pay up to $100 per unit
                            
                            actions.push(SimAction::Trading(TradingAction::PostBid {
                                agent_id: firm.id,
                                market_id: MarketId::Goods(*input_good_id),
                                quantity: buy_qty,
                                price: max_price,
                            }));
                        }
                    }
                }
            }
        }
    }

    fn calculate_selling_price(&self, firm: &Firm, good_id: GoodId, state: &SimState) -> f64 {
        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = state.financial_system.goods.get_recipe(&recipe_id) {
                if recipe.output.0 == good_id {
                    let labor_cost_per_hour = self.base_wage;
                    let labor_hours_per_unit = recipe.labour_hours;
                    let labor_cost_per_unit = labor_cost_per_hour * labor_hours_per_unit;
                    
                    let input_cost_per_unit: f64 = recipe.inputs.iter()
                        .map(|(_, qty)| qty * 50.0) // Assume $50 per unit of input
                        .sum();
                    
                    let total_cost = (labor_cost_per_unit + input_cost_per_unit) / recipe.efficiency;
                    return total_cost * self.target_markup;
                }
            }
        }
        
        match goods::CATALOGUE.get_good_by_id(&good_id) {
            Some(good) => match good.name.as_str() {
                "Bread" => 3.0,
                "Petrol" => 4.0,
                "Crude Oil" => 60.0,
                "Wheat" => 8.0,
                _ => 10.0,
            },
            None => 10.0,
        }
    }
}