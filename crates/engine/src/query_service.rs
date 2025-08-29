use crate::broadcast::*;
use crate::dto::query_dto::*;
use serde::de::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use surrealdb::engine::remote::ws::Client as WsClient;
use surrealdb::{Result as SurrealResult, Surreal};

use sim_core::BalanceSheet;

pub struct QueryService {
    writer: SurrealDbWriter,
}

impl QueryService {
    pub async fn connect_and_init(
        goods: &HashMap<sim_core::GoodId, sim_core::goods::Good>,
        recipes: &HashMap<sim_core::RecipeId, sim_core::goods::ProductionRecipe>,
    ) -> SurrealResult<Self> {
        let writer = SurrealDbWriter::connect_and_initialize(goods, recipes).await?;
        Ok(Self { writer })
    }

    pub fn get_writer(&self) -> SurrealDbWriter {
        self.writer.clone()
    }

    fn db(&self) -> &Surreal<WsClient> {
        &self.writer.db
    }

    pub async fn get_dashboard_data(&self) -> SurrealResult<Option<QueryServiceDashboardData>> {
        let latest_tick: Vec<TickRecordB> =
            self.db().query("SELECT * FROM tick ORDER BY tick_number DESC LIMIT 1").await?.take(0)?;

        let tick = match latest_tick.first() {
            Some(t) => t,
            None => return Ok(None),
        };

        let tick_id_str = tick.id.as_ref().unwrap().to_string();

        let macro_stats: Vec<MacroStatsRecord> = self
            .db()
            .query("SELECT * FROM macro_stats WHERE tick = type::thing($tick_id) LIMIT 1")
            .bind(("tick_id", tick_id_str.clone()))
            .await?
            .take(0)?;

        let stats = match macro_stats.first() {
            Some(s) => s,
            None => return Ok(None),
        };

        let agent_counts: Vec<serde_json::Value> = self
            .db()
            .query(
                r#"
                SELECT 
                    agent_type, 
                    count() as count 
                FROM agent_state 
                WHERE tick = type::thing($tick_id) 
                GROUP BY agent_type
            "#,
            )
            .bind(("tick_id", tick_id_str))
            .await?
            .take(0)?;

        let mut counts = HashMap::new();
        for item in agent_counts {
            if let (Some(agent_type), Some(count)) =
                (item.get("agent_type").and_then(|v| v.as_str()), item.get("count").and_then(|v| v.as_u64()))
            {
                counts.insert(agent_type.to_string(), count as usize);
            }
        }

        let dashboard = QueryServiceDashboardData {
            current_date: tick.sim_date.clone(), // Use the correct sim_date field
            tick_number: tick.tick_number,
            agent_counts: QueryServiceAgentCounts {
                banks: counts.get("Bank").copied().unwrap_or(0),
                firms: counts.get("Firm").copied().unwrap_or(0),
                consumers: counts.get("Consumer").copied().unwrap_or(0),
                total: counts.values().sum(),
            },
            economic_stats: QueryServiceEconomicStats {
                nominal_gdp_proxy: stats.nominal_gdp_proxy,
                consumer_spending_daily: stats.consumer_spending_daily,
                cpi: stats.cpi,
                ppi: stats.ppi,
                inflation_rate: stats.inflation_rate,
                unemployment_rate: stats.unemployment_rate,
                labor_force_participation: stats.labor_force_participation,
                job_openings: stats.job_openings,
                household_debt: stats.household_debt,
                corporate_debt: stats.corporate_debt,
                government_debt: stats.government_debt,
                bank_reserves: stats.bank_reserves,
                bank_credit: stats.bank_credit,
                bank_liabilities: stats.bank_liabilities,
                m0: stats.m0,
                m1: stats.m1,
                m2: stats.m2,
                overnight_rates: QueryServiceOvernightRates {
                    effr: stats.overnight_rates.effr,
                    sofr: stats.overnight_rates.sofr,
                    iorb: Some(stats.overnight_rates.iorb),
                    discount_rate: Some(stats.overnight_rates.discount_rate),
                    overnight_rrp: Some(stats.overnight_rates.overnight_rrp),
                },
            },
        };

        Ok(Some(dashboard))
    }

