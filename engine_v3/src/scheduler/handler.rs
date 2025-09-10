use super::{StepContext, StepHandler, StepResult};
use crate::executor::SimulationEngine;
use domains::{ResolutionContext, ResolutionPhase};
use rand::prelude::*;
use rust_decimal_macros::dec;
use sim_core::*;
use std::time::Instant;
use tracing::instrument;

fn execute_step<F>(step_fn: F) -> StepResult
where
    F: FnOnce() -> Result<serde_json::Value, String>,
{
    let start = Instant::now();
    match step_fn() {
        Ok(metadata) => StepResult::success(start.elapsed().as_millis() as u64, metadata),
        Err(e) => StepResult::failure(start.elapsed().as_millis() as u64, e),
    }
}
#[derive(Debug)]
pub struct UpkeepHandler;

impl StepHandler for UpkeepHandler {
    fn execute(&self, engine: &mut SimulationEngine, _ctx: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            if engine.state.current_session == Session::AM {
                engine.state.advance_time();
            }
            if let Err(e) = engine.state.financial_system.validate_accounting_identity() {
                return Err(format!("Accounting validation failed on tick {}: {}", engine.state.ticknum, e));
            }
            Ok(serde_json::json!({
                "current_date": engine.state.current_date,
                "tick_number": engine.state.ticknum,
                "session": format!("{:?}", engine.state.current_session),
            }))
        })
    }
}
#[derive(Debug)]
pub struct GatherIntentionsHandler;
impl StepHandler for GatherIntentionsHandler {
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let intentions = engine.gather_intentions(rng);
            let categorized = engine.domain_registry.categorize_intentions_by_phase(intentions.clone());

            context.store("intentions", &intentions)?;
            context.store("categorized_intentions", &categorized)?;
            let mut concatted = String::new();
            for intention in &intentions {
                concatted.push_str(&format!("{}; ", intention.name()));
            }
            Ok(serde_json::json!({ "total_intentions": intentions.len() }))
        })
    }
}
#[derive(Debug)]
pub struct ApplyInstrumentCreationHandler;

impl StepHandler for ApplyInstrumentCreationHandler {
    #[instrument(skip(self, engine, context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let all_effects = context.get_all_effects().unwrap_or_default();

            let instrument_effects: Vec<StateEffect> = all_effects
                .into_iter()
                .filter(|e| matches!(e, StateEffect::Financial(FinancialEffect::CreateInstrument { .. })))
                .collect();

            if instrument_effects.is_empty() {
                return Ok(serde_json::json!({ "instruments_created": 0 }));
            }

            let count = instrument_effects.len();

            engine.state.apply_effects(&instrument_effects).map_err(|e| e.to_string())?;

            Ok(serde_json::json!({ "instruments_created": count }))
        })
    }
}

#[derive(Debug)]
pub struct PhaseResolutionHandler {
    pub phase: ResolutionPhase,
}
impl StepHandler for PhaseResolutionHandler {
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let categorized = context.get_categorized_intentions()?;
            let intentions = categorized.get(&self.phase).cloned().unwrap_or_default();
            if intentions.is_empty() {
                return Ok(serde_json::json!({ "actions": 0, "effects": 0 }));
            }

            let resolution_context = ResolutionContext { state: &engine.state, current_tick: engine.state.ticknum };
            let action_offset = context.get_all_actions().unwrap_or_default().len();
            let effect_offset = context.get_all_effects().unwrap_or_default().len();

            let (action_records, action_to_effect_indices, effects) =
                engine.resolve_and_execute_phase(&intentions, &resolution_context, action_offset, effect_offset);

            let mut all_actions = context.get_all_actions().unwrap_or_default();
            all_actions.extend(action_records.clone());
            context.store("all_actions", &all_actions)?;

            let mut all_effects = context.get_all_effects().unwrap_or_default();
            all_effects.extend(effects.clone());
            context.store("all_effects", &all_effects)?;

            let mut mapping = context.get_action_to_effect_indices().unwrap_or_default();
            mapping.extend(action_to_effect_indices);
            context.store("action_to_effect_indices", &mapping)?;

            Ok(serde_json::json!({"actions": action_records.len(), "effects": effects.len()}))
        })
    }
}
#[derive(Debug)]
pub struct ApplyMarketEffectsHandler;
impl StepHandler for ApplyMarketEffectsHandler {
    #[instrument(skip(self, engine, context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let all_effects = context.get_all_effects().unwrap_or_default();
            let market_effects: Vec<StateEffect> =
                all_effects.into_iter().filter(|e| matches!(e, StateEffect::Market(_))).collect();

