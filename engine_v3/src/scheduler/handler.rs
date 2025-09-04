use super::{StepContext, StepHandler, StepResult};
use crate::executor::SimulationEngine;
use domains::{ResolutionContext, ResolutionPhase};
use rand::prelude::*;
use sim_core::*;
use std::time::Instant;
use tracing::instrument;
use uuid::Uuid;
use tracing::{event, Level};

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
pub struct ApplyInstrumentCreationHandler;

impl StepHandler for ApplyInstrumentCreationHandler {
    #[instrument(skip(self, engine, context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let all_effects = context.get_all_effects().unwrap_or_default();
            
            let instrument_effects: Vec<StateEffect> = all_effects
                .into_iter()
                .filter(|e| matches!(e, 
                    StateEffect::Financial(FinancialEffect::CreateInstrument { .. })
                ))
                .collect();

            if instrument_effects.is_empty() {
                return Ok(serde_json::json!({ "instruments_created": 0 }));
            }

            let count = instrument_effects.len();
            
            engine.state.apply_effects(&instrument_effects)
                .map_err(|e| e.to_string())?;
            
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
pub struct StartSettlementHandler;

impl StepHandler for StartSettlementHandler {
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let trades: Vec<Trade> = context.get("trades").unwrap_or_default();
            let mut payment_effects = Vec::new();
            let mut failed_reservations = 0;
            let mut successful_reservations = 0;
            let total_settlement_value = 0.0_f64;
            let fs = &engine.state.financial_system.clone();
            
            event!(Level::INFO,
                trade_count = trades.len(),
                "📋 Starting settlement process"
            );
            
            for trade in &trades {
                let instrument_id = match trade.market_id {
                    MarketId::Financial(id) => id,
                    _ => continue,
                };
                
                let instrument_name = engine.state.financial_system.clone().get_instrument_info(&instrument_id, &engine.state.agents, engine.state.current_date).unwrap().instrument_type;
                let seller_name = engine.state.get_agent_type_string(&trade.seller).unwrap_or_default();
                let buyer_name = engine.state.get_agent_type_string(&trade.buyer).unwrap_or_default();
                let trade_value = (trade.price * trade.quantity).to_f64();

                let instruction = SettlementInstruction {
                    instruction_id: Uuid::new_v4(),
                    trade_id: trade.trade_id,
                    seller: trade.seller,
                    buyer: trade.buyer,
                    instrument_id,
                    quantity: trade.quantity,
                    cash_amount: trade_value,
                    settlement_date: engine.state.current_date,
                    status: SettlementStatus::Pending,
                };

                event!(Level::DEBUG,
                    trade_id = %trade.trade_id.to_string()[..8],
                    instrument = %instrument_name,
                    seller = %seller_name,
                    buyer = %buyer_name,
                    quantity = trade.quantity,
                    price = ?trade.price,
                    value = trade_value,
                    "🔄 Attempting security reservation"
                );
                let ecl = engine.state.clone();
                match engine.state.financial_system.clearing_house.csd.reserve_securities_for_dvp(instruction.clone(), fs, &ecl) {
                    Ok(_) => {
                        successful_reservations += 1;
                        
                        event!(Level::INFO,
                            trade_id = %trade.trade_id.to_string()[..8],
                            instrument = %instrument_name,
                            instrument_id = %instrument_id,
                            seller = %seller_name,
                            buyer = %buyer_name,
                            quantity = trade.quantity,
                            value = trade_value,
                            "🔒 Securities reserved for DvP"
                        );
                        
                        let (_, buyer_settlement_agent) = engine.state.financial_system
                            .find_agent_liquid_account(&trade.buyer).unwrap();
                        let (_, seller_settlement_agent) = engine.state.financial_system
                            .find_agent_liquid_account(&trade.seller).unwrap();

                        let payment_instruction = PaymentInstruction {
                            id: Uuid::new_v4(),
                            from_bank: buyer_settlement_agent,
                            to_bank: seller_settlement_agent,
                            payer: trade.buyer,
                            payee: trade.seller,
                            amount: instruction.cash_amount,
                            context: TransactionContext::TradeSettlement { trade_id: trade.trade_id },
                            priority: PaymentPriority::Normal,
                            earliest_release_tick: engine.state.ticknum,
                            deadline_tick: engine.state.ticknum + 10,
                        };
                        
                        payment_effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(payment_instruction)));
                    }
                    Err(e) => {
                        failed_reservations += 1;
                        
                        event!(Level::ERROR,
                            trade_id = %trade.trade_id.to_string()[..8],
                            seller = %seller_name,
                            instrument = %instrument_name,
                            quantity = trade.quantity,
                            error = %e,
                            "🚨 CSD reservation failed"
                        );
                    }
                }
            }

            if !payment_effects.is_empty() {
                engine.state.apply_effects(&payment_effects)
                    .map_err(|e| format!("Failed to queue payments: {}", e))?;
            }

            event!(Level::INFO,
                successful_reservations,
                failed_reservations,
                payments_queued = payment_effects.len(),
                total_value = total_settlement_value,
                "📊 Settlement initiation complete"
            );

            Ok(serde_json::json!({ 
                "payments_queued": payment_effects.len(), 
                "failed_reservations": failed_reservations 
            }))
        })
    }
}

