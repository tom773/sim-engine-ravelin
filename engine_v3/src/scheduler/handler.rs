use super::{StepContext, StepHandler, StepResult};
use crate::executor::SimulationEngine;
use domains::{ResolutionContext, ResolutionPhase};
use rand::prelude::*;
use sim_core::*;
use std::time::Instant;
use tracing::instrument;
use uuid::Uuid;

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
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            engine.state.advance_time();
            let upkeep_actions = engine.process_financial_updates();
            let (_, _, upkeep_effects) = engine.execute_actions(&upkeep_actions);

            context.store("all_effects", &upkeep_effects)?;
            Ok(serde_json::json!({
                "date": engine.state.current_date.format("%Y-%m-%d").to_string(),
                "upkeep_actions": upkeep_actions.len(),
                "upkeep_effects": upkeep_effects.len()
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

            let _effects_str = market_effects.iter().map(|e| e.name()).collect::<Vec<_>>().join(", ");
            /*tracing::event!(
                tracing::Level::INFO,
                "Applying {} market effects: [{}]",
                market_effects.len(),
                effects_str
            );*/

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

            tracing::event!(
                tracing::Level::INFO,
                "Markets cleared. Generated {} trades and {} snapshots.",
                market_trades.len(),
                snapshots_s.len()
            );

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
pub struct BuildSettlementObligationsHandler;

impl StepHandler for BuildSettlementObligationsHandler {
    #[instrument(skip(self, engine, context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let trades = context.get_trades().unwrap_or_default();
            let mut settlement_effects = Vec::new();

            for trade in trades {
                let total_payment = (trade.price * trade.quantity).to_f64();

                if total_payment > 1e-9 {
                    let buyer_settlement_agent = engine
                        .state
                        .financial_system
                        .find_agent_liquid_account(&trade.buyer)
                        .map(|(_, agent)| agent)
                        .ok_or_else(|| format!("Could not find settlement agent for buyer {}", trade.buyer))?;

                    let seller_settlement_agent = engine
                        .state
                        .financial_system
                        .find_agent_liquid_account(&trade.seller)
                        .map(|(_, agent)| agent)
                        .ok_or_else(|| format!("Could not find settlement agent for seller {}", trade.seller))?;

                    let payment_instruction = PaymentInstruction {
                        id: Uuid::new_v4(),
                        from_bank: buyer_settlement_agent,
                        to_bank: seller_settlement_agent,
                        payer: trade.buyer,
                        payee: trade.seller,
                        amount: total_payment,
                        context: TransactionContext::TradeSettlement { trade_id: trade.trade_id },
                        priority: PaymentPriority::Normal,
                        earliest_release_tick: engine.state.ticknum,
                        deadline_tick: engine.state.ticknum + 10,
                    };

                    settlement_effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(payment_instruction)));
                }

                if let MarketId::Financial(instrument_id) = &trade.market_id {
                    settlement_effects.push(StateEffect::Financial(FinancialEffect::ReserveSecurityForDvP {
                        trade_id: trade.trade_id,
                        instrument_id: *instrument_id,
                        quantity: trade.quantity,
                    }));
                }
            }

            /*let effects_str = settlement_effects.iter().map(|e| e.name()).collect::<Vec<_>>().join(", ");
            tracing::event!(
                tracing::Level::INFO,
                "Created {} settlement obligations: [{}]",
                settlement_effects.len(),
                effects_str
            );*/

            let mut all_effects = context.get_all_effects().unwrap_or_default();
            all_effects.extend(settlement_effects.clone());
            context.store("all_effects", &all_effects)?;

            Ok(serde_json::json!({
                "settlement_obligations_created": settlement_effects.len()
            }))
        })
    }
}

#[derive(Debug)]
pub struct RunRTGSHandler;

impl StepHandler for RunRTGSHandler {
    #[instrument(skip(self, engine, _context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, _context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let initial_pending = engine.state.financial_system.rtgs.pending.len();

            run_rtgs(&mut engine.state).map_err(|e| format!("RTGS execution failed: {:?}", e))?;

