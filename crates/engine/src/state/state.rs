use crate::{AgentFactory, StateEffect};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use shared::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimState {
    pub ticknum: u32,
    pub consumers: Vec<Consumer>,
    pub firms: Vec<Firm>,
    pub financial_system: FinancialSystem,
    pub config: SimConfig,
    pub sim_history: SimHistory,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimConfig {
    pub iterations: u32,
    pub consumer_count: u32,
    pub firm_count: u32,
    pub scenario: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimHistory {
    pub transactions: Vec<Transaction>,
    pub state_effects: Vec<StateEffect>,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self { iterations: 5, consumer_count: 2, firm_count: 1, scenario: "default".to_string() }
    }
}
impl Default for SimState {
    fn default() -> Self {
        Self {
            ticknum: 0,
            consumers: Vec::new(),
            firms: Vec::new(),
            financial_system: FinancialSystem::default(),
            config: SimConfig::default(),
            sim_history: SimHistory::new(),
        }
    }
}
impl SimState {
    pub fn get_first_agents(&self) -> (Option<&Consumer>, Option<&Firm>, Option<&Bank>) {
        let consumer = self.consumers.first();
        let firm = self.firms.first();
        let bank = self.financial_system.commercial_banks.values().next();
        (consumer, firm, bank)
    }
}
pub fn initialize_economy(config: &SimConfig, rng: &mut StdRng) -> SimState {
    let mut ss = SimState::default();

    ss.financial_system.goods =
        GoodsRegistry::from_yaml(include_str!("../../../../config/goods.yaml")).expect("Failed to load goods and recipes");
    println!(
        "Loaded {} goods and {} recipes.",
        ss.financial_system.goods.goods.len(),
        ss.financial_system.goods.recipes.len()
    );

    for good_id in ss.financial_system.goods.goods.keys() {
        ss.financial_system.exchange.register_goods_market(*good_id);
    }

    let oil_refining_recipe_id = ss.financial_system.goods.get_recipe_id_by_name("Oil Refining");

    let mut factory = AgentFactory::new(&mut ss, rng);

    println!("=== Initializing Economy ===");

    let bank_ids: Vec<AgentId> = (0..2)
        .map(|_| {
            let bank = factory.create_bank();
            bank.id
        })
        .collect();

    println!("\n  Creating {} firms...", config.firm_count);
    for i in 0..config.firm_count {
        let bank_id = bank_ids[i as usize % bank_ids.len()].clone();
        let _firm = factory.create_firm(bank_id.clone(), oil_refining_recipe_id);
    }

    println!("\n  Creating {} consumers...", config.consumer_count);
    let mut income_distribution = vec![];

    for i in 0..config.consumer_count {
        let bank_id = bank_ids[i as usize % bank_ids.len()].clone();
        let consumer = factory.create_consumer(bank_id.clone());

        let annual_income = consumer.income * 52.0;
        income_distribution.push(annual_income);
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