#[derive(Debug)]
pub struct FinalizeSettlementHandler;

impl StepHandler for FinalizeSettlementHandler {
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let all_effects: Vec<StateEffect> = context.get("all_effects").unwrap_or_default();
            let mut success_count = 0;
            let mut fail_count = 0;

            event!(Level::INFO,
                dvp_finalize_count = all_effects.iter().filter(|e| matches!(e, StateEffect::Financial(FinancialEffect::DvPFinalize{..}))).count(),
                dvp_cancel_count = all_effects.iter().filter(|e| matches!(e, StateEffect::Financial(FinancialEffect::DvPCancel{..}))).count(),
                "🔧 Starting settlement finalization"
            );

            for effect in all_effects.iter() {
                match effect {
                    StateEffect::Financial(FinancialEffect::DvPFinalize { trade_id }) => {
                        let trade_id_str = trade_id.to_string()[..8].to_string();
                        
                        if let Err(e) = engine.state.financial_system.clearing_house.csd
                            .finalize_book_entry_transfer(trade_id) {
                            event!(Level::ERROR,
                                trade_id = %trade_id_str,
                                error = %e,
                                "🚨 CRITICAL: Failed to finalize asset leg"
                            );
                        } else {
                            success_count += 1;
                            event!(Level::INFO,
                                trade_id = %trade_id_str,
                                "✅ Asset transfer completed"
                            );
                        }
                    }
                    StateEffect::Financial(FinancialEffect::DvPCancel { trade_id }) => {
                        let trade_id_str = trade_id.to_string()[..8].to_string();
                        
                        if let Err(e) = engine.state.financial_system.clearing_house.csd
                            .cancel_security_reservation(trade_id) {
                            event!(Level::ERROR,
                                trade_id = %trade_id_str,
                                error = %e,
                                "🚨 CRITICAL: Failed to cancel reservation"
                            );
                        } else {
                            fail_count += 1;
                            event!(Level::INFO,
                                trade_id = %trade_id_str,
                                "↩️ Reservation cancelled"
                            );
                        }
                    }
                    _ => {}
                }
            }

            event!(Level::INFO,
                settlements_finalized = success_count,
                settlements_cancelled = fail_count,
                success_rate = if (success_count + fail_count) > 0 { 
                    (success_count as f64 / (success_count + fail_count) as f64) * 100.0 
                } else { 0.0 },
                "📋 Settlement batch completed"
            );

            Ok(serde_json::json!({ 
                "settlements_finalized": success_count, 
                "settlements_cancelled": fail_count 
            }))
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

            run_rtgs(&mut engine.state).map_err(|e| format!("RTGS execution failed: {:?}", e))?;

            let mut finalization_effects = Vec::new();

            for p in &engine.state.financial_system.rtgs.settled {
                if let TransactionContext::TradeSettlement { trade_id } = p.context {
                    finalization_effects.push(StateEffect::Financial(FinancialEffect::DvPFinalize { trade_id }));
                }
            }
            for (p, _reason) in &engine.state.financial_system.rtgs.rejected {
                if let TransactionContext::TradeSettlement { trade_id } = p.context {
                    finalization_effects.push(StateEffect::Financial(FinancialEffect::DvPCancel { trade_id }));
                }
            }
            let mut all_effects: Vec<StateEffect> = context.get("all_effects").unwrap_or_default();
            all_effects.extend(finalization_effects);

            context.store("all_effects", &all_effects)?;
            let final_pending = engine.state.financial_system.rtgs.pending.len();
            let settled_count = initial_pending - final_pending;

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