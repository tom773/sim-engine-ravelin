use crate::*;
use dyn_clone::{DynClone, clone_trait_object};
use rand::RngCore;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FirmDecision {
    Produce { recipe_id: RecipeId, batches: u32 },
    Hire { quantity: u32 },
    SetPrice { price: f64 },
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

        if firm.employees < 10 {
            decisions.push(FirmDecision::Hire { quantity: 5 });
        }

        if let Some(recipe_id) = firm.recipe {
            if firm.employees > 0 {
                decisions.push(FirmDecision::Produce { recipe_id, batches: 1 });
            }

            if let Some(recipe) = fs.goods.recipes.get(&recipe_id) {
                let market = fs.exchange.goods_market(&recipe.output.0);
                let _current_price = market.and_then(|m| m.quote()).unwrap_or(25.0);

                let unit_cost = if firm.employees > 0 { (firm.wage_rate * 40.0) / firm.productivity } else { 20.0 };
                let target_price = unit_cost * 1.2;

                decisions.push(FirmDecision::SetPrice { price: target_price });
            }
        }

        decisions
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Firm {
    pub id: AgentId,
    pub bank_id: AgentId,
    pub name: String,
    pub employees: u32,
    pub wage_rate: f64,
    pub productivity: f64,
    pub recipe: Option<RecipeId>,
    pub decision_model: Box<dyn FirmDecisionModel>,
}

impl Firm {
    pub fn new(id: AgentId, bank_id: AgentId, name: String, recipe: Option<RecipeId>) -> Self {
        Self {
            id,
            name,
            bank_id,
            employees: 0,
            wage_rate: 15.0,
            productivity: 1.0,
            recipe,
            decision_model: Box::new(BasicFirmDecisionModel),
        }
    }
    pub fn hire(&mut self, count: u32) {
        self.employees += count;
        println!("[HIRE] Firm {} hired {} employees", &self.id.0.to_string()[..4], count);
    }
    pub fn produce(&self, good_id: &GoodId, amount: u32) {
        println!("[PRODUCE] Firm {} producing {} of {}", &self.id.0.to_string()[..4], amount, &good_id.0);
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
            }
        }

        actions
    }
}

#[cfg(test)]
mod tests_firm {
    use super::*;
    use uuid::Uuid;

    fn test_scenario() -> (Firm, FinancialSystem) {
        let id = AgentId(Uuid::new_v4());
        let bank_id = AgentId(Uuid::new_v4());
        let name = "Test Firm".to_string();
        let recipe = Some(recipe_id!("Oil Refining"));
        let firm = Firm::new(id, bank_id, name, recipe);

        let mut fs = FinancialSystem::default();
        fs.goods =
            GoodsRegistry::from_yaml(include_str!("../../../../config/goods.yaml")).expect("failed to parse goods");
        for good_id in fs.goods.goods.keys() {
            fs.exchange.register_goods_market(*good_id);
        }

        (firm, fs)
    }

    #[test]
    fn test_firm_dm() {
        let (firm, fs) = test_scenario();
        let mut rng = StdRng::seed_from_u64(0);
        let decisions = firm.decide(&fs, &mut rng);
        assert!(!decisions.is_empty(), "Firm should make some decisions");
        assert!(decisions.iter().any(|d| matches!(d, FirmDecision::Produce { .. })), "Firm should decide to produce");
        assert!(decisions.iter().any(|d| matches!(d, FirmDecision::Hire { .. })), "Firm should decide to hire");
    }
    #[test]
    fn test_firm_recipe() {
        let (firm, mut fs) = test_scenario();
        assert!(firm.recipe.is_some(), "Firm should have a production recipe");
        let recipe = firm.recipe.as_ref().unwrap();
        let expected_recipe_id = recipe_id!("Oil Refining");
        assert_eq!(recipe, &expected_recipe_id, "Firm should have 'Oil Refining' as its recipe");
        let r_dets = firm.recipe.as_ref().and_then(|id| fs.goods.recipes.get(id)).unwrap();

        let output_good = fs.goods.goods.get(&r_dets.output.0).unwrap().name.clone();
        let input_goods =
            r_dets.inputs.iter().map(|(id, _)| fs.goods.goods.get(id).unwrap().clone()).collect::<Vec<_>>();
        let lab = r_dets.labour_hours;
        let eff = r_dets.efficiency;
        let name = r_dets.name.clone();

        println!("\n\nRecipe: {} -> {} ({} hours, {} efficiency)", name, output_good, lab, eff);
        input_goods.iter().for_each(|g| println!("Input: {}", g.name));

        assert_eq!(output_good, "Petrol", "Output good should be 'Refined Oil'");
        assert_eq!(input_goods.len(), 1, "Recipe should have two input goods");
        assert_eq!(input_goods[0].name, "Crude Oil", "First input should be 'Crude Oil'");

        fs.exchange.register_goods_market(r_dets.output.0);
        fs.exchange.goods_market_mut(&r_dets.output.0)
            .expect("Failed to get goods market")
            .post_ask(firm.id, 1.0, 9.8);
        fs.exchange.goods_market_mut(&r_dets.output.0)
            .expect("Failed to get goods market")
            .post_bid(AgentId(Uuid::new_v4()), 1.0, 10.2);
        let mid = fs.exchange.goods_market(&r_dets.output.0)
            .expect("Failed to get goods market")
            .quote()
            .expect("Failed to get market quote");

        println!("\nMarket price for {}: {}", &r_dets.output.0.0.to_string()[..3], mid);
        let unit_cost = (firm.wage_rate * 40.0) / firm.productivity;
        let target_price = unit_cost * 1.2;
        println!("Unit cost: {}, Target price: {}\n\n", unit_cost, target_price);

    }
}