            engine.state.apply_effects(&market_effects).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "market_effects_applied": market_effects.len() }))
        })
    }
}
#[derive(Debug)]
pub struct ClearMarketsHandler;
impl StepHandler for ClearMarketsHandler {
    #[instrument(skip(self, engine, context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let (market_trades, snapshots) = engine.clear_all_markets();

            let snapshots_s: std::collections::HashMap<String, MarketView> =
                snapshots.into_iter().map(|(k, v)| (k.to_string(), v)).collect();

            let mut all_trades = context.get_trades().unwrap_or_default();

            let trades_generated = market_trades.len();
            all_trades.extend(market_trades);

            context.store("trades", &all_trades)?;

            context.store("market_snapshots", &snapshots_s)?;
            Ok(serde_json::json!({ "trades_generated": trades_generated }))
        })
    }
}
#[derive(Debug)]
pub struct SettleTradesHandler;

impl StepHandler for SettleTradesHandler {
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let trades: Vec<Trade> = context.get("trades").unwrap_or_default();

            if trades.is_empty() {
                return Ok(serde_json::json!({
                    "trades_processed": 0,
                    "settlement_effects": 0
                }));
            }

            let settlement_effects = engine.settle_trades(&trades);

            let mut all_effects = context.get_all_effects().unwrap_or_default();
            let effect_count = settlement_effects.len();
            all_effects.extend(settlement_effects); // Still add to context for logging/history
            context.store("all_effects", &all_effects)?;

            Ok(serde_json::json!({
                "trades_processed": trades.len(),
                "settlement_effects": effect_count
            }))
        })
    }
}
#[derive(Debug)]
pub struct ApplyPaymentQueuingHandler;
impl StepHandler for ApplyPaymentQueuingHandler {
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let count = engine.consume_effects(context, |effect| {
                matches!(
                    effect,
                    StateEffect::Financial(
                        FinancialEffect::QueuePayment(_) | FinancialEffect::RecordSettlementInstruction(_)
                    )| StateEffect::Inventory(_)    
                )
            })?;
            Ok(serde_json::json!({ "payments_and_settlements_queued": count }))
        })
    }
}

#[derive(Debug)]
pub struct RunRTGSHandler;

impl StepHandler for RunRTGSHandler {
    #[instrument(skip(self, engine, context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let initial_pending = engine.state.financial_system.rtgs.pending.len();

            let finalization_effects =
                run_rtgs(&mut engine.state).map_err(|e| format!("RTGS execution failed: {:?}", e))?;

            let mut all_effects: Vec<StateEffect> = context.get("all_effects").unwrap_or_default();
            all_effects.extend(finalization_effects);
            context.store("all_effects", &all_effects)?;

            let final_pending = engine.state.financial_system.rtgs.pending.len();
            let settled_this_tick = initial_pending - final_pending;

            Ok(serde_json::json!({
                "payments_settled": settled_this_tick,
                "payments_remaining": final_pending
            }))
        })
    }
}
#[derive(Debug)]
pub struct ApplyAllEffectsHandler;
impl StepHandler for ApplyAllEffectsHandler {
    #[instrument(skip(self, engine, context, rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let mut all_effects = context.get_all_effects().unwrap_or_default();
            let labour_effects = engine.match_labour_markets(rng);
            all_effects.extend(labour_effects);

            let all_actions = context.get_all_actions().unwrap_or_default();
            let all_intentions = context.get_intentions().unwrap_or_default();
            let mapping = context.get_action_to_effect_indices().unwrap_or_default();

            let mut new_events: Vec<SimEvent> = Vec::new();

            new_events.extend(all_intentions.into_iter().map(SimEvent::Intention));
            new_events.extend(all_actions.clone().into_iter().map(SimEvent::Action));
            new_events.extend(all_effects.clone().into_iter().map(SimEvent::Effect));

