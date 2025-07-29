use crate::*;
use dyn_clone::{DynClone, clone_trait_object};
use rand::RngCore;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FirmDecision {
    Produce { recipe_id: RecipeId, batches: u32 },
    Hire { quantity: u32 },
    SetPrice { price: f64 },
    PayWages { employee: AgentId, amount: f64 },
    SellInventory { good_id: GoodId, quantity: f64 },
}

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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
                    amount: 1000.0, // TODO replace with either consumer income, or market based wage
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Firm {
    pub id: AgentId,
    pub bank_id: AgentId,
    pub name: String,
    pub employees: Vec<AgentId>,
    pub wage_rate: f64,
    pub productivity: f64,
    pub recipe: Option<RecipeId>,
    pub decision_model: Box<dyn FirmDecisionModel>,
    pub committed_inventory: HashMap<GoodId, InventoryItem>,
}

impl Firm {
    pub fn new(id: AgentId, bank_id: AgentId, name: String, recipe: Option<RecipeId>) -> Self {
        Self {
            id,
            name,
            bank_id,
            employees: Vec::new(),
            wage_rate: 15.0,
            productivity: 1.0,
            recipe,
            decision_model: Box::new(BasicFirmDecisionModel),
            committed_inventory: HashMap::new(),
        }
    }
    pub fn hire(&mut self, count: u32) {
        println!("[HIRE] Firm {} hiring {} employees", &self.id.0.to_string()[..4], count);
    }
    pub fn produce(&self, good_id: &GoodId, amount: u32) {
        println!("[PRODUCE] Firm {} producing {} of {}", &self.id.0.to_string()[..4], amount, &good_id.0);
    }
    pub fn pay_wages(&self, employee: &AgentId, amount: f64) {
        println!("[PAY WAGES] Firm {} paying {} to employee {}", &self.id.0.to_string()[..4], amount, &employee.0);
    }
    pub fn get_employees(&self) -> Vec<AgentId> {
        self.employees.clone()
    }
}

impl Agent for Firm {
    type DecisionType = FirmDecision;

    fn decide(&self, fs: &FinancialSystem, rng: &mut StdRng) -> Vec<FirmDecision> {
        self.decision_model.decide(self, fs, rng)
    }

    fn act(&self, decisions: &[FirmDecision]) -> Vec<SimAction> {
        let mut actions = Vec::new();

        for decision in decisions {
            match decision {
                FirmDecision::Produce { recipe_id, batches } => {
                    if *batches > 0 {
                        actions.push(SimAction::Produce {
                            agent_id: self.id.clone(),
                            recipe_id: *recipe_id,
                            batches: *batches,
                        });
                    }
                }
                FirmDecision::Hire { quantity } => {
                    if *quantity > 0 {
                        actions.push(SimAction::Hire { agent_id: self.id.clone(), count: *quantity });
                    }
                }
                FirmDecision::SetPrice { price: _ } => {}
                FirmDecision::PayWages { employee, amount } => {
                    actions.push(SimAction::PayWages {
                        agent_id: self.id.clone(),
                        employee: employee.clone(),
                        amount: *amount,
                    });
                }
                FirmDecision::SellInventory { good_id, quantity } => {
                    actions.push(SimAction::PostAsk {
                        agent_id: self.id.clone(),
                        market_id: MarketId::Goods(*good_id),
                        price: 100.0, // TODO pricing mechanism 
                        quantity: *quantity, // Assuming each batch produces 10 units
                    });
                }
            }
        }

        actions
    }
}