            let final_pending = engine.state.financial_system.rtgs.pending.len();
            let settled_count = initial_pending - final_pending;

            tracing::event!(
                tracing::Level::INFO,
                "RTGS run complete. Settled: {}, Remaining: {}.",
                settled_count,
                final_pending
            );

            Ok(serde_json::json!({
                "payments_settled": settled_count,
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
            let mapping = context.get_action_to_effect_indices().unwrap_or_default();

            let mut new_events = Vec::new();

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

            for effect in &all_effects {
                if let Some(event) = event_from_effect(effect) {
                    new_events.push(event);
                }
            }

            engine.event_log = new_events;

            //let effects_str = all_effects.iter().map(|e| e.name()).collect::<Vec<_>>().join(", ");
            //tracing::event!(tracing::Level::INFO, "Applying {} total effects: [{}]", all_effects.len(), effects_str);

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

            /*tracing::event!(
                tracing::Level::INFO,
                "Updating history for tick {}. Intentions: {}, Actions: {}, Effects: {}, Trades: {}",
                tick_record.tick_number,
                tick_record.intentions.len(),
                tick_record.actions.len(),
                tick_record.effects.len(),
                tick_record.trades.len()
            );*/

            engine.state.history.add_tick_record(tick_record);
            Ok(serde_json::Value::Null)
        })
    }
}

#[derive(Debug)]
pub struct DebtAuctionsHandler;
impl StepHandler for DebtAuctionsHandler {
    #[instrument(skip(self, engine, context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let mut auction_trades = Vec::new();

            let open_auction_ids: Vec<Uuid> = engine
                .state
                .financial_system
                .exchange
                .open_auctions
                .iter()
                .filter(|(_, auction)| auction.status == AuctionStatus::Open)
                .map(|(id, _)| *id)
                .collect();

            for auction_id in &open_auction_ids {
                let trades = engine
                    .state
                    .financial_system
                    .exchange
                    .conduct_dutch_auction(auction_id, &engine.state.financial_system.instruments);
                auction_trades.extend(trades);
            }

            tracing::event!(
                tracing::Level::INFO,
                "Conducted {} debt auctions, generating {} trades.",
                open_auction_ids.len(),
                auction_trades.len()
            );

            if !auction_trades.is_empty() {
                let mut all_trades = context.get_trades().unwrap_or_default();
                all_trades.extend(auction_trades.clone());
                context.store("trades", &all_trades)?;
            }

            Ok(
                serde_json::json!({ "debt_auctions_conducted": open_auction_ids.len(), "auction_trades_generated": auction_trades.len() }),
            )
        })
    }
}

fn event_from_effect(effect: &StateEffect) -> Option<SimEvent> {
    match effect {
        StateEffect::Financial(FinancialEffect::CreateInstrument { instrument, creditor, debtor, quantity }) => {
            Some(SimEvent::InstrumentLifecycle(sim_core::InstrumentLifecycleEvent::Created {
                instrument_id: instrument.id,
                creditor_id: *creditor,
                debtor_id: *debtor,
                quantity: *quantity,
                instrument_type: instrument.type_as_string().to_string(),
            }))
        }
        StateEffect::Financial(FinancialEffect::RemoveInstrument(instrument_id)) => {
            Some(SimEvent::InstrumentLifecycle(sim_core::InstrumentLifecycleEvent::Removed {
                instrument_id: *instrument_id,
            }))
        }
        StateEffect::Financial(FinancialEffect::RecordTransaction(tx)) => Some(SimEvent::TransactionRecord(tx.clone())),
        StateEffect::Financial(FinancialEffect::AdjustPosition {
            owner, instrument_id, delta_quantity, side, ..
        }) => Some(SimEvent::BalanceSheetUpdate(sim_core::BalanceSheetUpdateEvent {
            owner_id: *owner,
            instrument_id: *instrument_id,
            quantity_change: *delta_quantity,
            new_total_quantity: 0.0,
            side: side.clone(),
        })),
        _ => None,
    }
}
