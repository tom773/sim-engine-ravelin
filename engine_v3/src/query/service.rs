use crate::*;
use axum::response::Json;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use sim_core::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct QueryService {
    engine: Arc<Mutex<SimulationEngine>>,
}

type QueryResult<T> = Result<T, (axum::http::StatusCode, String)>;

impl QueryService {
    pub fn new(engine: Arc<Mutex<SimulationEngine>>) -> Self {
        Self { engine }
    }

    fn get_engine_lock(&self) -> Result<std::sync::MutexGuard<'_, SimulationEngine>, (axum::http::StatusCode, String)> {
        self.engine.lock().map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    pub fn get_market_overview(&self) -> QueryResult<MarketOverviewDto> {
        let engine = self.get_engine_lock()?;
        Ok(MarketOverviewDto { 
            instrument_registry: engine.state.financial_system.instruments.clone() 
        })
    }

    fn populate_balance_sheet(&self, agent_id: &AgentId, state: &SimState) -> PopulatedBalanceSheetDto {
        let all_assets_map = state.financial_system.get_agent_total_positions(agent_id);

        let assets = all_assets_map
            .iter()
            .filter_map(|(id, pos)| {
                state.financial_system.instruments.get(id).map(|inst| {
                    let market_price = state
                        .financial_system
                        .exchange
                        .financial_market(id)
                        .and_then(|book| book.representative_price());

                    PopulatedPositionDto { position: pos.clone(), instrument: inst.clone(), market_price }
                })
            })
            .collect();

        let bs = state.financial_system.balance_sheets.get(agent_id).unwrap();
        let liabilities = bs
            .liabilities
            .iter()
            .filter_map(|(id, pos)| {
                state.financial_system.instruments.get(id).map(|inst| {
                    let market_price = state
                        .financial_system
                        .exchange
                        .financial_market(id)
                        .and_then(|book| book.representative_price());

                    PopulatedPositionDto { position: pos.clone(), instrument: inst.clone(), market_price }
                })
            })
            .collect();

        PopulatedBalanceSheetDto { assets, liabilities, income_statement: bs.income_statement.clone() }
    }

