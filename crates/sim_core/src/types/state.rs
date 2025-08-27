use crate::*;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::collections::{HashMap, HashSet, VecDeque};

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
                last: latest.close,
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

        let inflation_rate = 0.02; // Placeholder for now

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

    pub fn macro_stats(&self) -> MacroStats {
        // Step 1: Get all the detailed calculations first. This is now the single source of truth.
        let core_stats = self.calculate_core_stats();

        // Step 2: Calculate stats that are NOT in CoreEconomicStats (money supply, velocity).
        let banks_map = HashSet::from_iter(self.agents.banks.keys().cloned());
        let m0 = self.financial_system.m0();
        let m1 = self.financial_system.m1(&banks_map);
        let m2 = self.financial_system.m2(&banks_map);

        let (velocity, velocity_note) = if m1 > 1e-9 {
            (core_stats.consumption / m1, "proxy: daily nominal spending divided by M1 (cash+DDs, non-banks)")
        } else {
            (0.0, "proxy: velocity undefined with zero M1; returning 0.0")
        };
        
        let labour_force = self.agents.consumers.len();
        let employment = self.agents.firms.values().map(|f| f.employees.len()).sum();

        // Step 3: Assemble the final struct, mapping the detailed stats to the summary fields.
        MacroStats {
            as_of: self.current_date,
            nominal_gdp_proxy: core_stats.gdp,
            nominal_gdp_note: "proxy: C (daily) + I(dummy) + G(proxy) + NX(dummy)",
            consumer_spending_daily: core_stats.consumption,
            consumer_spending_note: "proxy: sum of turnover over CPI-weighted goods for the last simulated day",
            cpi: core_stats.cpi,
            ppi: core_stats.ppi,
            inflation_rate: self.cpi_view().inflation_rate, // Placeholder from cpi_view
            employment,
            unemployment: labour_force.saturating_sub(employment),
            labour_force,
            unemployment_rate: core_stats.unemployment_rate / 100.0, // Convert from 5.0 to 0.05
            labor_force_participation: core_stats.labor_force_participation / 100.0,
            job_openings: core_stats.job_openings,
            household_debt: core_stats.household_debt,
            corporate_debt: core_stats.corporate_debt,
            government_debt: core_stats.government_debt,
            m0,
            m1,
            m2,
            velocity,
            velocity_note,
            // --- These fields are not in CoreEconomicStats and are still proxies/dummies ---
            avg_wage_rate: 0.0, // This could be moved to core_stats if needed
            payroll_proxy: 0.0, // This could be moved to core_stats if needed
            business_investment: 0.0,
            business_investment_note: "dummy: capital formation flows not implemented yet",
            government_spending: self.financial_system.government.spending_targets.purchases, // Example proxy
            government_spending_note: "proxy: uses current SpendingTargets (not realized fiscal outlays)",
        }
    }
}

