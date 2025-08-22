use serde::{Deserialize, Serialize};
use sim_core::*;
use surrealdb::engine::remote::ws::{Client as WsClient, Ws};
use surrealdb::{Result as SurrealResult, Surreal};


#[derive(Serialize, Deserialize, Clone)]
pub struct TickRecordB {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<surrealdb::sql::Thing>,
    pub tick_number: u32,
    pub ts: surrealdb::sql::Datetime,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EffectRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<surrealdb::sql::Thing>,
    pub tick: surrealdb::sql::Thing,
    pub kind: String,
    pub payload: serde_json::Value,
    pub ts: surrealdb::sql::Datetime,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InstrumentStateRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<surrealdb::sql::Thing>,
    pub instrument: String,
    pub tick: surrealdb::sql::Thing,
    pub principal: f64,
    pub accr_int: f64,
    pub details: serde_json::Value,
    pub debtor: String,
    pub creditor: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentStateRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<surrealdb::sql::Thing>,
    pub agent: String,
    pub tick: surrealdb::sql::Thing,
    pub agent_type: String,
    pub total_assets: f64,
    pub total_liabilities: f64,
    pub net_worth: f64,
    pub liquid_assets: f64,
    pub details: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TradeRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<surrealdb::sql::Thing>,
    pub tick: surrealdb::sql::Thing,
    pub market_id: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub quantity: f64,
    pub price: f64,
    pub ts: surrealdb::sql::Datetime,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TickSummaryRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<surrealdb::sql::Thing>,
    pub tick: surrealdb::sql::Thing,
    pub totals: serde_json::Value,
}

pub struct SurrealDbWriter {
    db: Surreal<WsClient>,
}

impl SurrealDbWriter {
    pub async fn connect() -> SurrealResult<Self> {
        let db = Surreal::new::<Ws>("localhost:8000").await?;
        db.signin(surrealdb::opt::auth::Root { 
            username: "root", 
            password: "root" 
        }).await?;
        db.use_ns("research").use_db("sim").await?;
        
        let writer = Self { db };
        writer.clear_database().await?;
        writer.setup_schema().await?;
        Ok(writer)
    }