    pub fn get_status_data(&self) -> QueryResult<StatusDto> {
        let engine_lock = self.get_engine_lock()?;
        let state = &engine_lock.state;

        let macro_stats = state.macro_stats();
        let agent_counts = AgentCounts {
            banks: state.agents.banks.len(),
            firms: state.agents.firms.len(),
            consumers: state.agents.consumers.len(),
            total: state.agents.banks.len() + state.agents.firms.len() + state.agents.consumers.len(),
        };
        let mut agents_map: HashMap<AgentId, String> = state
            .agents
            .all_agent_ids()
            .into_iter()
            .map(|id| {
                let name = engine_lock.get_agent_info(&id).1.unwrap_or_else(|| "N/A".to_string());
                (id, name)
            })
            .collect();
        agents_map.insert(state.financial_system.government.id.clone(), "Government".to_string());
        agents_map.insert(state.financial_system.central_bank.id.clone(), "Central Bank".to_string());
        let instruments_map: HashMap<InstrumentId, String> = state
            .financial_system
            .instruments
            .iter()
            .map(|(id, inst)| (id.clone(), inst.type_as_string().to_string()))
            .collect();
        let macro_stats_dto = MacroStatsDto {
            nominal_gdp_proxy: macro_stats.nominal_gdp_proxy,
            consumer_spending_daily: macro_stats.consumer_spending_daily,
            household_debt: macro_stats.household_debt,
            corporate_debt: macro_stats.corporate_debt,
            government_debt: macro_stats.government_debt,
            cpi: macro_stats.cpi,
            inflation_rate: macro_stats.inflation_rate,
            unemployment_rate: macro_stats.unemployment_rate,
            m0: macro_stats.m0,
            m1: macro_stats.m1,
            m2: macro_stats.m2,
            bank_reserves: macro_stats.bank_reserves,
        };
        let am_cl = agents_map.clone();
        let contracts: Vec<EmploymentRecordDto> = state
            .agents
            .firms
            .iter()
            .flat_map(|(fid, firm)| {
                let agents_map = agents_map.clone();
                firm.employees.values().clone().map(move |c| EmploymentRecordDto {
                    firm_id: *fid,
                    firm_name: agents_map.get(fid).cloned().unwrap_or_else(|| "Firm".into()),
                    contract: c.clone(),
                })
            })
            .collect();
        let labour_stats = LabourMarketStatsDto {
            employment: macro_stats.employment,
            unemployment: macro_stats.unemployment,
            labour_force: macro_stats.labour_force,
            unemployment_rate: macro_stats.unemployment_rate, // already fraction
            labor_force_participation: macro_stats.labor_force_participation, // remove *100.0
            job_openings: macro_stats.job_openings as usize,
            payroll_proxy: macro_stats.payroll_proxy as usize,
            avg_wage_rate: macro_stats.avg_wage_rate,
            contracts,
        };

        let on_rates = OvernightRatesDto {
            effr: Some(state.financial_system.central_bank.policy_rate_bps + dec!(13.0)),
            sofr: Some(state.financial_system.central_bank.policy_rate_bps + dec!(17.0)),
            iorb: Some(state.financial_system.central_bank.policy_rate_bps),
            discount_rate: Some(state.financial_system.central_bank.policy_rate_bps + dec!(20.0)),
            overnight_RRP: Some(state.financial_system.central_bank.policy_rate_bps + dec!(25.0)),
        };
        let monetary_stats_dto = MonetaryStatsDto {
            policy_rate: state.financial_system.central_bank.policy_rate_bps,
            reserve_requirement: Rate::from_f64(state.financial_system.central_bank.reserve_requirement)
                .unwrap_or_default(),
            overnight_rates: on_rates,
        };
        Ok(StatusDto {
            current_date: state.current_date.format("%Y-%m-%d").to_string(),
            tick_number: state.ticknum,
            total_iterations: state.config.iterations,
            agent_counts,
            macro_stats: macro_stats_dto,
            monetary_stats: monetary_stats_dto,
            labor_force_stats: labour_stats,
            maps: MapsDto { agents_map: am_cl.clone(), instruments_map },
        })
    }

    pub fn get_agents_summary(&self, agent_type_filter: Option<String>) -> QueryResult<Vec<AgentSummaryDto>> {
        let engine = self.get_engine_lock()?;
        let state = &engine.state;
        let _fs = &state.financial_system;

        let mut summaries = Vec::new();
        let filter_lower = agent_type_filter.as_deref().map(str::to_lowercase);

        let mut add_summary_for_agent = |id: &AgentId| {
            let (agent_type_str, name_opt) = engine.get_agent_info(id);
            let populated_bs = self.populate_balance_sheet(id, state);
            summaries.push(AgentSummaryDto {
                id: id.0,
                agent_type: agent_type_str,
                name: name_opt.unwrap_or_else(|| "N/A".to_string()),
                balance_sheet: populated_bs,
            });
        };

        if filter_lower.as_deref() == Some("bank") || filter_lower.is_none() {
            for id in state.agents.banks.keys() {
                add_summary_for_agent(id);
            }
        }
        if filter_lower.as_deref() == Some("firm") || filter_lower.is_none() {
            for id in state.agents.firms.keys() {
                add_summary_for_agent(id);
            }
        }
        if filter_lower.as_deref() == Some("consumer") || filter_lower.is_none() {
            for id in state.agents.consumers.keys() {
                add_summary_for_agent(id);
            }
        }

        if filter_lower.as_deref() == Some("government") || filter_lower.is_none() {
            add_summary_for_agent(&state.financial_system.government.id);
        }
        if filter_lower.as_deref() == Some("centralbank") || filter_lower.is_none() {
            add_summary_for_agent(&state.financial_system.central_bank.id);
        }

        Ok(summaries)
    }

