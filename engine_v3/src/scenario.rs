use crate::*;
use domains::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;
use sim_core::prelude::Money;

#[derive(Debug, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub description: String,
    pub config: ScenarioConfig,
    goods: Vec<GoodConfig>,
    recipes: Vec<RecipeConfig>,
    banks: Vec<BankConfig>,
    firms: Vec<FirmConfig>,
    consumers: Vec<ConsumerConfig>,
}

#[derive(Debug, Deserialize)] pub struct ScenarioConfig { pub iterations: u32, }
#[derive(Debug, Deserialize)] pub struct GoodConfig { pub id: String, pub name: String, pub unit: String, pub category: String, pub cpi_weight: f64, }
#[derive(Debug, Deserialize)] pub struct RecipeIOConfig { pub good_id: String, pub quantity: f64, }
#[derive(Debug, Deserialize)] pub struct RecipeConfig { pub id: String, pub name: String, pub inputs: Vec<RecipeIOConfig>, pub outputs: Vec<RecipeIOConfig>, pub labour_hours: f64, }
#[derive(Debug, Deserialize)] pub struct BankConfig { pub id: String, pub name: String, pub initial_assets: Vec<AssetConfig>, pub initial_liabilities: Vec<LiabilityConfig>, }

#[derive(Debug, Deserialize, Clone)] #[serde(tag = "type")]
pub enum DecisionModelConfig {
    ProductionFirm,
    InvestmentFirm { min_liquidity: f64, quote_qty: f64, },
}
impl DecisionModelConfig {
    pub fn into_decision_model(self) -> Box<dyn DecisionModel> {
        match self {
            DecisionModelConfig::ProductionFirm => Box::new(ProductionFirmDecisionModel::default()),
            DecisionModelConfig::InvestmentFirm { min_liquidity, quote_qty } => Box::new(InvestmentFirmDecisionModel { min_liquidity, quote_qty }),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FirmConfig {
    pub id: String, pub name: String, pub bank_id: String,
    #[serde(default)] pub recipe_name: Option<String>,
    #[serde(default)] pub desired_markup: Option<f64>,
    #[serde(default)] pub initial_assets: Vec<AssetConfig>,
    #[serde(default)] pub initial_liabilities: Vec<LiabilityConfig>,
    pub decision_model: DecisionModelConfig,
}

#[derive(Debug, Deserialize)]
pub struct ConsumerConfig {
    pub id: String, pub bank_id: String, pub income: f64,
    pub initial_assets: Vec<AssetConfig>,
    pub initial_liabilities: Vec<LiabilityConfig>,
    pub consumption_basket: HashMap<String, f64>,
}

#[derive(Debug, Deserialize)] #[serde(tag = "type")]
pub enum AssetConfig {
    Cash { amount: f64 }, Deposit { bank_id: String, amount: f64 }, Reserves { amount: f64 },
    Bond { tenor: String, quantity: u32 },
    Inventory { good_slug: String, quantity: f64, unit_cost: f64 },
}

#[derive(Debug, Deserialize)] #[serde(tag = "type")]
pub enum LiabilityConfig {
    Deposit { creditor_id: String, amount: f64, #[serde(default)] rate_bps: Option<f64>, },
    Loan { creditor_id: String, amount: f64, rate_bps: f64, #[serde(default)] maturity_days: Option<u32>, },
}

impl Scenario {
    pub fn from_toml_str(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    pub fn initialize_engine(&self) -> SimulationEngine {
        let mut state = SimState::default();
        state.config.iterations = self.config.iterations;
        let cb_id = state.financial_system.central_bank.id;
        let mut rng = rand::rng();

        let mut good_ids: HashMap<String, GoodId> = HashMap::new();
        for good_conf in &self.goods {
            let good_id = GoodId(Uuid::new_v4());
            good_ids.insert(good_conf.id.clone(), good_id);
            state.financial_system.goods.goods.insert(good_id, Good {
                id: good_id, name: good_conf.name.clone(), unit: good_conf.unit.clone(),
                category: match good_conf.category.as_str() {
                    "RawMaterial" => GoodCategory::RawMaterial,
                    "IntermediateGood" => GoodCategory::IntermediateGood,
                    "CapitalGood" => GoodCategory::CapitalGood,
                    _ => GoodCategory::ConsumerGood,
                },
                cpi_weight: good_conf.cpi_weight,
            });
            state.financial_system.exchange.register_goods_market(good_id);
        }
        for recipe_conf in &self.recipes {
            let recipe_id = RecipeId(Uuid::new_v4());
            let inputs = recipe_conf.inputs.iter().map(|io| RecipeIO { good_id: *good_ids.get(&io.good_id).unwrap(), quantity: io.quantity, }).collect();
            let outputs = recipe_conf.outputs.iter().map(|io| RecipeIO { good_id: *good_ids.get(&io.good_id).unwrap(), quantity: io.quantity, }).collect();
            state.financial_system.goods.recipes.insert(recipe_id, ProductionRecipe { id: recipe_id, name: recipe_conf.name.clone(), inputs, outputs, labour_hours: recipe_conf.labour_hours, });
        }
        
        let mut factory = AgentFactory::new(&mut state, &mut rng);
        let mut decision_models_map: HashMap<AgentId, Box<dyn DecisionModel>> = HashMap::new();
        let mut agent_ids: HashMap<String, AgentId> = HashMap::new();
        let _ = factory.initialize_treasury_general_account();
        for bank_conf in &self.banks {
            let bank = factory.create_bank(bank_conf, cb_id);
            agent_ids.insert(bank_conf.id.clone(), bank.id);
            decision_models_map.insert(bank.id, Box::new(BasicBankDecisionModel::default()));
        }

        for consumer_conf in &self.consumers {
            let bank_id = *agent_ids.get(&consumer_conf.bank_id).expect("Bank not found for consumer");
            let consumer = factory.create_consumer(consumer_conf, bank_id, cb_id, &agent_ids);
            agent_ids.insert(consumer_conf.id.clone(), consumer.id);

            let basket: HashMap<GoodId, f64> = consumer_conf.consumption_basket.iter()
                .filter_map(|(slug, share)| good_ids.get(slug).map(|id| (*id, *share)))
                .collect();

            decision_models_map.insert(consumer.id, Box::new(SimpleConsumerDecisionModel { consumption_basket: basket, ..Default::default() }));
        }

        for firm_conf in &self.firms {
            let bank_id = *agent_ids.get(&firm_conf.bank_id).expect("Bank not found for firm");
            let firm = factory.create_firm(firm_conf, bank_id, cb_id, &agent_ids);
            agent_ids.insert(firm_conf.id.clone(), firm.id);
            decision_models_map.insert(firm.id, firm_conf.decision_model.clone().into_decision_model());
            
            for asset_conf in &firm_conf.initial_assets {
                if let AssetConfig::Inventory { good_slug, quantity, unit_cost } = asset_conf {
                    if let Some(good_id) = good_ids.get(good_slug) {
                        let unit_cost_money = Money::from_f64(*unit_cost).unwrap_or(Money::ZERO);
                        factory.state.financial_system.add_to_inventory(
                            &firm.id,
                            good_id,
                            *quantity,
                            unit_cost_money,
                        );
                    } else {
                        println!("[SCENARIO WARNING] Good slug '{}' for initial inventory not found.", good_slug);
                    }
                }
            }
        }

        decision_models_map.insert(state.financial_system.government.id, Box::new(BasicGovernmentDecisionModel::default()));
        state.financial_system.exchange.register_labour_market(LabourMarketId(Uuid::new_v4()));
        let all_instruments = state.financial_system.instruments.clone();
        for (inst_id, inst) in all_instruments.iter() {
            state.financial_system.exchange.ensure_listed(*inst_id, inst);
        }

        let mut engine = SimulationEngine::new_with_scheduler(state);
        engine.decision_models = decision_models_map;
        engine
    }
}