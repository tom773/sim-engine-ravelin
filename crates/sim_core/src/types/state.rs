use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::collections::{HashMap, HashSet, VecDeque}; // Import VecDeque
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

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct InflationView {
    pub cpi: f64,
    pub inflation_rate: f64, // e.g., annualized rate based on recent ticks
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

    pub fn market_view(&self, market_id: &MarketId) -> Option<MarketView> {
        self.history.market_ticks.get(market_id).map(|ticks| {
            if ticks.is_empty() {
                return MarketView::default();
            }

            let latest = ticks.back().unwrap();
            let (volume, turnover) = ticks.iter().fold((0.0, 0.0), |(vol, turn), tick| {
                (vol + tick.volume, turn + tick.turnover)
            });

            let calculate_ma = |n: usize| -> Option<f64> {
                let relevant_ticks: Vec<_> = ticks.iter().rev().take(n).filter_map(|t| t.close).collect();
                if relevant_ticks.is_empty() {
                    return None;
                }
                let sum: f64 = relevant_ticks.iter().sum();
                Some(sum / relevant_ticks.len() as f64)
            };

            let calculate_vwap = |n: usize| -> Option<f64> {
                let (total_turnover, total_volume) = ticks.iter().rev().take(n).fold((0.0, 0.0), |(turn, vol), tick| {
                    (turn + tick.turnover, vol + tick.volume)
                });
                if total_volume > 1e-6 {
                    Some(total_turnover / total_volume)
                } else {
                    None
                }
            };

            let calculate_vol = |n: usize| -> Option<f64> {
                let prices: Vec<f64> = ticks.iter().rev().take(n + 1).filter_map(|t| t.close).collect();
                if prices.len() < 2 {
                    return None;
                }

                let returns: Vec<f64> = prices.windows(2).filter_map(|w| {
                    if w[1] > 1e-9 {
                         Some((w[0] / w[1]).ln())
                    } else {
                        None
                    }
                }).collect();

                if returns.len() < 1 {
                    return None;
                }

                let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
                let divisor = if returns.len() > 1 { (returns.len() - 1) as f64 } else { 1.0 };
                let variance = returns.iter().map(|r| (r - mean_return).powi(2)).sum::<f64>() / divisor;
                
                Some(variance.sqrt() * (365.0_f64).sqrt())
            };


            MarketView {
                last: latest.close, // Use close price as the definitive last price
                mid: latest.best_bid.and_then(|bid| latest.best_ask.map(|ask| (bid + ask) / 2.0)),
                spread: latest.spread,
                volume,
                turnover,
                vwap_5: calculate_vwap(5),
                ma_20: calculate_ma(20),
                realized_vol_20: calculate_vol(20),
            }
        })
    }

    pub fn cpi_view(&self) -> InflationView {
        let mut current_cpi = 0.0;
        let mut total_weight = 0.0;
        
        for (good_id, good) in &self.financial_system.goods.goods {
            if good.cpi_weight > 0.0 {
                let market_id = MarketId::Goods(*good_id);
                if let Some(market_view) = self.market_view(&market_id) {
                    if let Some(price) = market_view.ma_20.or(market_view.last_or_mid()) {
                        current_cpi += price * good.cpi_weight;
                        total_weight += good.cpi_weight;
                    }
                }
            }
        }
        
        if total_weight > 0.0 && (total_weight < 0.99 || total_weight > 1.01) {
             current_cpi /= total_weight;
        }

        
        let inflation_rate = 0.02; 

        InflationView { cpi: current_cpi, inflation_rate }
    }
    pub fn all_market_views(&self) -> HashMap<String, MarketView> {
        let mut views = HashMap::new();
        for (market_id, _) in &self.history.market_ticks {
            if let Some(market_view) = self.market_view(market_id) {
                views.insert(market_id.to_string(), market_view);
            }
        }
        views
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

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SimHistory {
    pub transactions: Vec<Transaction>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub market_ticks: HashMap<MarketId, VecDeque<MarketTick>>,
}


impl std::hash::Hash for MarketId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            MarketId::Goods(id) => {
                0.hash(state);
                id.hash(state);
            }
            MarketId::Financial(id) => {
                1.hash(state);
                id.hash(state);
            }
            MarketId::Labour(id) => {
                2.hash(state);
                id.hash(state);
            }
        }
    }
}

impl std::cmp::PartialEq for MarketId {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MarketId::Goods(id1), MarketId::Goods(id2)) => id1 == id2,
            (MarketId::Financial(id1), MarketId::Financial(id2)) => id1 == id2,
            (MarketId::Labour(id1), MarketId::Labour(id2)) => id1 == id2,
            _ => false,
        }
    }
}

impl std::cmp::Eq for MarketId {}

impl std::fmt::Display for MarketId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketId::Goods(id) => write!(f, "Goods({})", id),
            MarketId::Financial(id) => write!(f, "Financial({})", id),
            MarketId::Labour(id) => write!(f, "Labour({})", id),
        }
    }
}