    pub fn get_agent_detail(&self, agent_id: Uuid) -> QueryResult<AgentDetailDto> {
        let engine = self.get_engine_lock()?;
        let state = &engine.state;
        let agent_id = AgentId(agent_id);

        let (agent_type, name) = engine.get_agent_info(&agent_id);
        if agent_type == "Unknown" {
            return Err((axum::http::StatusCode::NOT_FOUND, format!("Agent with ID {} not found", agent_id)));
        }

        let populated_bs = self.populate_balance_sheet(&agent_id, state);

        Ok(AgentDetailDto {
            id: agent_id.0,
            agent_type,
            name: name.unwrap_or_else(|| "N/A".to_string()),
            balance_sheet: populated_bs,
        })
    }

    pub fn get_markets_summary(&self) -> QueryResult<Vec<MarketSummaryDto>> {
        let engine = self.get_engine_lock()?;
        let state = &engine.state;
        let mut summaries = Vec::new();

        for (inst_id, market) in &state.financial_system.exchange.markets {
            let market_id = MarketId::Financial(*inst_id);
            let view = state.market_view(&market_id).unwrap_or_default();

            let (best_bid_yield, best_ask_yield, mid_yield, last_yield) =
                if let Some(inst) = state.financial_system.instruments.get(inst_id) {
                    let bond_details = match &inst.instrument_type {
                        InstrumentType::Debt(debt_inst) => {
                            match debt_inst {
                                DebtInstrument::Bond(d) => Some(d),
                                DebtInstrument::Loan(_) => None, // Loans might not have yield calculations
                                DebtInstrument::CreditLine(_) => None,
                                DebtInstrument::TradeCredit(_) => None,
                            }
                        }
                        _ => None,
                    };

                    if let Some(d) = bond_details {
                        let calc_yield = |price_opt: Option<Money>| -> Option<Rate> {
                            price_opt.map(|price| {
                                let ytm_years = years_to_maturity(state.current_date, d.maturity_date);
                                ytm_bond(price, d.face_value, d.coupon_rate_bps, ytm_years, d.frequency as usize)
                            })
                        };

                        (
                            calc_yield(market.book.best_bid()),
                            calc_yield(market.book.best_ask()),
                            calc_yield(market.book.mid_price()),
                            calc_yield(market.book.last_price),
                        )
                    } else {
                        (None, None, None, None)
                    }
                } else {
                    (None, None, None, None)
                };

            summaries.push(MarketSummaryDto {
                market_id: market_id.to_string(),
                market_type: "financial".to_string(),
                name: state
                    .financial_system
                    .instruments
                    .get(inst_id)
                    .map_or("Unknown".to_string(), |i| self.get_instrument_display_name(i)),
                last_price: view.last.and_then(Rate::from_f64),
                mid_price: market.book.mid_price().map(|m| m.0),
                best_bid: market.book.best_bid().map(|m| m.0),
                best_ask: market.book.best_ask().map(|m| m.0),
                spread: market.book.spread().map(|m| m.0),
                volume_24h: view.volume,
                turnover_24h: view.turnover,
                depth: None,
                best_bid_yield,
                best_ask_yield,
                mid_yield,
                last_yield,
            });
        }

        for (good_id, market) in &state.financial_system.exchange.goods_markets {
            let market_id = MarketId::Goods(*good_id);
            let view = state.market_view(&market_id).unwrap_or_default();
            let depth_summary = market.book.depth_summary();

            let bid_levels =
                depth_summary.bid_levels.into_iter().map(|(k, v)| (k.into_inner().to_string(), v)).collect();
            let ask_levels =
                depth_summary.ask_levels.into_iter().map(|(k, v)| (k.into_inner().to_string(), v)).collect();

            let depth_dto = OrderBookDepthDto {
                bid_levels,
                ask_levels,
                bid_size_at_best: depth_summary.bid_size_at_best,
                ask_size_at_best: depth_summary.ask_size_at_best,
            };

            summaries.push(MarketSummaryDto {
                market_id: market_id.to_string(),
                market_type: "goods".to_string(),
                name: state.financial_system.goods.goods.get(good_id).map_or("Unknown".to_string(), |g| g.name.clone()),
                last_price: view.last.and_then(Rate::from_f64),
                mid_price: market.book.mid_price().map(|m| m.0),
                best_bid: depth_summary.best_bid.map(|m| m.0),
                best_ask: depth_summary.best_ask.map(|m| m.0),
                spread: market.book.spread().map(|m| m.0),
                volume_24h: view.volume,
                turnover_24h: view.turnover,
                depth: Some(depth_dto),
                best_bid_yield: None,
                best_ask_yield: None,
                mid_yield: None,
                last_yield: None,
            });
        }

        for (market_id, _market) in &state.financial_system.exchange.labour_markets {
            summaries.push(MarketSummaryDto {
                market_id: MarketId::Labour(*market_id).to_string(),
                market_type: "labour".to_string(),
                name: "General Labour Market".to_string(),
                last_price: None,
                mid_price: None,
                best_bid: None,
                best_ask: None,
                spread: None,
                volume_24h: 0.0,
                turnover_24h: 0.0,
                depth: None,
                best_bid_yield: None,
                best_ask_yield: None,
                mid_yield: None,
                last_yield: None,
            });
        }

        Ok(summaries)
    }

