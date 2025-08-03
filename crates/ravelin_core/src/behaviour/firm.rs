use crate::*;
use dyn_clone::{DynClone, clone_trait_object};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[typetag::serde(tag = "type")]
pub trait FirmDecisionModel: DynClone + Send + Sync {
    fn decide(&self, firm: &Firm, fs: &FinancialSystem, rng: &mut dyn RngCore) -> Vec<FirmDecision>;
}
clone_trait_object!(FirmDecisionModel);

impl Debug for dyn FirmDecisionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FirmDecisionModel")
    }
}

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct BasicFirmDecisionModel;

#[typetag::serde]
impl FirmDecisionModel for BasicFirmDecisionModel {
    fn decide(&self, firm: &Firm, fs: &FinancialSystem, _rng: &mut dyn RngCore) -> Vec<FirmDecision> {
        let mut decisions = Vec::new();

        if firm.employees.len() < 2 {
            decisions.push(FirmDecision::Hire { quantity: 1 });
        }

        if let Some(recipe_id) = firm.recipe {
            if firm.employees.len() > 0 {
                decisions.push(FirmDecision::Produce { recipe_id, batches: 1 });
            }

            if let Some(recipe) = fs.goods.recipes.get(&recipe_id) {
                let market = fs.exchange.goods_market(&recipe.output.0);
                let _current_price = market.and_then(|m| m.quote()).unwrap_or(25.0);

                let unit_cost = if firm.employees.len() > 0 { (firm.wage_rate * 40.0) / firm.productivity } else { 20.0 };
                let target_price = unit_cost * 1.2;

                decisions.push(FirmDecision::SetPrice { price: target_price });
            }
        }
        let assets = fs.get_liquid_assets(&firm.id.clone());
        if assets > assets - firm.wage_rate * firm.employees.len() as f64 {
            for employee in firm.get_employees() {
                decisions.push(FirmDecision::PayWages {
                    employee: employee.clone(),
                    amount: 1000.0,
                });
            }
        }
        if let Some(bs) = fs.get_bs_by_id(&firm.id.clone()){
            if let Some(good) =  bs.get_inventory() {
                for (good_id, item) in good.iter() {
                    if item.quantity > 50.0 {
                        decisions.push(FirmDecision::SellInventory {
                            good_id: *good_id,
                            quantity: item.quantity,
                        });
                    }
                }
            }
        } else {
            println!("No balance sheet found for firm {:?}", firm.id);
        } 
        decisions
    }
}