use crate::types::markets::{ParseFinancialMarketIdError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseMarketIdError {
    #[error("Invalid MarketId format: {0}")]
    InvalidFormat(String),
    #[error("Failed to parse GoodId: {0}")]
    ParseGoodId(String),
    #[error("Failed to parse FinancialMarketId: {0}")]
    ParseFinancialMarketId(#[from] ParseFinancialMarketIdError),
    #[error("Failed to parse LabourMarketId: {0}")]
    ParseLabourMarketId(String),
}


impl std::str::FromStr for MarketId {
    type Err = ParseMarketIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(content) = s.strip_prefix("Goods(").and_then(|s| s.strip_suffix(')')) {
            let id = content.parse().map_err(|_| ParseMarketIdError::ParseGoodId(content.to_string()))?;
            return Ok(MarketId::Goods(id));
        }
        if let Some(content) = s.strip_prefix("Financial(").and_then(|s| s.strip_suffix(')')) {
            let id = content.parse()?;
            return Ok(MarketId::Financial(id));
        }
        if let Some(content) = s.strip_prefix("Labour(").and_then(|s| s.strip_suffix(')')) {
            let id = content.parse().map_err(|e| ParseMarketIdError::ParseLabourMarketId(e))?;
            return Ok(MarketId::Labour(id));
        }
        Err(ParseMarketIdError::InvalidFormat(s.to_string()))
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct MacroStats {
    pub as_of: chrono::NaiveDate,

    pub nominal_gdp_proxy: f64,
    pub nominal_gdp_note: &'static str,

    pub consumer_spending_daily: f64,
    pub consumer_spending_note: &'static str,

    pub cpi: f64,
    pub inflation_rate: f64,

    pub employment: usize,
    pub unemployment: usize,
    pub labour_force: usize,
    pub unemployment_rate: f64,
    pub avg_wage_rate: f64,     // simple average across contracts
    pub payroll_proxy: f64,     // sum of (wage_rate * hours) across contracts

    pub business_investment: f64,
    pub business_investment_note: &'static str,

    pub government_spending: f64,
    pub government_spending_note: &'static str,

    pub net_exports: f64,
    pub net_exports_note: &'static str,
    pub m0: f64, // base money supply
    pub m2: f64, // broad money supply
    pub m1: f64, // cash + demand deposits held by non-banks
    pub velocity: f64,
    pub velocity_note: &'static str,
}

impl SimState {
    pub fn macro_stats(&self) -> MacroStats {
        use chrono::Duration;

        let mut employee_ids = std::collections::HashSet::new();
        let mut avg_wage_sum = 0.0;
        let mut avg_wage_n = 0usize;
        let mut payroll_sum = 0.0;

        for (_fid, firm) in &self.agents.firms {
            for (_emp_id, contract) in &firm.employees {
                employee_ids.insert(contract.employee_id);
                avg_wage_sum += contract.wage_rate;
                avg_wage_n += 1;
                payroll_sum += contract.wage_rate * contract.hours;
            }
        }

        let labour_force = self.agents.consumers.len();
        let employment = employee_ids.len();
        let unemployment = labour_force.saturating_sub(employment);
        let unemployment_rate = if labour_force > 0 {
            unemployment as f64 / labour_force as f64
        } else {
            0.0
        };
        let avg_wage_rate = if avg_wage_n > 0 { avg_wage_sum / avg_wage_n as f64 } else { 0.0 };

        let infl = self.cpi_view();
        let cpi = infl.cpi;
        let inflation_rate = infl.inflation_rate;

        let start_date = self.current_date - Duration::days(1);
        let mut consumer_spending_daily = 0.0;

        for (good_id, good) in &self.financial_system.goods.goods {
            if good.cpi_weight > 0.0 {
                let mkt_id = MarketId::Goods(*good_id);
                if let Some(ticks) = self.history.market_ticks.get(&mkt_id) {
                    for tick in ticks.iter().rev() {
                        if tick.date < start_date { break; }
                        consumer_spending_daily += tick.turnover;
                    }
                }
            }
        }

        let st = &self.financial_system.government.spending_targets;
        let government_spending_proxy = st.transfers + st.purchases + st.investment + st.debt_service;

        let business_investment_dummy = 0.0;

        let net_exports_dummy = 0.0;

        let nominal_gdp_proxy = consumer_spending_daily + business_investment_dummy + government_spending_proxy + net_exports_dummy;

        let banks_map = HashSet::from_iter(self.agents.banks.keys().cloned());
        let m0 = &self.financial_system.m0();
        let m1 = &self.financial_system.m1(&banks_map);
        let m2 = &self.financial_system.m2(&banks_map);

        let (velocity, velocity_note) = if *m1 > 1e-9 {
            (consumer_spending_daily / m1, "proxy: daily nominal spending divided by M1 (cash+DDs, non-banks)")
        } else {
            (0.0, "proxy: velocity undefined with zero M1; returning 0.0")
        };

        MacroStats {
            as_of: self.current_date,

            nominal_gdp_proxy,
            nominal_gdp_note: "proxy: C (daily) + I(dummy) + G(proxy) + NX(dummy)",

            consumer_spending_daily,
            consumer_spending_note: "proxy: sum of turnover over CPI-weighted goods for the last simulated day",

            cpi,
            inflation_rate,

            employment,
            unemployment,
            labour_force,
            unemployment_rate,
            avg_wage_rate,
            payroll_proxy: payroll_sum,

            business_investment: business_investment_dummy,
            business_investment_note: "dummy: capital formation flows not implemented yet",

            government_spending: government_spending_proxy,
            government_spending_note: "proxy: uses current SpendingTargets (not realized fiscal outlays)",

            net_exports: net_exports_dummy,
            net_exports_note: "dummy: trade not modeled (closed economy)",

            m0: *m0,
            m1: *m1,
            m2: *m2,
            velocity,
            velocity_note,
        }
    }
}