    async fn clear_database(&self) -> SurrealResult<()> {
        self.db.query("
            REMOVE TABLE tick;
            REMOVE TABLE effect;
            REMOVE TABLE instrument_state;
            REMOVE TABLE agent_state;
            REMOVE TABLE trade;
            REMOVE TABLE tick_summary;
        ").await?;
        Ok(())
    }

    async fn setup_schema(&self) -> SurrealResult<()> {
        self.db.query("
            DEFINE TABLE tick SCHEMAFULL;
            DEFINE FIELD tick_number ON tick TYPE int ASSERT $value >= 0;
            DEFINE FIELD ts ON tick TYPE datetime;
            DEFINE INDEX tick_number_unique ON TABLE tick FIELDS tick_number UNIQUE;
        ").await?;

        self.db.query("
            DEFINE TABLE effect SCHEMALESS;
            DEFINE FIELD tick ON effect TYPE record<tick>;
            DEFINE FIELD kind ON effect TYPE string;
            DEFINE FIELD payload ON effect TYPE object;
            DEFINE FIELD ts ON effect TYPE datetime;
            DEFINE INDEX effect_by_tick ON TABLE effect FIELDS tick;
        ").await?;

        self.db.query("
            DEFINE TABLE instrument_state SCHEMALESS;
            DEFINE FIELD instrument ON instrument_state TYPE string;
            DEFINE FIELD tick ON instrument_state TYPE record<tick>;
            DEFINE FIELD principal ON instrument_state TYPE number;
            DEFINE FIELD accr_int ON instrument_state TYPE number;
            DEFINE FIELD details ON instrument_state TYPE object;
            DEFINE FIELD debtor ON instrument_state TYPE string;
            DEFINE FIELD creditor ON instrument_state TYPE string;
            DEFINE INDEX inst_state_by_tick ON TABLE instrument_state FIELDS tick;
            DEFINE INDEX inst_state_by_inst_tick ON TABLE instrument_state FIELDS instrument, tick UNIQUE;
        ").await?;

        self.db.query("
            DEFINE TABLE agent_state SCHEMALESS;
            DEFINE FIELD agent ON agent_state TYPE string;
            DEFINE FIELD tick ON agent_state TYPE record<tick>;
            DEFINE FIELD agent_type ON agent_state TYPE string;
            DEFINE FIELD total_assets ON agent_state TYPE number;
            DEFINE FIELD total_liabilities ON agent_state TYPE number;
            DEFINE FIELD net_worth ON agent_state TYPE number;
            DEFINE FIELD liquid_assets ON agent_state TYPE number;
            DEFINE FIELD details ON agent_state TYPE object;
            DEFINE INDEX agent_state_by_tick ON TABLE agent_state FIELDS tick;
            DEFINE INDEX agent_state_by_agent_tick ON TABLE agent_state FIELDS agent, tick UNIQUE;
        ").await?;

        self.db.query("
            DEFINE TABLE trade SCHEMAFULL;
            DEFINE FIELD tick ON trade TYPE record<tick>;
            DEFINE FIELD market_id ON trade TYPE string;
            DEFINE FIELD buyer_id ON trade TYPE string;
            DEFINE FIELD seller_id ON trade TYPE string;
            DEFINE FIELD quantity ON trade TYPE float ASSERT $value > 0;
            DEFINE FIELD price ON trade TYPE float ASSERT $value > 0;
            DEFINE FIELD ts ON trade TYPE datetime;
            DEFINE INDEX trade_by_tick ON TABLE trade FIELDS tick;
            DEFINE INDEX trade_by_market ON TABLE trade FIELDS market_id;
        ").await?;

        self.db.query("
            DEFINE TABLE tick_summary SCHEMAFULL;
            DEFINE FIELD tick ON tick_summary TYPE record<tick>;
            DEFINE FIELD totals ON tick_summary TYPE object;
            DEFINE INDEX tick_summary_by_tick ON TABLE tick_summary FIELDS tick UNIQUE;
        ").await?;

        Ok(())
    }

    pub async fn write_tick_batch(
        &self,
        tick_number: u32,
        effects: &[StateEffect],
        trades: &[Trade],
        instruments: &[(InstrumentId, &FinancialInstrument)],
        agents: &[(AgentId, String, &BalanceSheet)],
    ) -> SurrealResult<()> {
        let now = surrealdb::sql::Datetime::from(chrono::Utc::now());

        let tick_record = TickRecordB {
            id: None,
            tick_number,
            ts: now.clone(),
        };
        let tick_result: Option<TickRecordB> = self.db
            .create("tick")
            .content(tick_record)
            .await?;
        
        let tick_ref = tick_result
            .and_then(|t| t.id)
            .unwrap_or_else(|| {
                surrealdb::sql::Thing::from(("tick", tick_number.to_string().as_str()))
            });

        if !effects.is_empty() {
            let effect_records: Vec<EffectRecord> = effects.iter().map(|effect| {
                EffectRecord {
                    id: None,
                    tick: tick_ref.clone(),
                    kind: effect.name(),
                    payload: serde_json::to_value(effect).unwrap_or(serde_json::Value::Null),
                    ts: now.clone(),
                }
            }).collect();

            let _: Vec<EffectRecord> = self.db
                .query("INSERT INTO effect $data")
                .bind(("data", effect_records))
                .await?
                .take(0)?;
        }

        if !trades.is_empty() {
            let trade_records: Vec<TradeRecord> = trades.iter().map(|trade| {
                TradeRecord {
                    id: None,
                    tick: tick_ref.clone(),
                    market_id: trade.market_id.to_string(),
                    buyer_id: trade.buyer.to_string(),
                    seller_id: trade.seller.to_string(),
                    quantity: trade.quantity,
                    price: trade.price,
                    ts: now.clone(),
                }
            }).collect();

            let _: Vec<TradeRecord> = self.db
                .query("INSERT INTO trade $data")
                .bind(("data", trade_records))
                .await?
                .take(0)?;
        }

        if !instruments.is_empty() {
            let instrument_records: Vec<InstrumentStateRecord> = instruments.iter().map(|(id, instrument)| {
                InstrumentStateRecord {
                    id: None,
                    instrument: id.to_string(),
                    tick: tick_ref.clone(),
                    principal: instrument.principal,
                    accr_int: instrument.accrued_interest,
                    debtor: instrument.debtor.to_string(),
                    creditor: instrument.creditor.to_string(),
                    details: serde_json::to_value(instrument.details.as_ref()).unwrap_or(serde_json::Value::Null),
                }
            }).collect();

            let _: Vec<InstrumentStateRecord> = self.db
                .query("INSERT INTO instrument_state $data")
                .bind(("data", instrument_records))
                .await?
                .take(0)?;
        }

        if !agents.is_empty() {
            let agent_records: Vec<AgentStateRecord> = agents.iter().map(|(id, agent_type, bs)| {
                AgentStateRecord {
                    id: None,
                    agent: id.to_string(),
                    tick: tick_ref.clone(),
                    agent_type: agent_type.clone(),
                    total_assets: bs.total_assets(),
                    total_liabilities: bs.total_liabilities(),
                    net_worth: bs.net_worth(),
                    liquid_assets: bs.liquid_assets(),
                    details: serde_json::json!({
                        "cash_assets": bs.liquid_assets(),
                        "deposit_assets": bs.total_deposits(),
                    }),
                }
            }).collect();

            let _: Vec<AgentStateRecord> = self.db
                .query("INSERT INTO agent_state $data")
                .bind(("data", agent_records))
                .await?
                .take(0)?;
        }

        let summary_totals = serde_json::json!({
            "effect_count": effects.len(),
            "trade_count": trades.len(),
            "instrument_count": instruments.len(),
            "agent_count": agents.len(),
            "total_trade_volume": trades.iter().map(|t| t.quantity).sum::<f64>(),
            "total_trade_value": trades.iter().map(|t| t.quantity * t.price).sum::<f64>(),
        });

        let summary_record = TickSummaryRecord {
            id: None,
            tick: tick_ref.clone(),
            totals: summary_totals,
        };

        let _: Option<TickSummaryRecord> = self.db
            .create("tick_summary")
            .content(summary_record)
            .await?;

        Ok(())
    }

    pub async fn cleanup_old_data(&self, keep_ticks: u32) -> SurrealResult<()> {
        let _: Result<surrealdb::Response, surrealdb::Error> = self.db
            .query("
                LET $cutoff_tick = (
                    SELECT VALUE tick_number 
                    FROM tick 
                    ORDER BY tick_number DESC 
                    LIMIT 1
                )[0] - $keep_ticks;
                
                DELETE tick WHERE tick_number < $cutoff_tick;
                DELETE effect WHERE tick IN (SELECT id FROM tick WHERE tick_number < $cutoff_tick);
                DELETE instrument_state WHERE tick IN (SELECT id FROM tick WHERE tick_number < $cutoff_tick);
                DELETE agent_state WHERE tick IN (SELECT id FROM tick WHERE tick_number < $cutoff_tick);
                DELETE trade WHERE tick IN (SELECT id FROM tick WHERE tick_number < $cutoff_tick);
                DELETE tick_summary WHERE tick IN (SELECT id FROM tick WHERE tick_number < $cutoff_tick);
            ")
            .bind(("keep_ticks", keep_ticks))
            .await;

        Ok(())
    }

    pub async fn get_instruments_at_tick(&self, tick_number: u32) -> SurrealResult<Vec<InstrumentStateRecord>> {
        let instruments: Vec<InstrumentStateRecord> = self.db
            .query("SELECT * FROM instrument_state WHERE tick = (SELECT id FROM tick WHERE tick_number = $tick_number)[0]")
            .bind(("tick_number", tick_number))
            .await?
            .take(0)?;
        Ok(instruments)
    }

    pub async fn get_instrument_history(&self, instrument_id: &str, tick_from: u32, tick_to: u32) -> SurrealResult<Vec<InstrumentStateRecord>> {
        let instrument_id = instrument_id.to_string();
        let history: Vec<InstrumentStateRecord> = self.db
            .query("
                SELECT * FROM instrument_state 
                WHERE instrument = $instrument_id 
                AND tick IN (SELECT id FROM tick WHERE tick_number BETWEEN $tick_from AND $tick_to)
                ORDER BY tick.tick_number
            ")
            .bind(("instrument_id", instrument_id))
            .bind(("tick_from", tick_from))
            .bind(("tick_to", tick_to))
            .await?
            .take(0)?;
        Ok(history)
    }

    pub async fn get_effects_for_tick(&self, tick_number: u32) -> SurrealResult<Vec<EffectRecord>> {
        let effects: Vec<EffectRecord> = self.db
            .query("SELECT * FROM effect WHERE tick = (SELECT id FROM tick WHERE tick_number = $tick_number)[0] ORDER BY ts")
            .bind(("tick_number", tick_number))
            .await?
            .take(0)?;
        Ok(effects)
    }

    pub async fn get_trades_for_market_range(&self, market_id: &str, tick_from: u32, tick_to: u32) -> SurrealResult<Vec<TradeRecord>> {
        let market_id = market_id.to_string();
        let trades: Vec<TradeRecord> = self.db
            .query("
                SELECT * FROM trade 
                WHERE market_id = $market_id 
                AND tick IN (SELECT id FROM tick WHERE tick_number BETWEEN $tick_from AND $tick_to)
                ORDER BY tick.tick_number, ts
            ")
            .bind(("market_id", market_id))
            .bind(("tick_from", tick_from))
            .bind(("tick_to", tick_to))
            .await?
            .take(0)?;
        Ok(trades)
    }
}