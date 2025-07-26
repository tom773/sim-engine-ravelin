use shared::*;
use serde::{Serialize, Deserialize};
use rand::prelude::*;
use crate::AgentFactory;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimState {
    pub ticknum: u32,
    pub consumers: Vec<Consumer>,
    pub firms: Vec<Firm>,
    pub financial_system: FinancialSystem,
    pub config: SimConfig,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimConfig {
    pub iterations: u32,
    pub consumer_count: u32,
    pub firm_count: u32,
    pub scenario: String,
}
impl Default for SimConfig {
    fn default() -> Self {
        Self {
            iterations: 5,
            consumer_count: 2,
            firm_count: 1,
            scenario: "default".to_string(),
        }
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
        }
    }
}
pub fn initialize_economy(config: &SimConfig, rng: &mut StdRng) -> SimState {
    let mut ss = SimState::default();
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
        let _firm = factory.create_firm(bank_id.clone());
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