use crate::factory::AgentFactory;
use serde::Deserialize;
use sim_prelude::*;
use std::{collections::HashMap, str::FromStr};
use uuid::Uuid;

const _SCENARIO_NAMESPACE: Uuid = uuid::uuid!("6E62B743-2623-404B-84C8-45F48A85189A");

#[derive(Debug, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub description: String,
    config: ScenarioConfig,
    banks: Vec<BankConfig>,
    firms: Vec<FirmConfig>,
    consumers: Vec<ConsumerConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioConfig {
    iterations: u32,
    treasury_tenors_to_register: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankConfig {
    pub id: String,
    pub name: String,
    pub initial_reserves: f64,
    pub initial_bonds: Vec<BondConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmConfig {
    pub id: String,
    pub name: String,
    pub bank_id: String,
    pub recipe_name: String,
    pub initial_cash: f64,
    pub initial_inventory: Vec<InventoryConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsumerConfig {
    pub id: String,
    pub bank_id: String,
    pub initial_cash: f64,
    pub income: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BondConfig {
    pub tenor: String,
    pub face_value: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryConfig {
    pub good_slug: String,
    pub quantity: f64,
    pub unit_cost: f64,
}

impl Scenario {
    pub fn from_toml_str(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    pub fn initialize_state(&self) -> SimState {
        let mut state = SimState::default();
        state.config.iterations = self.config.iterations;
        state.financial_system.goods = goods::CATALOGUE.clone();

        let cb_id = state.financial_system.central_bank.id;
        let mut rng = rand::rng();
        let mut factory = AgentFactory::new(&mut state, &mut rng);

        let mut agent_ids: HashMap<String, AgentId> = HashMap::new();

        for bank_conf in &self.banks {
            let bank = factory.create_bank(bank_conf, cb_id);
            agent_ids.insert(bank_conf.id.clone(), bank.id);
        }

        for consumer_conf in &self.consumers {
            let bank_id = *agent_ids.get(&consumer_conf.bank_id).expect("Bank not found for consumer");
            let consumer = factory.create_consumer(consumer_conf, bank_id, cb_id);
            agent_ids.insert(consumer_conf.id.clone(), consumer.id);
        }

        for firm_conf in &self.firms {
            let bank_id = *agent_ids.get(&firm_conf.bank_id).expect("Bank not found for firm");
            let firm = factory.create_firm(firm_conf, bank_id, cb_id);
            agent_ids.insert(firm_conf.id.clone(), firm.id);
        }

        let goods_ref = &state.financial_system.goods;
        state.financial_system.exchange.register_goods_market(good_id!("petrol"), goods_ref);
        state.financial_system.exchange.register_goods_market(good_id!("oil"), goods_ref);
        for tenor_str in &self.config.treasury_tenors_to_register {
            let tenor = Tenor::from_str(tenor_str).unwrap();
            state.financial_system.exchange.register_financial_market(FinancialMarketId::Treasury { tenor });
        }
        state.financial_system.exchange.register_financial_market(FinancialMarketId::SecuredOvernightFinancing);

        state
    }
}
