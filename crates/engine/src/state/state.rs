use shared::*;
use serde::{Serialize, Deserialize};
use rand::prelude::*;
use fake::Fake;
use fake::faker::company::en::*;
use uuid::Uuid;
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
            let bank = factory.create_bank_with_name();
            println!("  Created {}: lending spread {:.0}bps, deposit spread {:.0}bps", 
                bank.name, bank.lending_spread, bank.deposit_spread);
            bank.id
        })
        .collect();
    
    println!("\n  Creating {} firms...", config.firm_count);
    for i in 0..config.firm_count {
        let bank_id = bank_ids[i as usize % bank_ids.len()].clone();
        let firm = factory.create_firm_with_capital(bank_id.clone());
        
        let capital = factory.ss.financial_system.get_deposits_at_bank(&firm.id, &bank_id);
        println!("    {} (Bank: {}, Initial capital: ${:.0})", 
            firm.name, 
            &bank_id.0.to_string()[0..4],
            capital
        );
    }
    
    println!("\n  Creating {} consumers...", config.consumer_count);
    let mut income_distribution = vec![];
    
    for i in 0..config.consumer_count {
        let bank_id = bank_ids[i as usize % bank_ids.len()].clone();
        let income_percentile = factory.rng.random::<f64>();
        let consumer = factory.create_consumer_with_income(bank_id.clone(), income_percentile);
        
        let annual_income = consumer.income * 52.0;
        income_distribution.push(annual_income);
        
        if i < 5 || i == config.consumer_count - 1 {
            println!("    Consumer {}: Annual income ${:.0}, Bank: {}", 
                &consumer.id.0.to_string()[0..4],
                annual_income,
                &bank_id.0.to_string()[0..4]
            );
        } else if i == 5 {
            println!("    ... {} more consumers ...", config.consumer_count - 6);
        }
    }
    
    drop(factory);
    
    ss
}