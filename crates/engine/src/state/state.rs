use crate::{AgentFactory, StateEffect, execution::domain::DomainRegistry, state::scenario::Scenario};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use shared::*;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimState {
    pub ticknum: u32,
    pub consumers: Vec<Consumer>,
    pub domain_registry: DomainRegistry,
    pub firms: Vec<Firm>,
    pub financial_system: FinancialSystem,
    pub config: SimConfig,
    pub sim_history: SimHistory,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimConfig {
    pub iterations: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimHistory {
    pub transactions: Vec<Transaction>,
    pub state_effects: Vec<StateEffect>,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self { iterations: 100 }
    }
}
impl Default for SimState {
    fn default() -> Self {
        Self {
            ticknum: 0,
            consumers: Vec::new(),
            domain_registry: DomainRegistry::default(),
            firms: Vec::new(),
            financial_system: FinancialSystem::default(),
            config: SimConfig::default(),
            sim_history: SimHistory::new(),
        }
    }
}
impl SimState {
    pub fn with_domain_registry(mut self, registry: DomainRegistry) -> Self {
        self.domain_registry = registry;
        self
    }
}

pub fn initialize_economy_from_scenario(scenario: &Scenario, rng: &mut StdRng) -> SimState {
    let mut ss = SimState::default();
    ss.config.iterations = scenario.config.iterations;

    ss.financial_system.goods =
        GoodsRegistry::from_toml(include_str!("./config/goods.toml")).expect("Failed to load goods and recipes");

    for good_id in ss.financial_system.goods.goods.keys() {
        ss.financial_system.exchange.register_goods_market(*good_id);
    }
    for tenor in &scenario.config.treasury_tenors_to_register {
        ss.financial_system.exchange.register_financial_market(FinancialMarketId::Treasury { tenor: *tenor });
    }

    let recipe_id_map: HashMap<String, Option<RecipeId>> = scenario
        .firms
        .iter()
        .map(|f_conf| {
            (f_conf.recipe_name.clone(), ss.financial_system.goods.get_recipe_id_by_name(&f_conf.recipe_name))
        })
        .collect();

    let mut factory = AgentFactory::new(&mut ss, rng);
    let central_bank_id = factory.ss.financial_system.central_bank.id.clone();

    let mut bank_id_map = HashMap::new();
    let mut consumer_ids = Vec::new();

    for bank_conf in &scenario.banks {
        let bank = factory.create_bank_from_config(bank_conf, &central_bank_id);
        bank_id_map.insert(bank_conf.id.clone(), bank.id);
    }

    for consumer_conf in &scenario.consumers {
        let bank_id = bank_id_map.get(&consumer_conf.bank_id).expect("Bank ID not found for consumer").clone();
        let consumer = factory.create_consumer_from_config(consumer_conf, bank_id, &central_bank_id);
        consumer_ids.push(consumer.id);
    }

    for (i, firm_conf) in scenario.firms.iter().enumerate() {
        let bank_id = bank_id_map.get(&firm_conf.bank_id).expect("Bank ID not found for firm").clone();
        let recipe_id = *recipe_id_map.get(&firm_conf.recipe_name).expect("Recipe name not found in map");

        if !consumer_ids.is_empty() {
            let employee_id = &consumer_ids[i % consumer_ids.len()];
            factory.create_firm_from_config(firm_conf, bank_id, recipe_id, employee_id, &central_bank_id);
        } else {
            println!("Warning: No consumers available to hire for firm {}", firm_conf.name);
        }
    }

    drop(factory);
    ss
}

impl SimHistory {
    pub fn new() -> Self {
        Self { transactions: Vec::new(), state_effects: Vec::new() }
    }

    pub fn record_transaction(&mut self, tx: Transaction) {
        self.transactions.push(tx);
    }

    pub fn record_state_effect(&mut self, effect: StateEffect) {
        self.state_effects.push(effect);
    }
}
