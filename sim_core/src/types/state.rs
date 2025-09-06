use crate::prelude::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimState {
    pub ticknum: u32,
    pub current_date: chrono::NaiveDate,
    pub financial_system: FinancialSystem,
    pub agents: AgentRegistry,
    pub config: SimConfig,
    pub history: SimHistory,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct InflationView {
    pub cpi: f64,
    pub inflation_rate: f64,
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            ticknum: 0,
            current_date: chrono::NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            financial_system: FinancialSystem::default(),
            agents: AgentRegistry::default(),
            config: SimConfig::default(),
            history: SimHistory::default(),
        }
    }
}

impl SimState {
    pub fn advance_time(&mut self) {
        self.current_date += chrono::Duration::days(1);
    }
    pub fn get_agent_type_string(&self, id: &AgentId) -> Option<&'static str> {
        if self.agents.banks.contains_key(id) {
            Some("Bank")
        } else if self.agents.consumers.contains_key(id) {
            Some("Consumer")
        } else if self.agents.firms.contains_key(id) {
            Some("Firm")
        } else if *id == self.financial_system.government.id {
            Some("Government")
        } else if *id == self.financial_system.central_bank.id {
            Some("Central Bank")
        } else {
            None
        }
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
        self.banks.contains_key(id)
            || self.consumers.contains_key(id)
            || self.firms.contains_key(id)
    }
    pub fn get_bank_for_agent(&self, agent_id: &AgentId) -> Option<&Bank> {
        if let Some(consumer) = self.consumers.get(agent_id) {
            return self.banks.get(&consumer.bank_id);
        } else if let Some(firm) = self.firms.get(agent_id) {
            return self.banks.get(&firm.bank_id);
        }
        None
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

    pub fn get_agent_as_any_mut(&mut self, id: &AgentId) -> Option<&mut dyn std::any::Any> {
        if let Some(bank) = self.banks.get_mut(id) {
            Some(bank)
        } else if let Some(consumer) = self.consumers.get_mut(id) {
            Some(consumer)
        } else if let Some(firm) = self.firms.get_mut(id) {
            Some(firm)
        } else {
            None
        }
    }

    pub fn get_consumer_mut(&mut self, id: &AgentId) -> Option<&mut Consumer> {
        self.consumers.get_mut(id)
    }

    pub fn all_agent_ids(&self) -> HashSet<AgentId> {
        self.banks
            .keys()
            .cloned()
            .chain(self.consumers.keys().cloned())
            .chain(self.firms.keys().cloned())
            .collect()
    }
    pub fn get_agent_type_string(&self, id: &AgentId) -> Option<&'static str> {
        if self.banks.contains_key(id) {
            Some("Bank")
        } else if self.consumers.contains_key(id) {
            Some("Consumer")
        } else if self.firms.contains_key(id) {
            Some("Firm")
        } else {
            None
        }
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

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SimHistory {
    pub transactions: Vec<Transaction>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub market_ticks: HashMap<MarketId, VecDeque<MarketTick>>,
    pub tick_records: VecDeque<TickRecord>,
}

impl SimHistory {
    pub fn add_tick_record(&mut self, record: TickRecord) {
        self.tick_records.push_back(record);
        if self.tick_records.len() > 1000 {
            self.tick_records.pop_front();
        }
    }

    pub fn get_recent_ticks(&self, limit: usize) -> Vec<&TickRecord> {
        self.tick_records.iter().rev().take(limit).rev().collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TickRecord {
    pub tick_number: u32,
    pub date: chrono::NaiveDate,
    pub intentions: Vec<SimIntention>,
    pub actions: Vec<ActionRecord>,
    pub effects: Vec<StateEffect>,
    pub action_to_effect_indices: HashMap<usize, Vec<usize>>,
    pub trades: Vec<Trade>,
    pub events: Vec<SimEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionRecord {
    pub id: Uuid,
    pub action: SimAction,
    pub agent_id: AgentId,
    pub agent_type: String,
    pub agent_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub from_agent: AgentId,
    pub to_agent: AgentId,
    pub amount: f64,
    pub transaction_type: String,
    pub timestamp: NaiveDate,
    pub instrument_id: Option<InstrumentId>,
    pub ref_id: Option<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketTick {
    pub date: chrono::NaiveDate,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub volume: f64,
    pub turnover: f64,
    pub last_price: Option<f64>,
    pub last_qty: Option<f64>,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub spread: Option<f64>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MarketView {
    pub last: Option<f64>,
    pub mid: Option<f64>,
    pub spread: Option<f64>,
    pub volume: f64,
    pub turnover: f64,
    pub vwap_5: Option<f64>,
    pub ma_20: Option<f64>,
    pub realized_vol_20: Option<f64>,
}

impl MarketView {
    pub fn last_or_mid(&self) -> Option<f64> {
        self.last.or(self.mid)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct MacroStats {
    pub as_of: chrono::NaiveDate,
    pub nominal_gdp_proxy: f64,
    pub nominal_gdp_note: &'static str,
    pub consumer_spending_daily: f64,
    pub consumer_spending_note: &'static str,
    pub avg_wage_rate: f64,
    pub payroll_proxy: f64,
    pub business_investment: f64,
    pub business_investment_note: &'static str,
    pub government_spending: f64,
    pub government_spending_note: &'static str,
    pub cpi: f64,
    pub ppi: f64,
    pub inflation_rate: f64,
    pub employment: usize,
    pub unemployment: usize,
    pub labour_force: usize,
    pub unemployment_rate: f64,
    pub labor_force_participation: f64,
    pub job_openings: f64,
    pub household_debt: f64,
    pub corporate_debt: f64,
    pub government_debt: f64,
    pub overnight_rates: OvernightRates,
    pub bank_credit: f64,
    pub bank_liabilities: f64,
    pub bank_reserves: f64,
    pub m0: f64,
    pub m1: f64,
    pub m2: f64,
    pub velocity: f64,
    pub velocity_note: &'static str,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct OvernightRates {
    pub effr: Option<f64>,
    pub sofr: Option<f64>,
    pub iorb: f64,
    pub discount_rate: f64,
    pub overnight_rrp: f64,
}