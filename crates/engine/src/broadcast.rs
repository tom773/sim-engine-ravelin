use serde::{Deserialize, Serialize};
use sim_core::*;
use surrealdb::engine::remote::ws::{Client as WsClient, Ws};
use surrealdb::{Result as SurrealResult, Surreal};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TickRecordB {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<surrealdb::sql::Thing>,
    pub tick_number: u32,
    pub ts: surrealdb::sql::Datetime,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ActionRecordB {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<surrealdb::sql::Thing>,
    pub tick: surrealdb::sql::Thing,
    pub action_type: String,
    pub agent_id: String,
    pub agent_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    pub details: serde_json::Value,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct ImpactRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<surrealdb::sql::Thing>,
    #[serde(rename = "in")]
    pub in_: surrealdb::sql::Thing, // effect record
    pub out: surrealdb::sql::Thing, // state record that was impacted
    pub delta: serde_json::Value,
}

pub struct SurrealDbWriter {
    db: Surreal<WsClient>,
}

impl SurrealDbWriter {
    pub async fn connect() -> SurrealResult<Self> {
        let db = Surreal::new::<Ws>("localhost:8000").await?;
        db.signin(surrealdb::opt::auth::Root { username: "root", password: "root" }).await?;
        db.use_ns("research").use_db("sim").await?;
        let writer = Self { db };
        writer.clear_database().await?;
        writer.setup_schema().await?;
        Ok(writer)
    }

    async fn clear_database(&self) -> SurrealResult<()> {
        self.db
            .query(
                "
            REMOVE TABLE impacts;
            REMOVE TABLE causes;
            REMOVE TABLE tick;
            REMOVE TABLE effect;
            REMOVE TABLE action;
            REMOVE TABLE instrument_state;
            REMOVE TABLE agent_state;
            REMOVE TABLE trade;
            REMOVE TABLE tick_summary;
        ",
            )
            .await?;
        Ok(())
    }

    async fn setup_schema(&self) -> SurrealResult<()> {
        self.db.query("DEFINE TABLE tick SCHEMALESS;").await?;
        self.db.query("DEFINE FIELD tick_number ON tick TYPE int ASSERT $value >= 0;").await?;
        self.db.query("DEFINE FIELD ts ON tick TYPE datetime;").await?;
        self.db.query("DEFINE INDEX tick_number_unique ON TABLE tick FIELDS tick_number UNIQUE;").await?;
        
        self.db.query("DEFINE TABLE action SCHEMALESS;").await?;
        self.db.query("DEFINE FIELD tick ON action TYPE record<tick>;").await?;
        self.db.query("DEFINE FIELD action_type ON action TYPE string;").await?;
        self.db.query("DEFINE FIELD agent_id ON action TYPE string;").await?;
        self.db.query("DEFINE FIELD agent_type ON action TYPE string;").await?;
        self.db.query("DEFINE FIELD agent_name ON action TYPE option<string>;").await?;
        self.db.query("DEFINE FIELD details ON action TYPE object;").await?;
        self.db.query("DEFINE FIELD ts ON action TYPE datetime;").await?;
        self.db.query("DEFINE INDEX action_by_tick ON TABLE action FIELDS tick;").await?;
        self.db.query("DEFINE INDEX action_by_agent ON TABLE action FIELDS agent_id;").await?;

        self.db.query("DEFINE TABLE effect SCHEMALESS;").await?;
        self.db.query("DEFINE FIELD tick ON effect TYPE record<tick>;").await?;
        self.db.query("DEFINE FIELD kind ON effect TYPE string;").await?;
        self.db.query("DEFINE FIELD payload ON effect TYPE object;").await?;
        self.db.query("DEFINE FIELD ts ON effect TYPE datetime;").await?;
        self.db.query("DEFINE INDEX effect_by_tick ON TABLE effect FIELDS tick;").await?;

        self.db.query("DEFINE TABLE instrument_state SCHEMALESS;").await?;
        self.db.query("DEFINE FIELD instrument ON instrument_state TYPE string;").await?;
        self.db.query("DEFINE FIELD tick ON instrument_state TYPE record<tick>;").await?;
        self.db.query("DEFINE FIELD principal ON instrument_state TYPE number;").await?;
        self.db.query("DEFINE FIELD accr_int ON instrument_state TYPE number;").await?;
        self.db.query("DEFINE FIELD details ON instrument_state TYPE object;").await?;
        self.db.query("DEFINE FIELD debtor ON instrument_state TYPE string;").await?;
        self.db.query("DEFINE FIELD creditor ON instrument_state TYPE string;").await?;
        self.db.query("DEFINE INDEX inst_state_by_tick ON TABLE instrument_state FIELDS tick;").await?;
        self.db.query("DEFINE INDEX inst_state_by_inst_tick ON TABLE instrument_state FIELDS instrument, tick UNIQUE;").await?;

        self.db.query("DEFINE TABLE agent_state SCHEMALESS;").await?;
        self.db.query("DEFINE FIELD agent ON agent_state TYPE string;").await?;
        self.db.query("DEFINE FIELD tick ON agent_state TYPE record<tick>;").await?;
        self.db.query("DEFINE FIELD agent_type ON agent_state TYPE string;").await?;
        self.db.query("DEFINE FIELD total_assets ON agent_state TYPE number;").await?;
        self.db.query("DEFINE FIELD total_liabilities ON agent_state TYPE number;").await?;
        self.db.query("DEFINE FIELD net_worth ON agent_state TYPE number;").await?;
        self.db.query("DEFINE FIELD liquid_assets ON agent_state TYPE number;").await?;
        self.db.query("DEFINE FIELD details ON agent_state TYPE object;").await?;
        self.db.query("DEFINE INDEX agent_state_by_tick ON TABLE agent_state FIELDS tick;").await?;
        self.db.query("DEFINE INDEX agent_state_by_agent_tick ON TABLE agent_state FIELDS agent, tick UNIQUE;").await?;

        self.db.query("DEFINE TABLE trade SCHEMALESS;").await?;
        self.db.query("DEFINE FIELD tick ON trade TYPE record<tick>;").await?;
        self.db.query("DEFINE FIELD market_id ON trade TYPE string;").await?;
        self.db.query("DEFINE FIELD buyer_id ON trade TYPE string;").await?;
        self.db.query("DEFINE FIELD seller_id ON trade TYPE string;").await?;
        self.db.query("DEFINE FIELD quantity ON trade TYPE float ASSERT $value > 0;").await?;
        self.db.query("DEFINE FIELD price ON trade TYPE float ASSERT $value > 0;").await?;
        self.db.query("DEFINE FIELD ts ON trade TYPE datetime;").await?;
        self.db.query("DEFINE INDEX trade_by_tick ON TABLE trade FIELDS tick;").await?;
        self.db.query("DEFINE INDEX trade_by_market ON TABLE trade FIELDS market_id;").await?;

        self.db.query("DEFINE TABLE tick_summary SCHEMALESS;").await?;
        self.db.query("DEFINE FIELD tick ON tick_summary TYPE record<tick>;").await?;
        self.db.query("DEFINE FIELD totals ON tick_summary TYPE object;").await?;
        self.db.query("DEFINE INDEX tick_summary_by_tick ON TABLE tick_summary FIELDS tick UNIQUE;").await?;

        self.db.query("DEFINE TABLE impacts TYPE RELATION SCHEMALESS;").await?;
        self.db.query("DEFINE FIELD in ON impacts TYPE record<effect>;").await?;
        self.db.query("DEFINE FIELD out ON impacts TYPE record;").await?;
        self.db.query("DEFINE FIELD delta ON impacts TYPE object;").await?;

        self.db.query("DEFINE TABLE causes TYPE RELATION SCHEMALESS;").await?;
        self.db.query("DEFINE FIELD in ON causes TYPE record<action>;").await?;
        self.db.query("DEFINE FIELD out ON causes TYPE record<effect>;").await?;

        Ok(())
    }

    pub async fn write_tick_batch(
        &self,
        tick_number: u32,
        actions: &[ActionRecord],
        effects: &[StateEffect],
        action_to_effect_indices: &HashMap<usize, Vec<usize>>,
        trades: &[Trade],
        instruments: &[(InstrumentId, &FinancialInstrument)],
        agents: &[(AgentId, String, &BalanceSheet)],
    ) -> SurrealResult<()> {
        let now = surrealdb::sql::Datetime::from(chrono::Utc::now());
        
        let tick_results: Vec<TickRecordB> = self
            .db
            .query("CREATE tick SET tick_number = $tick_number, ts = $ts RETURN *")
            .bind(("tick_number", tick_number as i64))
            .bind(("ts", now.clone()))
            .await?
            .take(0)?;
        let tick_ref = tick_results.first().and_then(|t| t.id.clone()).unwrap();
        
        let mut created_actions = Vec::with_capacity(actions.len());
        if !actions.is_empty() {
            for action_rec in actions {
                let created: Vec<ActionRecordB> = self.db
                    .query("CREATE action SET tick = type::thing($tick), action_type = $kind, agent_id = $agent_id, agent_type = $agent_type, agent_name = $agent_name, details = $details, ts = $ts RETURN *")
                    .bind(("tick", tick_ref.to_string()))
                    .bind(("kind", action_rec.action.name()))
                    .bind(("agent_id", action_rec.agent_id.to_string()))
                    .bind(("agent_type", action_rec.agent_type.clone()))
                    .bind(("agent_name", action_rec.agent_name.clone()))
                    .bind(("details", serde_json::to_value(action_rec.action.clone()).unwrap_or(serde_json::Value::Null)))
                    .bind(("ts", now.clone()))
                    .await?.take(0)?;
                if let Some(action_with_id) = created.first() {
                    created_actions.push(action_with_id.clone());
                }
            }
        }

        let mut effect_records_with_ids = Vec::new();
        let mut created_effects = Vec::with_capacity(effects.len());
        if !effects.is_empty() {
            for effect in effects {
                let created: Vec<EffectRecord> = self.db
                    .query("CREATE effect SET tick = type::thing($tick), kind = $kind, payload = $payload, ts = $ts RETURN *")
                    .bind(("tick", tick_ref.to_string()))
                    .bind(("kind", effect.name()))
                    .bind(("payload", serde_json::to_value(effect.clone()).unwrap_or(serde_json::Value::Null)))
                    .bind(("ts", now.clone()))
                    .await?.take(0)?;
                if let Some(effect_with_id) = created.first() {
                    effect_records_with_ids.push((effect.clone(), effect_with_id.clone()));
                    created_effects.push(effect_with_id.clone());
                }
            }
        }

        for (action_idx, effect_indices) in action_to_effect_indices {
            if let Some(action_db_rec) = created_actions.get(*action_idx) {
                if let Some(action_id) = action_db_rec.id.clone() {
                    for effect_idx in effect_indices {
                        if let Some(effect_db_rec) = created_effects.get(*effect_idx) {
                            if let Some(effect_id) = effect_db_rec.id.clone() {
                                self.db
                                    .query("RELATE $action->causes->$effect")
                                    .bind(("action", action_id.clone()))
                                    .bind(("effect", effect_id.clone())).await?;

                            }
                        }
                    }
                }
            }
        }
        
        if !trades.is_empty() {
            for trade in trades {
                let _: Vec<TradeRecord> = self.db
                    .query("CREATE trade SET tick = type::thing($tick), market_id = $market_id, buyer_id = $buyer_id, seller_id = $seller_id, quantity = $quantity, price = $price, ts = $ts")
                    .bind(("tick", tick_ref.to_string()))
                    .bind(("market_id", trade.market_id.to_string()))
                    .bind(("buyer_id", trade.buyer.to_string()))
                    .bind(("seller_id", trade.seller.to_string()))
                    .bind(("quantity", trade.quantity))
                    .bind(("price", trade.price))
                    .bind(("ts", now.clone()))
                    .await?.take(0)?;
            }
        }
        
        if !instruments.is_empty() {
            for (id, instrument) in instruments {
                let _: Vec<InstrumentStateRecord> = self.db
                    .query("CREATE instrument_state SET instrument = $instrument, tick = type::thing($tick), principal = $principal, accr_int = $accr_int, debtor = $debtor, creditor = $creditor, details = $details")
                    .bind(("instrument", id.to_string()))
                    .bind(("tick", tick_ref.to_string()))
                    .bind(("principal", instrument.principal))
                    .bind(("accr_int", instrument.accrued_interest))
                    .bind(("debtor", instrument.debtor.to_string()))
                    .bind(("creditor", instrument.creditor.to_string()))
                    .bind(("details", serde_json::to_value(instrument.details.as_ref()).unwrap_or(serde_json::Value::Null)))
                    .await?.take(0)?;
            }
        }

        if !agents.is_empty() {
            for (id, agent_type, bs) in agents {
                let details = serde_json::json!({ "cash_assets": bs.liquid_assets(), "deposit_assets": bs.total_deposits() });
                let _: Vec<AgentStateRecord> = self.db
                    .query("CREATE agent_state SET agent = $agent, tick = type::thing($tick), agent_type = $agent_type, total_assets = $total_assets, total_liabilities = $total_liabilities, net_worth = $net_worth, liquid_assets = $liquid_assets, details = $details")
                    .bind(("agent", id.to_string()))
                    .bind(("tick", tick_ref.to_string()))
                    .bind(("agent_type", agent_type.to_string()))
                    .bind(("total_assets", bs.total_assets()))
                    .bind(("total_liabilities", bs.total_liabilities()))
                    .bind(("net_worth", bs.net_worth()))
                    .bind(("liquid_assets", bs.liquid_assets()))
                    .bind(("details", details))
                    .await?.take(0)?;
            }
        }

        let prev_tick_ref = if tick_number > 0 {
            let prev_tick_query: Vec<TickRecordB> = self
                .db
                .query("SELECT * FROM tick WHERE tick_number = $prev")
                .bind(("prev", (tick_number - 1) as i64))
                .await?.take(0)?;
            prev_tick_query.first().and_then(|t| t.id.clone())
        } else {
            None
        };

        if let Some(prev_tick) = prev_tick_ref {
            if let Err(e) = self.create_attribution_edges(&effect_records_with_ids, &tick_ref, &prev_tick).await {
                println!("[WARNING] Failed to create attribution edges: {}", e);
            }
        }

        let summary_totals = serde_json::json!({
            "action_count": actions.len(),
            "effect_count": effects.len(),
            "trade_count": trades.len(),
            "instrument_count": instruments.len(),
            "agent_count": agents.len(),
            "total_trade_volume": trades.iter().map(|t| t.quantity).sum::<f64>(),
            "total_trade_value": trades.iter().map(|t| t.quantity * t.price).sum::<f64>(),
        });
        let _: Vec<TickSummaryRecord> = self
            .db
            .query("CREATE tick_summary SET tick = type::thing($tick), totals = $totals")
            .bind(("tick", tick_ref.to_string()))
            .bind(("totals", summary_totals))
            .await?.take(0)?;
        Ok(())
    }

    pub async fn create_attribution_edges(
        &self, effect_records: &[(StateEffect, EffectRecord)], current_tick_ref: &surrealdb::sql::Thing,
        previous_tick_ref: &surrealdb::sql::Thing,
    ) -> SurrealResult<()> {
        for (effect, effect_record) in effect_records {
            let effect_id = match &effect_record.id {
                Some(id) => id,
                None => continue,
            };
            match effect {
                StateEffect::Financial(financial_effect) => {
                    if let Err(e) = self
                        .handle_financial_attribution(financial_effect, effect_id, current_tick_ref, previous_tick_ref)
                        .await
                    {
                        println!(
                            "[WARNING] Failed to create financial attribution for effect {:?}: {}",
                            financial_effect.name(),
                            e
                        );
                    }
                }
                StateEffect::Market(MarketEffect::ExecuteTrade(trade)) => {
                    if let Err(e) =
                        self.handle_trade_attribution(trade, effect_id, current_tick_ref, previous_tick_ref).await
                    {
                        println!("[WARNING] Failed to create trade attribution: {}", e);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn handle_financial_attribution(
        &self, effect: &FinancialEffect, effect_id: &surrealdb::sql::Thing, current_tick_ref: &surrealdb::sql::Thing,
        previous_tick_ref: &surrealdb::sql::Thing,
    ) -> SurrealResult<()> {
        let maybe_inst_id = match effect {
            FinancialEffect::CreateInstrument(instrument) => Some(instrument.id.to_string()),
            FinancialEffect::UpdateInstrument { id, .. } => Some(id.to_string()),
            FinancialEffect::RemoveInstrument(id) => Some(id.to_string()),
            FinancialEffect::TransferInstrument { id, .. } => Some(id.to_string()),
            FinancialEffect::SplitAndTransferInstrument { id, .. } => Some(id.to_string()),
            _ => None,
        };
        if let Some(inst_id) = maybe_inst_id {
            self.create_instrument_attribution(&inst_id, effect_id, current_tick_ref, previous_tick_ref).await?;
            let _ = self.create_parties_attribution(&inst_id, effect_id, current_tick_ref, previous_tick_ref).await;
        }
        Ok(())
    }

    async fn create_instrument_attribution(
        &self, inst_id: &str, effect_id: &surrealdb::sql::Thing, current_tick_ref: &surrealdb::sql::Thing,
        previous_tick_ref: &surrealdb::sql::Thing,
    ) -> SurrealResult<()> {
        let inst_id = inst_id.to_string();
        let current_states: Vec<InstrumentStateRecord> = self
            .db
            .query("SELECT * FROM instrument_state WHERE instrument = $inst AND tick = type::thing($tick)")
            .bind(("inst", inst_id.clone()))
            .bind(("tick", current_tick_ref.to_string()))
            .await?
            .take(0)?;
        let previous_states: Vec<InstrumentStateRecord> = self
            .db
            .query("SELECT * FROM instrument_state WHERE instrument = $inst AND tick = type::thing($tick)")
            .bind(("inst", inst_id.clone()))
            .bind(("tick", previous_tick_ref.to_string()))
            .await?
            .take(0)?;

        let (target_state, using_previous) = match (current_states.first(), previous_states.first()) {
            (Some(cur), _) => (cur.clone(), false),
            (None, Some(prev)) => (prev.clone(), true),
            (None, None) => return Ok(()), // nothing to relate
        };

        let prev = previous_states.first();
        let principal_delta = target_state.principal - prev.map_or(0.0, |p| p.principal);
        let accr_int_delta = target_state.accr_int - prev.map_or(0.0, |p| p.accr_int);

        let debtor_changed = prev.map_or(false, |p| p.debtor != target_state.debtor);
        let creditor_changed = prev.map_or(false, |p| p.creditor != target_state.creditor);

        let ownership = serde_json::json!({
            "from": prev.map(|p| serde_json::json!({ "debtor": p.debtor, "creditor": p.creditor })),
            "to": if using_previous { serde_json::Value::Null } else { serde_json::json!({
                "debtor": target_state.debtor, "creditor": target_state.creditor
            })},
        });

        let (rel_state_id, principal, accr_int) = if using_previous {
            (target_state.id.clone(), -prev.map_or(0.0, |p| p.principal), -prev.map_or(0.0, |p| p.accr_int))
        } else {
            (target_state.id.clone(), principal_delta, accr_int_delta)
        };

        if let Some(state_id) = rel_state_id {
            let delta = serde_json::json!({
                "principal": principal,
                "accr_int": accr_int,
                "debtor_changed": debtor_changed,
                "creditor_changed": creditor_changed,
                "ownership": ownership,
            });
            let _: Vec<ImpactRecord> = self.db
                .query("RELATE $effect->impacts->$state SET delta = $delta")
                .bind(("effect", effect_id.clone()))
                .bind(("state", state_id))
                .bind(("delta", delta))
                .await?
                .take(0)?;
        }
        Ok(())
    }

    async fn create_agent_attribution(
        &self, agent_id: &str, effect_id: &surrealdb::sql::Thing, current_tick_ref: &surrealdb::sql::Thing,
        previous_tick_ref: &surrealdb::sql::Thing,
    ) -> SurrealResult<()> {
        let agent_id = agent_id.to_string();
        let current_states: Vec<AgentStateRecord> = self
            .db
            .query("SELECT * FROM agent_state WHERE agent = $agent AND tick = type::thing($tick)")
            .bind(("agent", agent_id.clone()))
            .bind(("tick", current_tick_ref.to_string()))
            .await?
            .take(0)?;
        let previous_states: Vec<AgentStateRecord> = self
            .db
            .query("SELECT * FROM agent_state WHERE agent = $agent AND tick = type::thing($tick)")
            .bind(("agent", agent_id.clone()))
            .bind(("tick", previous_tick_ref.to_string()))
            .await?
            .take(0)?;
        if let Some(current_state) = current_states.first() {
            let previous_state = previous_states.first();
            let delta = serde_json::json!({
                "total_assets": current_state.total_assets - previous_state.map_or(0.0, |p| p.total_assets),
                "total_liabilities": current_state.total_liabilities - previous_state.map_or(0.0, |p| p.total_liabilities),
                "net_worth": current_state.net_worth - previous_state.map_or(0.0, |p| p.net_worth),
                "liquid_assets": current_state.liquid_assets - previous_state.map_or(0.0, |p| p.liquid_assets),
            });
            let effect_id = effect_id.clone();
            if let Some(current_id) = current_state.clone().id {
                let _: Vec<ImpactRecord> = self
                    .db
                    .query("RELATE $effect->impacts->$state SET delta = $delta")
                    .bind(("effect", effect_id))
                    .bind(("state", current_id))
                    .bind(("delta", delta))
                    .await?
                    .take(0)?;
            }
        }
        Ok(())
    }

    async fn create_parties_attribution(
        &self,
        inst_id: &str,
        effect_id: &surrealdb::sql::Thing,
        current_tick_ref: &surrealdb::sql::Thing,
        previous_tick_ref: &surrealdb::sql::Thing,
    ) -> SurrealResult<()> {
        let cur: Vec<InstrumentStateRecord> = self.db
            .query("SELECT * FROM instrument_state WHERE instrument = $inst AND tick = type::thing($tick)")
            .bind(("inst", inst_id.to_string()))
            .bind(("tick", current_tick_ref.to_string()))
            .await?
            .take(0)?;
        let prev: Vec<InstrumentStateRecord> = self.db
            .query("SELECT * FROM instrument_state WHERE instrument = $inst AND tick = type::thing($tick)")
            .bind(("inst", inst_id.to_string()))
            .bind(("tick", previous_tick_ref.to_string()))
            .await?
            .take(0)?;

        let parties = cur.first()
            .map(|s| vec![s.creditor.clone(), s.debtor.clone()])
            .or_else(|| prev.first().map(|s| vec![s.creditor.clone(), s.debtor.clone()]))
            .unwrap_or_default();

        for party in parties {
            if let Err(e) = self.create_agent_attribution(&party, effect_id, current_tick_ref, previous_tick_ref).await {
                println!("[WARNING] Failed to create agent attribution for party {}: {}", party, e);
            }
        }
        Ok(())
    }

    async fn handle_trade_attribution(
        &self, trade: &Trade, effect_id: &surrealdb::sql::Thing, current_tick_ref: &surrealdb::sql::Thing,
        previous_tick_ref: &surrealdb::sql::Thing,
    ) -> SurrealResult<()> {
        if let Err(e) = self
            .create_agent_attribution(&trade.buyer.to_string(), effect_id, current_tick_ref, previous_tick_ref)
            .await
        {
            println!("[WARNING] Failed to create buyer attribution: {}", e);
        }
        if let Err(e) = self
            .create_agent_attribution(&trade.seller.to_string(), effect_id, current_tick_ref, previous_tick_ref)
            .await
        {
            println!("[WARNING] Failed to create seller attribution: {}", e);
        }
        Ok(())
    }

    pub async fn cleanup_old_data(&self, keep_ticks: u32) -> SurrealResult<()> {
        let _: Result<surrealdb::Response, surrealdb::Error> = self
            .db
            .query(
                "
                LET $cutoff_tick = (
                    SELECT VALUE tick_number 
                    FROM tick 
                    ORDER BY tick_number DESC 
                    LIMIT 1
                )[0] - $keep_ticks;
                DELETE tick WHERE tick_number < $cutoff_tick;
                DELETE action WHERE tick IN (SELECT id FROM tick WHERE tick_number < $cutoff_tick);
                DELETE effect WHERE tick IN (SELECT id FROM tick WHERE tick_number < $cutoff_tick);
                DELETE instrument_state WHERE tick IN (SELECT id FROM tick WHERE tick_number < $cutoff_tick);
                DELETE agent_state WHERE tick IN (SELECT id FROM tick WHERE tick_number < $cutoff_tick);
                DELETE trade WHERE tick IN (SELECT id FROM tick WHERE tick_number < $cutoff_tick);
                DELETE tick_summary WHERE tick IN (SELECT id FROM tick WHERE tick_number < $cutoff_tick);
                DELETE impacts WHERE in.tick IN (SELECT id FROM tick WHERE tick_number < $cutoff_tick);
                DELETE causes WHERE in.tick IN (SELECT id FROM tick WHERE tick_number < $cutoff_tick);
            ",
            )
            .bind(("keep_ticks", keep_ticks as i64))
            .await;
        Ok(())
    }

    pub async fn get_instruments_at_tick(&self, tick_number: u32) -> SurrealResult<Vec<InstrumentStateRecord>> {
        let instruments: Vec<InstrumentStateRecord> = self
            .db
            .query(
                "SELECT * FROM instrument_state WHERE tick = (SELECT id FROM tick WHERE tick_number = $tick_number)[0]",
            )
            .bind(("tick_number", tick_number as i64))
            .await?
            .take(0)?;
        Ok(instruments)
    }

    pub async fn get_instrument_history(
        &self, instrument_id: &str, tick_from: u32, tick_to: u32,
    ) -> SurrealResult<Vec<InstrumentStateRecord>> {
        let instrument_id = instrument_id.to_string();
        let history: Vec<InstrumentStateRecord> = self
            .db
            .query(
                "
                SELECT * FROM instrument_state 
                WHERE instrument = $instrument_id 
                AND tick IN (SELECT id FROM tick WHERE tick_number BETWEEN $tick_from AND $tick_to)
                ORDER BY tick.tick_number
            ",
            )
            .bind(("instrument_id", instrument_id))
            .bind(("tick_from", tick_from as i64))
            .bind(("tick_to", tick_to as i64))
            .await?
            .take(0)?;
        Ok(history)
    }

    pub async fn get_effects_for_tick(&self, tick_number: u32) -> SurrealResult<Vec<EffectRecord>> {
        let effects: Vec<EffectRecord> = self.db
            .query("SELECT * FROM effect WHERE tick = (SELECT id FROM tick WHERE tick_number = $tick_number)[0] ORDER BY ts")
            .bind(("tick_number", tick_number as i64))
            .await?
            .take(0)?;
        Ok(effects)
    }

    pub async fn get_trades_for_market_range(
        &self, market_id: &str, tick_from: u32, tick_to: u32,
    ) -> SurrealResult<Vec<TradeRecord>> {
        let market_id = market_id.to_string();
        let trades: Vec<TradeRecord> = self
            .db
            .query(
                "
                SELECT * FROM trade 
                WHERE market_id = $market_id 
                AND tick IN (SELECT id FROM tick WHERE tick_number BETWEEN $tick_from AND $tick_to)
                ORDER BY tick.tick_number, ts
            ",
            )
            .bind(("market_id", market_id))
            .bind(("tick_from", tick_from as i64))
            .bind(("tick_to", tick_to as i64))
            .await?
            .take(0)?;
        Ok(trades)
    }

    pub async fn get_effect_impacts(&self, effect_id: &surrealdb::sql::Thing) -> SurrealResult<Vec<serde_json::Value>> {
        let impacts: Vec<serde_json::Value> = self
            .db
            .query(
                "
                SELECT ->impacts->{
                    out: *,
                    delta
                } as impacts 
                FROM type::thing($effect_id)
            ",
            )
            .bind(("effect_id", effect_id.to_string()))
            .await?
            .take(0)?;
        Ok(impacts)
    }

    pub async fn get_agent_deltas(&self, tick_number: u32) -> SurrealResult<Vec<serde_json::Value>> {
        let deltas: Vec<serde_json::Value> = self
            .db
            .query(
                "
                LET $t = (SELECT VALUE id FROM tick WHERE tick_number = $tick)[0];
                LET $p = (SELECT VALUE id FROM tick WHERE tick_number = $tick - 1)[0];
                SELECT 
                    agent, 
                    agent_type,
                    (a.total_assets - prev.total_assets) AS d_assets,
                    (a.total_liabilities - prev.total_liabilities) AS d_liabs,
                    (a.net_worth - prev.net_worth) AS d_equity,
                    a.details
                FROM agent_state AS a
                LET prev = (SELECT * FROM agent_state WHERE agent = a.agent AND tick = $p)[0]
                WHERE a.tick = $t AND prev != NONE
                ORDER BY d_equity DESC;
            ",
            )
            .bind(("tick", tick_number as i64))
            .await?
            .take(0)?;
        Ok(deltas)
    }

    pub async fn get_instrument_deltas(&self, tick_number: u32) -> SurrealResult<Vec<serde_json::Value>> {
        let deltas: Vec<serde_json::Value> = self
            .db
            .query(
                "
                LET $t = (SELECT VALUE id FROM tick WHERE tick_number = $tick)[0];
                LET $p = (SELECT VALUE id FROM tick WHERE tick_number = $tick - 1)[0];
                SELECT 
                    i.instrument, 
                    i.creditor, 
                    i.debtor, 
                    i.details,
                    (i.principal - prev.principal) AS d_principal,
                    (i.accr_int - prev.accr_int) AS d_accr_int
                FROM instrument_state AS i
                LET prev = (SELECT * FROM instrument_state WHERE instrument = i.instrument AND tick = $p)[0]
                WHERE i.tick = $t AND prev != NONE
                ORDER BY abs(d_principal) DESC, abs(d_accr_int) DESC;
            ",
            )
            .bind(("tick", tick_number as i64))
            .await?
            .take(0)?;
        Ok(deltas)
    }
}