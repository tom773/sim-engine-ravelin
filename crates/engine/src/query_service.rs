use crate::broadcast::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use surrealdb::engine::remote::ws::{Client as WsClient, Ws};
use surrealdb::{Result as SurrealResult, Surreal};

pub struct QueryService {
    db: Surreal<WsClient>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DashboardData {
    pub current_date: String,
    pub tick_number: u32,
    pub agent_counts: AgentCounts,
    pub economic_stats: EconomicStats,
    pub overnight_rates: OvernightRatesData,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentCounts {
    pub banks: usize,
    pub firms: usize,
    pub consumers: usize,
    pub total: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EconomicStats {
    pub nominal_gdp_proxy: f64,
    pub consumer_spending_daily: f64,
    pub cpi: f64,
    pub unemployment_rate: f64,
    pub m0: f64,
    pub m1: f64,
    pub m2: f64,

    pub ppi: f64,
    pub inflation_rate: f64,
    pub labor_force_participation: f64,
    pub job_openings: f64,
    pub household_debt: f64,
    pub corporate_debt: f64,
    pub government_debt: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OvernightRatesData {
    pub effr: Option<f64>,
    pub sofr: Option<f64>,
    pub iorb: Option<f64>,
    pub discount_rate: Option<f64>,
    pub overnight_rrp: Option<f64>,
}

impl QueryService {
    pub async fn connect() -> SurrealResult<Self> {
        let db = Surreal::new::<Ws>("localhost:8000").await?;
        db.signin(surrealdb::opt::auth::Root { username: "root", password: "root" }).await?;
        db.use_ns("research").use_db("sim").await?;
        Ok(Self { db })
    }

    pub async fn get_dashboard_data(&self) -> SurrealResult<Option<DashboardData>> {
        let latest_tick: Vec<TickRecordB> =
            self.db.query("SELECT * FROM tick ORDER BY tick_number DESC LIMIT 1").await?.take(0)?;

        let tick = match latest_tick.first() {
            Some(t) => t,
            None => return Ok(None),
        };

        let tick_id_str = tick.id.as_ref().unwrap().to_string();

        let macro_stats: Vec<MacroStatsRecord> = self
            .db
            .query("SELECT * FROM macro_stats WHERE tick = type::thing($tick_id) LIMIT 1")
            .bind(("tick_id", tick_id_str.clone()))
            .await?
            .take(0)?;

        let stats = match macro_stats.first() {
            Some(s) => s,
            None => return Ok(None),
        };

        let agent_counts: Vec<serde_json::Value> = self
            .db
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

        let dashboard = DashboardData {
            current_date: tick.ts.to_string().split('T').next().unwrap_or("").to_string(),
            tick_number: tick.tick_number,
            agent_counts: AgentCounts {
                banks: counts.get("Bank").copied().unwrap_or(0),
                firms: counts.get("Firm").copied().unwrap_or(0),
                consumers: counts.get("Consumer").copied().unwrap_or(0),
                total: counts.values().sum(),
            },
            economic_stats: EconomicStats {
                nominal_gdp_proxy: stats.nominal_gdp_proxy,
                consumer_spending_daily: stats.consumer_spending_daily,
                cpi: stats.cpi,
                unemployment_rate: stats.unemployment_rate,
                m0: stats.m0,
                m1: stats.m1,
                m2: stats.m2,
                ppi: stats.ppi,
                inflation_rate: stats.inflation_rate,
                labor_force_participation: stats.labor_force_participation,
                job_openings: stats.job_openings,
                household_debt: stats.household_debt,
                corporate_debt: stats.corporate_debt,
                government_debt: stats.government_debt,
            },
            overnight_rates: OvernightRatesData {
                effr: None, // TODO: Query from market data
                sofr: None,
                iorb: None,
                discount_rate: None,
                overnight_rrp: None,
            },
        };

        Ok(Some(dashboard))
    }

    pub async fn get_agent_summaries(
        &self, agent_type: &str, page: u32, page_size: u32,
    ) -> SurrealResult<(Vec<AgentSummaryData>, u64)> {
        let offset = (page - 1) * page_size;
        let agent_type_owned = agent_type.to_string();

        let latest_tick_query: Vec<serde_json::Value> =
            self.db.query("SELECT VALUE id FROM tick ORDER BY tick_number DESC LIMIT 1").await?.take(0)?;

        let tick_id_str = match latest_tick_query.first() {
            Some(id) => id.as_str().unwrap_or("").to_string(),
            None => return Ok((vec![], 0)),
        };

        let agents: Vec<AgentStateRecord> = self
            .db
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
            .db
            .query("SELECT count() FROM agent_state WHERE agent_type = $agent_type AND tick = type::thing($tick_id)")
            .bind(("agent_type", agent_type_owned))
            .bind(("tick_id", tick_id_str))
            .await?
            .take(0)?;

        let total_count = count_result.first().and_then(|v| v.get("count")).and_then(|v| v.as_u64()).unwrap_or(0);

        let summaries: Vec<AgentSummaryData> = agents
            .into_iter()
            .map(|agent| AgentSummaryData {
                id: agent.agent.clone(),
                agent_type: agent.agent_type.clone(),
                total_assets: agent.total_assets,
                total_liabilities: agent.total_liabilities,
                net_worth: agent.net_worth,
                liquid_assets: agent.liquid_assets,
            })
            .collect();

        Ok((summaries, total_count))
    }

    pub async fn get_market_summaries(&self) -> SurrealResult<Vec<MarketSummaryData>> {
        let latest_tick_query: Vec<serde_json::Value> =
            self.db.query("SELECT VALUE id FROM tick ORDER BY tick_number DESC LIMIT 1").await?.take(0)?;

        let tick_id_str = match latest_tick_query.first() {
            Some(id) => id.as_str().unwrap_or("").to_string(),
            None => return Ok(vec![]),
        };

        let summaries: Vec<MarketSummaryRecord> = self
            .db
            .query("SELECT * FROM market_summary WHERE tick = type::thing($tick_id)")
            .bind(("tick_id", tick_id_str))
            .await?
            .take(0)?;

        let market_data: Vec<MarketSummaryData> = summaries
            .into_iter()
            .map(|record| MarketSummaryData {
                market_id: record.market_id,
                market_type: record.market_type,
                best_bid: record.best_bid,
                best_ask: record.best_ask,
                mid_price: record.mid_price,
                spread: record.spread,
                volume_24h: record.volume_24h,
                last_price: record.last_price,
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

        let ticks: Vec<TickRecordB> = self.db.query(&query).await?.take(0)?;

        let history: Vec<TickHistoryData> = ticks
            .into_iter()
            .map(|tick| TickHistoryData {
                tick_number: tick.tick_number,
                date: tick.ts.to_string().split('T').next().unwrap_or("").to_string(),
            })
            .collect();

        Ok(history)
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
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TickHistoryData {
    pub tick_number: u32,
    pub date: String,
}
