use crate::*;
use sim_core::prelude::*;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use axum::response::Json;
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

    fn populate_balance_sheet(&self, bs: &BalanceSheet, state: &SimState) -> PopulatedBalanceSheetDto {
        let assets = bs.assets.iter().filter_map(|(id, pos)| {
            state.financial_system.instruments.get(id).map(|inst| {
                let market_price = state.financial_system.exchange
                    .financial_market(id)
                    .and_then(|book| book.representative_price());

                PopulatedPositionDto {
                    position: pos.clone(),
                    instrument: inst.clone(),
                    market_price,
                }
            })
        }).collect();

        let liabilities = bs.liabilities.iter().filter_map(|(id, pos)| {
            state.financial_system.instruments.get(id).map(|inst| {
                 let market_price = state.financial_system.exchange
                    .financial_market(id)
                    .and_then(|book| book.representative_price());

                PopulatedPositionDto {
                    position: pos.clone(),
                    instrument: inst.clone(),
                    market_price,
                }
            })
        }).collect();

        PopulatedBalanceSheetDto { assets, liabilities }
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

        let macro_stats_dto = MacroStatsDto {
            nominal_gdp_proxy: macro_stats.nominal_gdp_proxy,
            consumer_spending_daily: macro_stats.consumer_spending_daily,
            cpi: macro_stats.cpi,
            inflation_rate: macro_stats.inflation_rate,
            unemployment_rate: macro_stats.unemployment_rate,
            m0: macro_stats.m0, m1: macro_stats.m1, m2: macro_stats.m2,
            bank_reserves: macro_stats.bank_reserves,
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
            reserve_requirement: Rate::from_f64(state.financial_system.central_bank.reserve_requirement).unwrap_or_default(),
            overnight_rates: on_rates,
        };
        Ok(StatusDto {
            current_date: state.current_date.format("%Y-%m-%d").to_string(),
            tick_number: state.ticknum,
            total_iterations: state.config.iterations,
            agent_counts,
            macro_stats: macro_stats_dto,
            monetary_stats: monetary_stats_dto,
        })
    }

    pub fn get_agents_summary(&self, agent_type_filter: Option<String>) -> QueryResult<Vec<AgentSummaryDto>> {
        let engine = self.get_engine_lock()?;
        let state = &engine.state;
        let fs = &state.financial_system;
        
        let mut summaries = Vec::new();
        let filter_lower = agent_type_filter.as_deref().map(str::to_lowercase);

        let mut add_summary_for_agent = |id: &AgentId| {
            let (agent_type_str, name_opt) = engine.get_agent_info(id);
            let bs = fs.balance_sheets.get(id).unwrap();
            let populated_bs = self.populate_balance_sheet(bs, state);
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
        
        let raw_balance_sheet = state.financial_system.balance_sheets.get(&agent_id)
            .ok_or((axum::http::StatusCode::NOT_FOUND, format!("Balance sheet for agent {} not found", agent_id)))?;

        let populated_bs = self.populate_balance_sheet(raw_balance_sheet, state);

        Ok(AgentDetailDto {
            id: agent_id.0,
            agent_type,
            name: name.unwrap_or_else(|| "N/A".to_string()),
            balance_sheet: populated_bs,
        })
    }
    pub fn get_market_overiew(&self) -> QueryResult<MarketOverviewDto> {
        let engine = self.get_engine_lock()?;
        let state = &engine.state;
        Ok(MarketOverviewDto {
            instrument_registry: state.financial_system.instruments.clone(),
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
                    if let InstrumentType::Bond(d) = &inst.instrument_type {
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
                    } else { (None, None, None, None) }
                } else { (None, None, None, None) };


            summaries.push(MarketSummaryDto {
                market_id: market_id.to_string(),
                market_type: "financial".to_string(),
                name: state.financial_system.instruments.get(inst_id).map_or("Unknown".to_string(), |i| i.type_as_string().to_string()),
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

            let bid_levels = depth_summary.bid_levels.into_iter()
                .map(|(k, v)| (k.into_inner().to_string(), v))
                .collect();
            let ask_levels = depth_summary.ask_levels.into_iter()
                .map(|(k, v)| (k.into_inner().to_string(), v))
                .collect();

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
               last_price: None, mid_price: None, best_bid: None, best_ask: None, spread: None,
               volume_24h: 0.0, turnover_24h: 0.0, depth: None,
               best_bid_yield: None,
               best_ask_yield: None,
               mid_yield: None,
               last_yield: None,
           });
       }

        Ok(summaries)
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
        Ok(InstrumentRegistryDto {
            instruments: engine.state.financial_system.instruments.clone(),
        })
    }
    pub fn get_market_detail(&self, market_id_str: &str) -> QueryResult<MarketDetailDto> {
        let engine = self.get_engine_lock()?;
        let state = &engine.state;
        let market_id: MarketId = market_id_str.parse().map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;

        match market_id {
            MarketId::Financial(inst_id) => {
                let market = state.financial_system.exchange.markets.get(&inst_id)
                    .ok_or((axum::http::StatusCode::NOT_FOUND, "Financial market not found".to_string()))?;
                let name = state.financial_system.instruments.get(&inst_id).map_or("N/A".to_string(), |i| i.type_as_string().to_string());
                Ok(MarketDetailDto { market_id: market_id_str.to_string(), name, order_book: market.book.clone() })
            },
            MarketId::Goods(good_id) => {
                let market = state.financial_system.exchange.goods_markets.get(&good_id)
                    .ok_or((axum::http::StatusCode::NOT_FOUND, "Goods market not found".to_string()))?;
                let name = state.financial_system.goods.goods.get(&good_id).map_or("N/A".to_string(), |g| g.name.clone());
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

        let history = state.history.market_ticks.get(&market_id)
            .map(|deque| deque.iter().cloned().collect())
            .unwrap_or_else(Vec::new);
            
        Ok(history)
    }

    pub fn get_ticks_summary(&self) -> QueryResult<Vec<TickSummaryDto>> {
        let engine = self.get_engine_lock()?;
        let summaries = engine.state.history.tick_records.iter().map(|record| TickSummaryDto {
            tick_number: record.tick_number,
            date: record.date.format("%Y-%m-%d").to_string(),
            intentions: record.intentions.len(),
            actions: record.actions.len(),
            effects: record.effects.len(),
            trades: record.trades.len(),
        }).collect();
        Ok(summaries)
    }
    
    pub fn get_tick_detail(&self, tick_number: u32) -> QueryResult<TickDetailDto> {
        let engine = self.get_engine_lock()?;
        let record = engine.state.history.tick_records.iter()
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
}