impl EconomicAnalytics for SimState {
    fn calculate_core_stats(&self) -> CoreEconomicStats {
        let fs = &self.financial_system;
        let last_tick = self.history.tick_records.back();

        // --- Duplicated logic from macro_stats() is now self-contained here ---
        let consumer_spending_daily = self.history.market_ticks
            .iter()
            .filter(|(market_id, _)| matches!(market_id, MarketId::Goods(_)))
            .flat_map(|(_, ticks)| ticks.iter())
            .filter(|tick| tick.date == self.current_date)
            .map(|tick| tick.turnover)
            .sum();

        let st = &self.financial_system.government.spending_targets;
        let government_spending_proxy = st.transfers + st.purchases + st.investment + st.debt_service;
        let gdp = consumer_spending_daily + 0.0 /* investment_dummy */ + government_spending_proxy + 0.0 /* net_exports_dummy */;
        let cpi = self.cpi_view().cpi;
        // --- End of self-contained logic ---

        let mut ppi = 0.0;
        let mut ppi_total_weight = 0.0;
        for (good_id, good) in &fs.goods.goods {
            if matches!(good.category, GoodCategory::RawMaterial | GoodCategory::IntermediateGood) {
                if let Some(market_view) = self.market_view(&MarketId::Goods(*good_id)) {
                    if let Some(price) = market_view.last_or_mid() {
                        let weight = 1.0; // Simple average for now
                        ppi += price * weight;
                        ppi_total_weight += weight;
                    }
                }
            }
        }
        if ppi_total_weight > 0.0 {
            ppi /= ppi_total_weight;
        }

        let employed_count: usize = self.agents.firms.values().map(|f| f.employees.len()).sum();
        let seeking_work_count: HashSet<AgentId> = fs.exchange.labour_markets.values()
            .flat_map(|m| m.job_applications.iter().map(|app| app.consumer_id))
            .collect();
        let labor_force = (employed_count + seeking_work_count.len()) as f64;
        let total_population = self.agents.consumers.len() as f64;
        let labor_force_participation = if total_population > 0.0 { labor_force / total_population } else { 0.0 };
        let unemployment_rate = if labor_force > 0.0 { (labor_force - employed_count as f64) / labor_force } else { 0.0 };

        let job_openings: u32 = fs.exchange.labour_markets.values()
            .flat_map(|m| &m.job_offers)
            .map(|offer| offer.quantity)
            .sum();

        let mut industrial_production = 0.0;
        let mut actual_batches_produced = 0;
        if let Some(tick) = last_tick {
            for effect in &tick.effects {
                if let StateEffect::Agent(AgentEffect::Produce { good_id, amount, .. }) = effect {
                    if let Some(market_view) = self.market_view(&MarketId::Goods(*good_id)) {
                        let price = market_view.last_or_mid().unwrap_or(1.0);
                        industrial_production += amount * price;
                    }
                }
            }
            for action in &tick.actions {
                if let SimAction::Production(ProductionAction::Produce { batches, .. }) = &action.action {
                    actual_batches_produced += batches;
                }
            }
        }
        let potential_batches: usize = self.agents.firms.values().map(|f| f.employees.len()).sum();
        let capacity_utilization = if potential_batches > 0 { (actual_batches_produced as f64) / (potential_batches as f64) } else { 0.0 };

        let household_debt: f64 = self.agents.consumers.values()
            .map(|c| fs.get_total_liabilities(&c.id)).sum();
        let corporate_debt: f64 = self.agents.firms.values()
            .map(|f| fs.get_total_liabilities(&f.id)).sum();
        let government_debt = fs.get_total_liabilities(&fs.government.id);
        let total_debt = household_debt + corporate_debt + government_debt;
        
        let mut new_credit_this_tick = 0.0;
        if let Some(tick) = last_tick {
            for effect in &tick.effects {
                if let StateEffect::Financial(FinancialEffect::CreateInstrument(inst)) = effect {
                    if inst.details.as_any().is::<LoanDetails>() || inst.details.as_any().is::<BondDetails>() {
                        new_credit_this_tick += inst.principal;
                    }
                }
            }
        }
        let credit_growth = if total_debt > 0.0 { new_credit_this_tick / total_debt } else { 0.0 };

        let bank_liabilities: f64 = self.agents.banks.values()
            .map(|b| fs.get_total_liabilities(&b.id)).sum();

        CoreEconomicStats {
            gdp,
            consumption: consumer_spending_daily,
            cpi,
            ppi,
            unemployment_rate: unemployment_rate * 100.0,
            labor_force_participation: labor_force_participation * 100.0,
            job_openings: job_openings as f64,
            capacity_utilization: capacity_utilization * 100.0,
            industrial_production,
            credit_growth,
            household_debt,
            corporate_debt,
            government_debt,
            bank_liabilities,
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
    pub tick_records: VecDeque<TickRecord>, // New field for action/effect history
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
    pub action_to_effect_indices: HashMap<usize, Vec<usize>>, // Maps action index to effect indices
    pub trades: Vec<Trade>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionRecord {
    pub action: SimAction,
    pub agent_id: AgentId,
    pub agent_type: String, // "Bank", "Consumer", "Firm", "Government"
    pub agent_name: Option<String>,
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
    pub ppi: f64, // Add
    pub inflation_rate: f64,
    pub employment: usize,
    pub unemployment: usize,
    pub labour_force: usize,
    pub unemployment_rate: f64,
    pub labor_force_participation: f64, // Add
    pub job_openings: f64, // Add
    pub household_debt: f64, // Add
    pub corporate_debt: f64, // Add
    pub government_debt: f64, // Add
    pub m0: f64,
    pub m2: f64,
    pub m1: f64,
    pub velocity: f64,
    pub velocity_note: &'static str,
}