    fn get_instrument_display_name(&self, instrument: &Instrument) -> String {
        match &instrument.instrument_type {
            InstrumentType::Debt(debt_inst) => match debt_inst {
                DebtInstrument::Bond(bond) => {
                    format!("{:?} Bond", bond.bond_type)
                }
                DebtInstrument::Loan(_) => "Loan".to_string(),
                DebtInstrument::CreditLine(_) => "Credit Line".to_string(),
                DebtInstrument::TradeCredit(_) => "Mortgage".to_string(),
            },
            InstrumentType::Cash(cash) => format!("{:?}", cash.cash_type).replace('_', " "),
            InstrumentType::Equity(_) => "Equity".to_string(),
            InstrumentType::RealAsset(_) => "Real Asset".to_string(),
            InstrumentType::Repo(_) => "Repo".to_string(),
            InstrumentType::Derivative(_) => "Derivative".to_string(),
            InstrumentType::StructuredTranche(_) => "Structured Product".to_string(),
        }
    }

    pub fn get_catalog(&self) -> QueryResult<CatalogDto> {
        let engine = self.get_engine_lock()?;
        Ok(CatalogDto {
            goods: engine.state.financial_system.goods.goods.values().cloned().collect(),
            recipes: engine.state.financial_system.goods.recipes.values().cloned().collect(),
        })
    }
    pub fn get_instrument_registry(&self) -> QueryResult<InstrumentRegistryDto> {
        let engine = self.get_engine_lock()?;
        Ok(InstrumentRegistryDto { instruments: engine.state.financial_system.instruments.clone() })
    }
    pub fn get_market_detail(&self, market_id_str: &str) -> QueryResult<MarketDetailDto> {
        let engine = self.get_engine_lock()?;
        let state = &engine.state;
        let market_id: MarketId = market_id_str.parse().map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;

        match market_id {
            MarketId::Financial(inst_id) => {
                let market = state
                    .financial_system
                    .exchange
                    .markets
                    .get(&inst_id)
                    .ok_or((axum::http::StatusCode::NOT_FOUND, "Financial market not found".to_string()))?;
                let name = state
                    .financial_system
                    .instruments
                    .get(&inst_id)
                    .map_or("N/A".to_string(), |i| i.type_as_string().to_string());
                Ok(MarketDetailDto { market_id: market_id_str.to_string(), name, order_book: market.book.clone() })
            }
            MarketId::Goods(good_id) => {
                let market = state
                    .financial_system
                    .exchange
                    .goods_markets
                    .get(&good_id)
                    .ok_or((axum::http::StatusCode::NOT_FOUND, "Goods market not found".to_string()))?;
                let name =
                    state.financial_system.goods.goods.get(&good_id).map_or("N/A".to_string(), |g| g.name.clone());
                Ok(MarketDetailDto { market_id: market_id_str.to_string(), name, order_book: market.book.clone() })
            }
            MarketId::Labour(id) => Ok(MarketDetailDto {
                market_id: MarketId::Labour(id).to_string(),
                name: "Labour Market".to_string(),
                order_book: OrderBook::default(),
            }),
        }
    }

