use crate::*;
use domains::prelude::*;
use serde::Deserialize;
use std::{collections::HashMap, str::FromStr};
use uuid::Uuid;

const _SCENARIO_NAMESPACE: Uuid = uuid::uuid!("6E62B743-2623-404B-84C8-45F48A85189A");

#[derive(Debug, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub description: String,
    pub config: ScenarioConfig,
    banks: Vec<BankConfig>,
    firms: Vec<FirmConfig>,
    consumers: Vec<ConsumerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ScenarioConfig {
    pub iterations: u32,
    treasury_tenors_to_register: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct BankConfig {
    pub id: String,
    pub name: String,
    pub initial_assets: Vec<AssetConfig>,
    pub initial_liabilities: Vec<LiabilityConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum DecisionModelConfig {
    SimpleFirmDecisionModel {
        #[serde(default)]
        target_markup: Option<f64>,
        #[serde(default)]
        production_capacity: Option<f64>,
        #[serde(default)]
        inventory_threshold: Option<f64>,
    },
    InvestmentFirmDecisionModel {
        #[serde(default)]
        risk_tolerance: Option<f64>,
        #[serde(default)]
        rebalance_threshold: Option<f64>,
        #[serde(default)]
        portfolio_target: Option<String>,
    },
}

impl DecisionModelConfig {
    pub fn into_decision_model(self) -> Box<dyn DecisionModel> {
        match self {
            DecisionModelConfig::SimpleFirmDecisionModel {
                target_markup: _,
                production_capacity: _,
                inventory_threshold: _,
            } => {
                let model = ProductionFirmDecisionModel::default();

                Box::new(model)
            }
            DecisionModelConfig::InvestmentFirmDecisionModel {
                risk_tolerance: _,
                rebalance_threshold: _,
                portfolio_target: _,
            } => {
                let model = InvestmentFirmDecisionModel::default();
                Box::new(model)
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FirmConfig {
    pub id: String,
    pub name: String,
    pub bank_id: String,
    #[serde(default)]
    pub recipe_name: Option<String>,
    #[serde(default)]
    pub desired_markup: Option<f64>,
    #[serde(default)]
    pub initial_assets: Vec<AssetConfig>,
    #[serde(default)]
    pub initial_liabilities: Vec<LiabilityConfig>,
    pub decision_model: DecisionModelConfig,
}

#[derive(Debug, Deserialize)]
pub struct ConsumerConfig {
    pub id: String,
    pub bank_id: String,
    pub income: f64,
    pub initial_assets: Vec<AssetConfig>,
    pub initial_liabilities: Vec<LiabilityConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AssetConfig {
    Cash { amount: f64 },
    Deposit { bank_id: String, amount: f64 },
    Reserves { amount: f64 },
    Bond { tenor: String, quantity: u32 },
    Inventory { good_slug: String, quantity: f64, unit_cost: f64 },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum LiabilityConfig {
    Deposit {
        creditor_id: String,
        amount: f64,
        #[serde(default)]
        rate_bps: Option<f64>,
    },
    Loan {
        creditor_id: String,
        amount: f64,
        rate_bps: f64,
        #[serde(default)]
        maturity_days: Option<u32>,
    },
}

impl Scenario {
    pub fn from_toml_str(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    pub fn initialize_engine(&self) -> SimulationEngine {
        let mut state = SimState::default();
        state.config.iterations = self.config.iterations;
        state.financial_system.goods = goods::CATALOGUE.clone();

        let cb_id = state.financial_system.central_bank.id;
        let mut rng = rand::rng();
        let mut factory = AgentFactory::new(&mut state, &mut rng);

        let mut decision_models_map: HashMap<AgentId, Box<dyn DecisionModel>> = HashMap::new();
        let mut agent_ids: HashMap<String, AgentId> = HashMap::new();

        for bank_conf in &self.banks {
            let bank = factory.create_bank(&bank_conf, cb_id);
            agent_ids.insert(bank_conf.id.clone(), bank.id);
            decision_models_map.insert(bank.id, Box::new(BasicBankDecisionModel::default()));
        }

        for consumer_conf in &self.consumers {
            let bank_id = *agent_ids.get(&consumer_conf.bank_id).expect("Bank not found for consumer");
            let consumer = factory.create_consumer(&consumer_conf, bank_id, cb_id, &agent_ids);
            agent_ids.insert(consumer_conf.id.clone(), consumer.id);
            decision_models_map.insert(consumer.id, Box::new(SimpleConsumerDecisionModel::default()));
        }

        for firm_conf in &self.firms {
            let bank_id = *agent_ids.get(&firm_conf.bank_id).expect("Bank not found for firm");
            let firm = factory.create_firm(&firm_conf, bank_id, cb_id, &agent_ids);
            agent_ids.insert(firm_conf.id.clone(), firm.id);
            decision_models_map.insert(firm.id, firm_conf.decision_model.clone().into_decision_model());
        }

        decision_models_map
            .insert(state.financial_system.government.id, Box::new(BasicGovernmentDecisionModel::default()));

        let goods_ref = &state.financial_system.goods;
        state.financial_system.exchange.register_goods_market(good_id!("petrol"), goods_ref);
        state.financial_system.exchange.register_goods_market(good_id!("oil"), goods_ref);
        state.financial_system.exchange.register_goods_market(good_id!("bread"), goods_ref);
        state.financial_system.exchange.register_goods_market(good_id!("wheat"), goods_ref);

        for tenor_str in &self.config.treasury_tenors_to_register {
            let tenor = Tenor::from_str(tenor_str).unwrap();
            state.financial_system.exchange.register_financial_market(FinancialMarketId::Treasury { tenor });
        }

        state.financial_system.exchange.register_financial_market(FinancialMarketId::FederalFundsOvernight);
        state.financial_system.exchange.register_financial_market(FinancialMarketId::TreasuryRepoOvernight);
        state.financial_system.exchange.register_financial_market(FinancialMarketId::DiscountWindow);
        state.financial_system.exchange.register_financial_market(FinancialMarketId::StandingRepoFacility);
        state.financial_system.exchange.register_financial_market(FinancialMarketId::OvernightReverseRepo);

        state.financial_system.exchange.register_labour_market(LabourMarketId::GeneralLabour);

        let mut engine = SimulationEngine::new_with_scheduler(state);
        engine.decision_models = decision_models_map;
        engine.run_initialization();

        engine
    }

    pub fn get_goods_catalogue(&self) -> HashMap<GoodId, Good> {
        sim_core::goods::CATALOGUE.goods.clone()
    }

    pub fn get_recipes_catalogue(&self) -> HashMap<RecipeId, ProductionRecipe> {
        sim_core::goods::CATALOGUE.recipes.clone()
    }
}
