use crate::*;
use dyn_clone::{DynClone, clone_trait_object};
use rand::RngCore;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::collections::HashMap;
use serde_with::serde_as;
    
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FirmDecision {
    Produce { recipe_id: RecipeId, batches: u32 },
    Hire { quantity: u32 },
    SetPrice { price: f64 },
    PayWages { employee: AgentId, amount: f64 },
    SellInventory { good_id: GoodId, quantity: f64 },
}

#[serde_as]
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
    #[serde_as(as = "HashMap<_, _>")]
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
                        price: 100.0,
                        quantity: *quantity,
                    });
                }
            }
        }

        actions
    }
}