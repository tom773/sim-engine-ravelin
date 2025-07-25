use shared::*;
use serde::{Serialize, Deserialize};
use rand::prelude::*;

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

pub fn initialize_economy(_config: &SimConfig, _rng: &mut StdRng) -> SimState {
    let ss = SimState::default();
    return ss;
}