    pub async fn get_stats_data(&self) -> SurrealResult<Option<QueryServiceStatsData>> {
        let latest_tick: Vec<TickRecordB> =
            self.db().query("SELECT * FROM tick ORDER BY tick_number DESC LIMIT 1").await?.take(0)?;

        let tick = match latest_tick.first() {
            Some(t) => t,
            None => return Ok(None),
        };

        let tick_id_str = tick.id.as_ref().unwrap().to_string();

        let macro_stats: Vec<MacroStatsRecord> = self
            .db()
            .query("SELECT * FROM macro_stats WHERE tick = type::thing($tick_id) LIMIT 1")
            .bind(("tick_id", tick_id_str))
            .await?
            .take(0)?;

        let stats = match macro_stats.first() {
            Some(s) => s,
            None => return Ok(None),
        };

        let stats_data = QueryServiceStatsData {
            tick_number: tick.tick_number,
            economic_stats: QueryServiceEconomicStats {
                nominal_gdp_proxy: stats.nominal_gdp_proxy,
                consumer_spending_daily: stats.consumer_spending_daily,
                cpi: stats.cpi,
                ppi: stats.ppi,
                inflation_rate: stats.inflation_rate,
                unemployment_rate: stats.unemployment_rate,
                labor_force_participation: stats.labor_force_participation,
                job_openings: stats.job_openings,
                household_debt: stats.household_debt,
                corporate_debt: stats.corporate_debt,
                government_debt: stats.government_debt,
                bank_reserves: stats.bank_reserves,
                bank_credit: stats.bank_credit,
                bank_liabilities: stats.bank_liabilities,
                overnight_rates: QueryServiceOvernightRates {
                    effr: stats.overnight_rates.effr,
                    sofr: stats.overnight_rates.sofr,
                    iorb: Some(stats.overnight_rates.iorb),
                    discount_rate: Some(stats.overnight_rates.discount_rate),
                    overnight_rrp: Some(stats.overnight_rates.overnight_rrp),
                },
                m0: stats.m0,
                m1: stats.m1,
                m2: stats.m2,
            },
        };

        Ok(Some(stats_data))
    }

    pub async fn get_agent_summaries(
        &self, agent_type: &str, page: u32, page_size: u32,
    ) -> SurrealResult<(Vec<AgentSummaryData>, u64)> {
        let offset = (page - 1) * page_size;
        let agent_type_owned = agent_type.to_string();

        let tick_id_str = match get_latest_tick_id(self.db()).await? {
            Some(id) => id,
            None => return Ok((vec![], 0)),
        };
        let agents: Vec<AgentStateRecord> = self
            .db()
            .query(
                r#"
                SELECT * FROM agent_state 
                WHERE agent_type = $agent_type AND tick = type::thing($tick_id)
                ORDER BY agent 
                LIMIT $limit 
                START $offset
            "#,
            )
            .bind(("agent_type", agent_type_owned.clone()))
            .bind(("tick_id", tick_id_str.clone()))
            .bind(("limit", page_size as i64))
            .bind(("offset", offset as i64))
            .await?
            .take(0)?;

        let count_result: Vec<serde_json::Value> = self
            .db()
            .query("SELECT count() FROM agent_state WHERE agent_type = $agent_type AND tick = type::thing($tick_id)")
            .bind(("agent_type", agent_type_owned.to_string()))
            .bind(("tick_id", tick_id_str.to_string()))
            .await?
            .take(0)?;

        let total_count = count_result.first().and_then(|v| v.get("count")).and_then(|v| v.as_u64()).unwrap_or(0);

        let summaries: Vec<AgentSummaryData> = agents
            .into_iter()
            .map(|agent| {
                // Deserialize the balance_sheet JSON to the proper type
                let balance_sheet: BalanceSheet = agent.balance_sheet.clone();

                AgentSummaryData {
                    id: agent.agent.clone(),
                    agent_type: agent.agent_type.clone(),
                    total_assets: agent.total_assets,
                    total_liabilities: agent.total_liabilities,
                    net_worth: agent.net_worth,
                    liquid_assets: agent.liquid_assets,
                    balance_sheet: Some(balance_sheet), // <-- Assign the deserialized balance sheet
                }
            })
            .collect();

        Ok((summaries, total_count))
    }

    pub async fn get_market_summaries(&self) -> SurrealResult<Vec<MarketSummaryData>> {
        let latest: Vec<TickRecordB> =
            self.db().query("SELECT * FROM tick ORDER BY tick_number DESC LIMIT 1").await?.take(0)?;
        let tick_id_str = latest.first().and_then(|t| t.id.as_ref()).map(|t| t.clone());
        let summaries: Vec<MarketSummaryRecord> = self
            .db()
            .query("SELECT * FROM market_summary WHERE tick = type::thing($tick_id)")
            .bind(("tick_id", tick_id_str))
            .await?
            .take(0)?;

        let market_data: Vec<MarketSummaryData> = summaries
            .into_iter()
            .map(|record| {
                MarketSummaryData {
                    market_id: record.market_id,
                    market_type: record.market_type,
                    best_bid: record.best_bid,
                    best_ask: record.best_ask,
                    mid_price: record.mid_price,
                    spread: record.spread,
                    volume_24h: record.volume_24h,
                    last_price: record.last_price,
                    depth: record.depth,
                    best_bid_yield: record.best_bid_yield,
                    best_ask_yield: record.best_ask_yield,
                    mid_yield: record.mid_yield,
                    last_yield: record.last_yield,
                }
            })
            .collect();
        Ok(market_data)
    }