            for (action_idx, action_record) in all_actions.iter().enumerate() {
                if let Some(effect_indices) = mapping.get(&action_idx) {
                    let financial_effects: Vec<FinancialEffect> = effect_indices
                        .iter()
                        .filter_map(|&effect_idx| {
                            all_effects.get(effect_idx).and_then(|eff| match eff {
                                StateEffect::Financial(fe) => Some(fe.clone()),
                                _ => None,
                            })
                        })
                        .collect();

                    if !financial_effects.is_empty() {
                        let action_context = ActionContext {
                            action_instance_id: action_record.id,
                            action_name: action_record.action.name(),
                            agent_id: action_record.agent_id,
                            tick: engine.state.ticknum,
                        };
                        new_events.push(SimEvent::FinancialTransaction {
                            context: action_context,
                            effects: financial_effects,
                        });
                    }
                }
            }

            engine.event_log = new_events;

            engine.state.apply_effects(&all_effects).map_err(|e| e.to_string())?;

            context.store("all_effects", &all_effects)?;
            Ok(serde_json::json!({"total_effects_applied": all_effects.len()}))
        })
    }
}
#[derive(Debug)]
pub struct UpdateHistoryHandler;
impl StepHandler for UpdateHistoryHandler {
    #[instrument(skip(self, engine, context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let trades = context.get_trades().unwrap_or_default();
            let snapshots = context.get_market_snapshots().unwrap_or_default();
            engine.update_market_history(&trades, &snapshots);
            engine.state.financial_system.update_yield_curve(engine.state.current_date);
            let tick_record = TickRecord {
                tick_number: engine.state.ticknum,
                date: engine.state.current_date,
                intentions: context.get_intentions().unwrap_or_default(),
                actions: context.get_all_actions().unwrap_or_default(),
                effects: context.get_all_effects().unwrap_or_default(),
                action_to_effect_indices: context.get_action_to_effect_indices().unwrap_or_default(),
                trades,
                events: engine.event_log.clone(),
            };

            engine.state.history.add_tick_record(tick_record);
            Ok(serde_json::Value::Null)
        })
    }
}
#[derive(Debug)]
pub struct DebtAuctionsHandler;
impl StepHandler for DebtAuctionsHandler {
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let open_ids: Vec<_> = engine
                .state
                .financial_system
                .exchange
                .open_auctions
                .iter()
                .filter(|(_, a)| a.status == AuctionStatus::Open)
                .map(|(id, _)| *id)
                .collect();

            let mut auction_trades: Vec<Trade> = Vec::new();

            for auction_id in open_ids {
                let (inst_id, quantity_offered) = {
                    let a = &engine.state.financial_system.exchange.open_auctions[&auction_id];
                    (a.instrument_id, a.quantity_offered as f64)
                };

                let mut trades = engine
                    .state
                    .financial_system
                    .exchange
                    .conduct_dutch_auction(&auction_id, &engine.state.financial_system.instruments);

                let sold_qty: f64 =
                    trades.iter().filter(|t| t.market_id == MarketId::Financial(inst_id)).map(|t| t.quantity).sum();

                let leftover = (quantity_offered - sold_qty).max(0.0);

                if leftover > 0.0 {
                    let fs = &engine.state.financial_system;
                    let cb_id = fs.central_bank.id;
                    let gov_id = fs.government.id;

                    let fallback_price = if let Some(inst) = fs.instruments.get(&inst_id) {
                        if let InstrumentType::Debt(DebtInstrument::Bond(d)) = &inst.instrument_type {
                            let ytm = pricing::years_to_maturity(engine.state.current_date, d.maturity_date);
                            let y_backstop = (fs.central_bank.policy_rate_bps - dec!(50)).max(dec!(0));
                            pricing::bond_price(
                                d.face_value,
                                bps_to_decimal(d.coupon_rate_bps),
                                bps_to_decimal(y_backstop),
                                ytm,
                                d.frequency as usize,
                            )
                        } else {
                            Money::from(1000u64)
                        }
                    } else {
                        Money::from(1000u64)
                    };

                    let backstop_price = trades
                        .iter()
                        .rev()
                        .find(|t| t.market_id == MarketId::Financial(inst_id))
                        .map(|t| t.price)
                        .unwrap_or(fallback_price);

                    trades.push(Trade {
                        trade_id: uuid::Uuid::new_v4(),
                        market_id: MarketId::Financial(inst_id),
                        buyer: cb_id,
                        seller: gov_id,
                        price: backstop_price,
                        quantity: leftover,
                    });
                }

                auction_trades.extend(trades);
            }

            if !auction_trades.is_empty() {
                let mut all_trades = context.get_trades().unwrap_or_default();
                all_trades.extend(auction_trades);
                context.store("trades", &all_trades)?;
            }

            Ok(serde_json::json!({ "auctions_processed": true }))
        })
    }
}