    pub fn get_market_history(&self, market_id_str: &str) -> QueryResult<Vec<MarketTick>> {
        let engine = self.get_engine_lock()?;
        let state = &engine.state;
        let market_id: MarketId = market_id_str.parse().map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;

        let history = state
            .history
            .market_ticks
            .get(&market_id)
            .map(|deque| deque.iter().cloned().collect())
            .unwrap_or_else(Vec::new);

        Ok(history)
    }

    pub fn get_ticks_summary(&self) -> QueryResult<Vec<TickSummaryDto>> {
        let engine = self.get_engine_lock()?;
        let summaries = engine
            .state
            .history
            .tick_records
            .iter()
            .map(|record| TickSummaryDto {
                tick_number: record.tick_number,
                date: record.date.format("%Y-%m-%d").to_string(),
                intentions: record.intentions.len(),
                actions: record.actions.len(),
                effects: record.effects.len(),
                trades: record.trades.len(),
            })
            .collect();
        Ok(summaries)
    }

    pub fn get_tick_detail(&self, tick_number: u32) -> QueryResult<TickDetailDto> {
        let engine = self.get_engine_lock()?;
        let record = engine
            .state
            .history
            .tick_records
            .iter()
            .find(|r| r.tick_number == tick_number)
            .ok_or((axum::http::StatusCode::NOT_FOUND, format!("Tick {} not found in history.", tick_number)))?;

        Ok(TickDetailDto {
            tick_number: record.tick_number,
            date: record.date.format("%Y-%m-%d").to_string(),
            intentions: record.intentions.clone(),
            actions: record.actions.clone(),
            effects: record.effects.clone(),
            trades: record.trades.clone(),
            action_to_effect_indices: record.action_to_effect_indices.clone(),
        })
    }
    pub fn get_exchange(&self) -> QueryResult<ExchangeDto> {
        let engine = self.get_engine_lock()?;
        let exchange = &engine.state.financial_system.exchange;
        let exchange_data = Json(ExchangeDto::from(exchange));
        Ok(exchange_data.0)
    }
    pub fn get_financial_infrastructure_state(&self) -> QueryResult<FinancialInfrastructureDto> {
        let engine_lock = self.get_engine_lock()?;
        let state = &engine_lock.state;

        let csd_dto = CsdStateDto {
            custody_accounts: state.financial_system.clearing_house.csd.custody_accounts.clone(),
            pending_settlements: state.financial_system.clearing_house.csd.pending_settlements.clone(),
            settlement_history: state.financial_system.clearing_house.csd.settlement_history.clone(),
            registered_securities: state.financial_system.clearing_house.csd.registered_securities.clone(),
        };

        let rtgs_dto = RtgsStateDto {
            pending_payments: state.financial_system.rtgs.pending.clone(),
            settled_payments: state.financial_system.rtgs.settled.clone(),
            rejected_payments: state.financial_system.rtgs.rejected.clone(),
        };

        Ok(FinancialInfrastructureDto { csd: csd_dto, rtgs: rtgs_dto })
    }
    pub fn get_credit_registry(&self) -> QueryResult<CreditRegistryDto> {
        let engine_lock = self.get_engine_lock()?;
        let state = &engine_lock.state;
        let reg = &state.financial_system.credit_registry.clone();
        Ok({
            CreditRegistryDto {
                applications: reg.applications.clone(),
                loans: reg.loans.clone(),
                loans_by_borrower: reg.loans_by_borrower.clone(),
                loans_by_lender: reg.loans_by_lender.clone(),
                applications_by_bank: reg.applications_by_bank.clone(),
                credit_histories: reg.credit_histories.clone(),
            }
        })
    }
}