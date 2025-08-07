use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::collections::{HashMap, HashSet};
use crate::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimState {
    pub ticknum: u32,
    pub current_date: chrono::NaiveDate,
    pub financial_system: FinancialSystem,
    pub agents: AgentRegistry,
    pub config: SimConfig,
    pub history: SimHistory,
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            ticknum: 0,
            current_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            financial_system: FinancialSystem::default(),
            agents: AgentRegistry::default(),
            config: SimConfig::default(),
            history: SimHistory::default(),
        }
    }
}

impl SimState {
    pub fn advance_time(&mut self) {
        self.ticknum += 1;
        self.current_date = self.current_date + chrono::Duration::days(1);
    }
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AgentRegistry {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub banks: HashMap<AgentId, Bank>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub consumers: HashMap<AgentId, Consumer>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub firms: HashMap<AgentId, Firm>,
}

impl AgentRegistry {
    pub fn agent_exists(&self, id: &AgentId) -> bool {
        self.banks.contains_key(id) || self.consumers.contains_key(id) || self.firms.contains_key(id)
    }
    pub fn get_agent_as_any(&self, id: &AgentId) -> Option<&dyn std::any::Any> {
        if let Some(bank) = self.banks.get(id) {
            Some(bank)
        } else if let Some(consumer) = self.consumers.get(id) {
            Some(consumer)
        } else if let Some(firm) = self.firms.get(id) {
            Some(firm)
        } else {
            None
        }
    }
    pub fn get_bank(&self, id: &AgentId) -> Option<&Bank> { self.banks.get(id) }
    pub fn get_consumer(&self, id: &AgentId) -> Option<&Consumer> { self.consumers.get(id) }
    pub fn get_firm(&self, id: &AgentId) -> Option<&Firm> { self.firms.get(id) }
    pub fn get_bank_mut(&mut self, id: &AgentId) -> Option<&mut Bank> { self.banks.get_mut(id) }
    pub fn get_consumer_mut(&mut self, id: &AgentId) -> Option<&mut Consumer> { self.consumers.get_mut(id) }
    pub fn get_firm_mut(&mut self, id: &AgentId) -> Option<&mut Firm> { self.firms.get_mut(id) }
    pub fn all_agent_ids(&self) -> HashSet<AgentId> {
        self.banks.keys().cloned()
            .chain(self.consumers.keys().cloned())
            .chain(self.firms.keys().cloned())
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimConfig {
    pub iterations: u32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self { iterations: 100 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SimHistory {
    pub transactions: Vec<Transaction>,
}