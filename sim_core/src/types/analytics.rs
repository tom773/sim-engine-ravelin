use crate::prelude::*;
use std::collections::{HashMap, HashSet};

pub trait EconomicAnalytics {
    fn calculate_core_stats(&self) -> CoreEconomicStats;

    fn macro_stats(&self) -> MacroStats;

    fn market_view(&self, market_id: &MarketId) -> Option<MarketView>;

    fn cpi_view(&self) -> InflationView;

    fn all_market_views(&self) -> HashMap<String, MarketView>;
}

#[derive(Clone, Debug, Default)]
pub struct CoreEconomicStats {
    pub gdp: f64,
    pub consumption: f64,
    pub cpi: f64,
    pub ppi: f64,
    pub unemployment_rate: f64,
    pub payroll_proxy: f64,
    pub avg_wage_rate: f64,
    pub labor_force_participation: f64,
    pub job_openings: f64,
    pub capacity_utilization: f64,
    pub industrial_production: f64,
    pub credit_growth: f64,
    pub household_debt: f64,
    pub corporate_debt: f64,
    pub government_debt: f64,
    pub bank_liabilities: f64,
}

impl EconomicAnalytics for SimState {
    fn market_view(&self, market_id: &MarketId) -> Option<MarketView> {
        self.history.market_ticks.get(market_id).map(|ticks| {
            if ticks.is_empty() {
                return MarketView::default();
            }

            let latest = ticks.back().unwrap();
            let (volume, turnover) =
                ticks.iter().fold((0.0, 0.0), |(vol, turn), tick| (vol + tick.volume, turn + tick.turnover));

            let calculate_ma = |n: usize| -> Option<f64> {
                let relevant_ticks: Vec<_> = ticks.iter().rev().take(n).filter_map(|t| t.close).collect();
                if relevant_ticks.is_empty() {
                    return None;
                }
                let sum: f64 = relevant_ticks.iter().sum();
                Some(sum / relevant_ticks.len() as f64)
            };

            let calculate_vwap = |n: usize| -> Option<f64> {
                let (total_turnover, total_volume) = ticks
                    .iter()
                    .rev()
                    .take(n)
                    .fold((0.0, 0.0), |(turn, vol), tick| (turn + tick.turnover, vol + tick.volume));
                if total_volume > 1e-6 { Some(total_turnover / total_volume) } else { None }
            };

            let calculate_vol = |n: usize| -> Option<f64> {
                let prices: Vec<f64> = ticks.iter().rev().take(n + 1).filter_map(|t| t.close).collect();
                if prices.len() < 2 {
                    return None;
                }

                let returns: Vec<f64> = prices
                    .windows(2)
                    .filter_map(|w| if w[1] > 1e-9 { Some((w[0] / w[1]).ln()) } else { None })
                    .collect();

                if returns.is_empty() {
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

    fn cpi_view(&self) -> InflationView {
        let total_weight = 0.0;

        let mut current_cpi = 100.0;

        if total_weight > 0.0 && (total_weight < 0.99 || total_weight > 1.01) {
            current_cpi /= total_weight;
        }

        let inflation_rate = 0.02; // Placeholder

        InflationView { cpi: current_cpi, inflation_rate }
    }

    fn all_market_views(&self) -> HashMap<String, MarketView> {
        let mut views = HashMap::new();
        for (market_id, _) in &self.history.market_ticks {
            if let Some(market_view) = self.market_view(market_id) {
                views.insert(market_id.to_string(), market_view);
            }
        }
        views
    }

    fn macro_stats(&self) -> MacroStats {
        let core_stats = self.calculate_core_stats();
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
        let bank_credit = self.agents.banks.keys().map(|id| self.financial_system.get_total_liabilities(id)).sum();
        MacroStats {
            as_of: self.current_date,
            nominal_gdp_proxy: core_stats.gdp,
            nominal_gdp_note: "proxy: C (daily) + I(dummy) + G(proxy) + NX(dummy)",
            consumer_spending_daily: core_stats.consumption,
            consumer_spending_note: "proxy: sum of turnover over CPI-weighted goods for the last simulated day",
            cpi: core_stats.cpi,
            ppi: core_stats.ppi,
            inflation_rate: self.cpi_view().inflation_rate,
            employment,
            unemployment: labour_force.saturating_sub(employment),
            labour_force,
            unemployment_rate: core_stats.unemployment_rate, // no /100
            labor_force_participation: core_stats.labor_force_participation,
            payroll_proxy: core_stats.payroll_proxy,
            avg_wage_rate: core_stats.avg_wage_rate,
            job_openings: core_stats.job_openings,
            household_debt: core_stats.household_debt,
            corporate_debt: core_stats.corporate_debt,
            government_debt: core_stats.government_debt,
            overnight_rates: Default::default(),
            bank_credit,
            bank_liabilities: self.financial_system.all_bank_deposits(&banks_map),
            bank_reserves: self.financial_system.all_bank_reserves(&banks_map),
            m0,
            m1,
            m2,
            velocity,
            velocity_note,
            business_investment: 0.0, // Placeholder
            business_investment_note: "dummy: capital formation flows not implemented yet",
            government_spending: self.financial_system.government.spending_targets.purchases,
            government_spending_note: "proxy: uses current SpendingTargets (not realized fiscal outlays)",
        }
    }

    fn calculate_core_stats(&self) -> CoreEconomicStats {
        let fs = &self.financial_system;
        let _last_tick = self.history.tick_records.back();

        let consumer_spending_daily: f64 = self
            .history
            .market_ticks
            .iter()
            .filter(|(market_id, _)| matches!(market_id, MarketId::Goods(_)))
            .flat_map(|(_, ticks)| ticks.iter())
            .filter(|tick| tick.date == self.current_date)
            .map(|tick| tick.turnover)
            .sum();

        let st = &fs.government.spending_targets;
        let government_spending_proxy = st.transfers + st.purchases + st.investment + st.debt_service;
        let gdp = consumer_spending_daily + 0.0 + government_spending_proxy + 0.0;
        let cpi = self.cpi_view().cpi;

        let ppi = 100.0; // Placeholder until goods catalog confirmed

        let job_openings: u32 = self
            .financial_system
            .exchange
            .labour_markets
            .values()
            .map(|lm| lm.job_offers.iter().map(|o| o.quantity).sum::<u32>())
            .sum();

        let employed_count: usize = self.agents.firms.values().map(|f| f.employees.len()).sum();
        let total_population: usize = self.agents.consumers.len();
        let unemployed_count: usize = total_population.saturating_sub(employed_count);
        let labor_force = (employed_count + unemployed_count) as f64;

        let labor_force_participation =
            if total_population > 0 { labor_force / (total_population as f64) } else { 0.0 };

        let unemployment_rate = if labor_force > 0.0 { (unemployed_count as f64) / labor_force } else { 0.0 };

        let (payroll_proxy, avg_wage_rate) = {
            let mut wage_sum = 0.0;
            let mut hour_sum = 0.0;
            for firm in self.agents.firms.values() {
                for c in firm.employees.values() {
                    wage_sum += c.wage_rate * c.hours;
                    hour_sum += c.hours;
                }
            }
            let avg = if hour_sum > 0.0 { wage_sum / hour_sum } else { 0.0 };
            (wage_sum, avg)
        };

        let household_debt: f64 = self.agents.consumers.keys().map(|id| fs.get_total_liabilities(id)).sum();
        let corporate_debt: f64 = self.agents.firms.keys().map(|id| fs.get_total_liabilities(id)).sum();
        let government_debt = fs.get_total_liabilities(&fs.government.id);
        println!(
            "Household debt: {}, Corporate debt: {}, Government debt: {}",
            household_debt, corporate_debt, government_debt
        );
        CoreEconomicStats {
            gdp,
            consumption: consumer_spending_daily,
            cpi,
            ppi,
            unemployment_rate,
            labor_force_participation,
            payroll_proxy,
            avg_wage_rate,
            job_openings: job_openings as f64,
            capacity_utilization: 0.0,  // Placeholder
            industrial_production: 0.0, // Placeholder
            credit_growth: 0.0,         // Placeholder
            household_debt,
            corporate_debt,
            government_debt,
            bank_liabilities: 0.0, // Placeholder
        }
    }
}

impl FinancialSystem {
    pub fn m0(&self) -> f64 {
        self.balance_sheets
            .values()
            .flat_map(|bs| bs.assets.iter())
            .filter_map(|(id, pos)| {
                self.instruments.get(id).and_then(|inst| {
                    if let InstrumentType::Cash(d) = &inst.instrument_type {
                        if matches!(d.cash_type, CashType::CentralBankReserves | CashType::Currency) {
                            return Some(pos.quantity);
                        }
                    }
                    None
                })
            })
            .sum()
    }

    pub fn m1(&self, bank_ids: &std::collections::HashSet<AgentId>) -> f64 {
        self.balance_sheets
            .iter()
            .filter(|(id, _)| !bank_ids.contains(id) && **id != self.central_bank.id)
            .flat_map(|(_, bs)| bs.assets.iter())
            .filter_map(|(id, pos)| {
                self.instruments.get(id).and_then(|inst| {
                    if let InstrumentType::Cash(d) = &inst.instrument_type {
                        if matches!(d.cash_type, CashType::Currency | CashType::DemandDeposit) {
                            return Some(pos.quantity);
                        }
                    }
                    None
                })
            })
            .sum()
    }

    pub fn m2(&self, bank_ids: &std::collections::HashSet<AgentId>) -> f64 {
        self.balance_sheets
            .iter()
            .filter(|(id, _)| !bank_ids.contains(id) && **id != self.central_bank.id)
            .flat_map(|(_, bs)| bs.assets.iter())
            .filter_map(|(id, pos)| {
                self.instruments.get(id).and_then(|inst| {
                    if let InstrumentType::Cash(d) = &inst.instrument_type {
                        if matches!(
                            d.cash_type,
                            CashType::Currency
                                | CashType::DemandDeposit
                                | CashType::SavingsDeposit
                                | CashType::TimeDeposit
                        ) {
                            return Some(pos.quantity);
                        }
                    }
                    None
                })
            })
            .sum()
    }
    pub fn all_bank_reserves(&self, bank_ids: &std::collections::HashSet<AgentId>) -> f64 {
        bank_ids.iter().map(|id| self.get_bank_reserves(id).unwrap_or(0.0)).sum()
    }

    pub fn get_bank_reserves(&self, bank_id: &AgentId) -> Option<f64> {
        let bs = self.balance_sheets.get(bank_id)?;
        bs.assets.iter().find_map(|(id, pos)| {
            self.instruments.get(id).and_then(|inst| {
                if let InstrumentType::Cash(d) = &inst.instrument_type {
                    if d.cash_type == CashType::CentralBankReserves {
                        return Some(pos.quantity);
                    }
                }
                None
            })
        })
    }

    pub fn all_bank_deposits(&self, bank_ids: &std::collections::HashSet<AgentId>) -> f64 {
        bank_ids.iter().map(|id| self.get_bank_deposits(id)).sum()
    }

    pub fn get_bank_deposits(&self, bank_id: &AgentId) -> f64 {
        let bs = match self.balance_sheets.get(bank_id) {
            Some(b) => b,
            None => return 0.0,
        };
        bs.liabilities
            .iter()
            .filter_map(|(id, pos)| {
                self.instruments.get(id).and_then(|inst| {
                    if let InstrumentType::Cash(d) = &inst.instrument_type {
                        if matches!(
                            d.cash_type,
                            CashType::DemandDeposit | CashType::SavingsDeposit | CashType::TimeDeposit
                        ) {
                            return Some(pos.quantity);
                        }
                    }
                    None
                })
            })
            .sum()
    }
}
