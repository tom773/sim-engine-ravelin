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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimHistory {
    pub actions: Vec<SimAction>,
    pub financial_system_snapshots: Vec<FinancialSystem>,
    pub consumer_snapshots: Vec<Consumer>,
    pub firm_snapshots: Vec<Firm>,
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

impl SimHistory {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            financial_system_snapshots: Vec::new(),
            consumer_snapshots: Vec::new(),
            firm_snapshots: Vec::new(),
        }
    }
    pub fn record_action(&mut self, action: SimAction) {
        self.actions.push(action);
    }
    pub fn record_snapshot(&mut self, fs: &FinancialSystem, consumers: &[Consumer], firms: &[Firm]) {
        self.financial_system_snapshots.push(fs.clone());
        self.consumer_snapshots.extend(consumers.to_vec());
        self.firm_snapshots.extend(firms.to_vec());
    }
    pub fn get_last_n_snapshots(
        &self,
        n: usize,
    ) -> (Vec<FinancialSystem>, Vec<Consumer>, Vec<Firm>) {
        let fs_snapshots = self.financial_system_snapshots
            .iter()
            .rev()
            .take(n)
            .cloned()
            .collect();
        let consumer_snapshots = self.consumer_snapshots
            .iter()
            .rev()
            .take(n)
            .cloned()
            .collect();
        let firm_snapshots = self.firm_snapshots
            .iter()
            .rev()
            .take(n)
            .cloned()
            .collect();
        
        (fs_snapshots, consumer_snapshots, firm_snapshots)
    }
    pub fn get_last_snapshot(&self) -> Option<(FinancialSystem, Vec<Consumer>, Vec<Firm>)> {
        if self.financial_system_snapshots.is_empty() {
            return None;
        }
        let fs = self.financial_system_snapshots.last().cloned()?;
        let consumers = self.consumer_snapshots.clone();
        let firms = self.firm_snapshots.clone();
        Some((fs, consumers, firms))
    }
    pub fn clear(&mut self) {
        self.actions.clear();
        self.financial_system_snapshots.clear();
        self.consumer_snapshots.clear();
        self.firm_snapshots.clear();
    }
}