    pub async fn get_tick_history(
        &self, limit: usize, tick_from: Option<u32>, tick_to: Option<u32>,
    ) -> SurrealResult<Vec<TickHistoryData>> {
        let mut query = "SELECT * FROM tick".to_string();
        let mut conditions = vec![];

        if tick_from.is_some() || tick_to.is_some() {
            if let Some(from) = tick_from {
                conditions.push(format!("tick_number >= {}", from));
            }
            if let Some(to) = tick_to {
                conditions.push(format!("tick_number <= {}", to));
            }
        }

        if !conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
        }

        query.push_str(&format!(" ORDER BY tick_number DESC LIMIT {}", limit));

        let ticks: Vec<TickRecordB> = self.db().query(&query).await?.take(0)?;

        let history: Vec<TickHistoryData> = ticks
            .into_iter()
            .map(|tick| TickHistoryData {
                tick_number: tick.tick_number,
                date: tick.ts.to_string().split('T').next().unwrap_or("").to_string(),
            })
            .collect();

        Ok(history)
    }

    pub async fn get_agent_balance_sheet(&self, agent_id: &str) -> SurrealResult<Option<BalanceSheet>> {
        let tick_id_str = match get_latest_tick_id(self.db()).await? {
            Some(id) => id,
            None => return Ok(None),
        };

        let result: Vec<serde_json::Value> = self.db()
            .query("SELECT VALUE balance_sheet FROM agent_state WHERE agent = $agent_id AND tick = type::thing($tick_id) LIMIT 1")
            .bind(("agent_id", agent_id.to_string()))
            .bind(("tick_id", tick_id_str))
            .await?
            .take(0)?;

        if let Some(bs_json) = result.first() {
            let bs: BalanceSheet = serde_json::from_value(bs_json.clone()).map_err(|e| {
                surrealdb::Error::Api(surrealdb::error::Api::custom(format!(
                    "BalanceSheet deserialization error: {}",
                    e
                )))
            })?;
            Ok(Some(bs))
        } else {
            Ok(None)
        }
    }

    pub async fn get_goods_catalogue(&self) -> SurrealResult<GoodsRawDto> {
        let goods_records: Vec<sim_core::goods::Good> = self.db().select("good").await?;
        let recipes_records: Vec<sim_core::goods::ProductionRecipe> = self.db().select("recipe").await?;
        Ok(GoodsRawDto { goods: goods_records, recipies: recipes_records }) // Placeholder
    }

    pub async fn get_order_book(&self, market_id: &str) -> SurrealResult<Option<OrderBookSnapshotRecord>> {
        let tick_id_str = match get_latest_tick_id(self.db()).await? {
            Some(id) => id,
            None => return Ok(None),
        };

        let result: Vec<OrderBookSnapshotRecord> = self.db()
            .query("SELECT * FROM order_book_snapshot WHERE market_id = $market_id AND tick = type::thing($tick_id) LIMIT 1")
            .bind(("market_id", market_id.to_string()))
            .bind(("tick_id", tick_id_str))
            .await?
            .take(0)?;

        Ok(result.into_iter().next())
    }

    pub async fn health_check(&self) -> SurrealResult<bool> {
        let result: Result<Vec<serde_json::Value>, _> =
            self.db().query("SELECT count() FROM tick LIMIT 1").await?.take(0);
        Ok(result.is_ok())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentSummaryData {
    pub id: String,
    pub agent_type: String,
    pub total_assets: f64,
    pub total_liabilities: f64,
    pub net_worth: f64,
    pub liquid_assets: f64,
    pub balance_sheet: Option<BalanceSheet>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MarketSummaryData {
    pub market_id: String,
    pub market_type: String,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub mid_price: Option<f64>,
    pub spread: Option<f64>,
    pub volume_24h: f64,
    pub last_price: Option<f64>,
    pub depth: Option<serde_json::Value>,
    pub best_bid_yield: Option<f64>,
    pub best_ask_yield: Option<f64>,
    pub mid_yield: Option<f64>,
    pub last_yield: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TickHistoryData {
    pub tick_number: u32,
    pub date: String,
}

#[derive(Serialize, Deserialize)]
pub struct GoodsRawDto {
    goods: Vec<sim_core::goods::Good>,
    recipies: Vec<sim_core::goods::ProductionRecipe>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FinancialMarketDetails {
    pub best_bid_yield: Option<f64>,
    pub best_ask_yield: Option<f64>,
    pub mid_yield: Option<f64>,
    pub last_yield: Option<f64>,
}
