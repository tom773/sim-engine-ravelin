use super::{StepContext, StepHandler, StepResult};
use crate::executor::SimulationEngine;
use domains::{ResolutionContext, ResolutionPhase};
use rand::prelude::*;
use sim_core::*;
use std::time::Instant;
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
    fn execute(
        &self,
        engine: &mut SimulationEngine,
        _context: &mut StepContext,
        _rng: &mut dyn RngCore,
    ) -> StepResult {
        execute_step(|| {
            engine.state.advance_time();
            let upkeep_actions = engine.process_financial_updates();
            let (_, _, upkeep_effects) = engine.execute_actions(&upkeep_actions);
            engine
                .state
                .apply_effects(&upkeep_effects)
                .map_err(|e| e.to_string())?;

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
    fn execute(
        &self,
        engine: &mut SimulationEngine,
        context: &mut StepContext,
        rng: &mut dyn RngCore,
    ) -> StepResult {
        execute_step(|| {
            let intentions = engine.gather_intentions(rng);
            let categorized = engine
                .domain_registry
                .categorize_intentions_by_phase(intentions.clone());

            context.store("intentions", &intentions)?;
            context.store("categorized_intentions", &categorized)?;

            Ok(serde_json::json!({ "total_intentions": intentions.len() }))
        })
    }
}
#[derive(Debug)]
pub struct PhaseResolutionHandler {
    pub phase: ResolutionPhase,
}
impl StepHandler for PhaseResolutionHandler {
    fn execute(
        &self,
        engine: &mut SimulationEngine,
        context: &mut StepContext,
        _rng: &mut dyn RngCore,
    ) -> StepResult {
        execute_step(|| {
            let categorized = context.get_categorized_intentions()?;
            let intentions = categorized.get(&self.phase).cloned().unwrap_or_default();
            if intentions.is_empty() {
                return Ok(serde_json::json!({ "actions": 0, "effects": 0 }));
            }

            let resolution_context = ResolutionContext {
                state: &engine.state,
                current_tick: engine.state.ticknum,
            };
            let action_offset = context.get_all_actions().unwrap_or_default().len();
            let effect_offset = context.get_all_effects().unwrap_or_default().len();

            let (action_records, action_to_effect_indices, effects) = engine
                .resolve_and_execute_phase(
                    &intentions,
                    &resolution_context,
                    action_offset,
                    effect_offset,
                );

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
    fn execute(
        &self,
        engine: &mut SimulationEngine,
        context: &mut StepContext,
        _rng: &mut dyn RngCore,
    ) -> StepResult {
        execute_step(|| {
            let all_effects = context.get_all_effects().unwrap_or_default();
            let market_effects: Vec<StateEffect> = all_effects
                .into_iter()
                .filter(|e| matches!(e, StateEffect::Market(_)))
                .collect();
            engine
                .state
                .apply_effects(&market_effects)
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "market_effects_applied": market_effects.len() }))
        })
    }
}
#[derive(Debug)]
pub struct ClearMarketsHandler;
impl StepHandler for ClearMarketsHandler {
    fn execute(
        &self,
        engine: &mut SimulationEngine,
        context: &mut StepContext,
        _rng: &mut dyn RngCore,
    ) -> StepResult {
        execute_step(|| {
            let (trades, snapshots) = engine.clear_all_markets();

            let snapshots_s: std::collections::HashMap<String, MarketView> = snapshots
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect();

            context.store("trades", &trades)?;
            context.store("market_snapshots", &snapshots_s)?;
            Ok(serde_json::json!({ "trades_generated": trades.len() }))
        })
    }
}
#[derive(Debug)]
pub struct SettleTradesHandler;
impl StepHandler for SettleTradesHandler {
    fn execute(
        &self,
        engine: &mut SimulationEngine,
        context: &mut StepContext,
        _rng: &mut dyn RngCore,
    ) -> StepResult {
        execute_step(|| {
            let trades = context.get_trades().unwrap_or_default();
            let settlement_effects = engine.settle_trades(&trades);

            let mut all_effects = context.get_all_effects().unwrap_or_default();
            all_effects.extend(settlement_effects.clone());
            context.store("all_effects", &all_effects)?;

            Ok(serde_json::json!({ "settlement_effects": settlement_effects.len() }))
        })
    }
}
#[derive(Debug)]
pub struct ApplyAllEffectsHandler;
impl StepHandler for ApplyAllEffectsHandler {
    fn execute(
        &self,
        engine: &mut SimulationEngine,
        context: &mut StepContext,
        rng: &mut dyn RngCore,
    ) -> StepResult {
        execute_step(|| {
            let mut all_effects = context.get_all_effects().unwrap_or_default();

            let labour_effects = engine.match_labour_markets(rng);
            all_effects.extend(labour_effects);
            for effect in &all_effects {
                if let Some(event) = event_from_effect(effect) {
                    engine.event_log.push(event);
                }
            }
            engine
                .state
                .apply_effects(&all_effects)
                .map_err(|e| e.to_string())?;

            context.store("all_effects", &all_effects)?;
            Ok(serde_json::json!({"total_effects_applied": all_effects.len()}))
        })
    }
}
#[derive(Debug)]
pub struct UpdateHistoryHandler;
impl StepHandler for UpdateHistoryHandler {
    fn execute(
        &self,
        engine: &mut SimulationEngine,
        context: &mut StepContext,
        _rng: &mut dyn RngCore,
    ) -> StepResult {
        execute_step(|| {
            let trades = context.get_trades().unwrap_or_default();
            let snapshots = context.get_market_snapshots().unwrap_or_default();
            engine.update_market_history(&trades, &snapshots);
            engine
                .state
                .financial_system
                .update_yield_curve(engine.state.current_date);
            let tick_record = TickRecord {
                tick_number: engine.state.ticknum,
                date: engine.state.current_date,
                intentions: context.get_intentions().unwrap_or_default(),
                actions: context.get_all_actions().unwrap_or_default(),
                effects: context.get_all_effects().unwrap_or_default(),
                action_to_effect_indices: context
                    .get_action_to_effect_indices()
                    .unwrap_or_default(),
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
    fn execute(
        &self,
        engine: &mut SimulationEngine,
        context: &mut StepContext,
        _rng: &mut dyn RngCore,
    ) -> StepResult {
        execute_step(|| {
            let mut all_trades = Vec::new();

            let open_auction_ids: Vec<Uuid> = engine
                .state
                .financial_system
                .exchange
                .open_auctions
                .iter()
                .filter(|(_, auction)| auction.status == AuctionStatus::Open)
                .map(|(id, _)| *id)
                .collect();

            for auction_id in open_auction_ids {
                let trades = engine
                    .state
                    .financial_system
                    .exchange
                    .conduct_dutch_auction(&auction_id, &engine.state.financial_system.instruments);
                all_trades.extend(trades);
            }

            let auction_effects: Vec<StateEffect> = all_trades
                .into_iter()
                .map(|trade| StateEffect::Financial(FinancialEffect::DvP { trade }))
                .collect();

            if !auction_effects.is_empty() {
                let mut all_effects = context.get_all_effects().unwrap_or_default();
                all_effects.extend(auction_effects);
                context.store("all_effects", &all_effects)?;
            }

            Ok(serde_json::json!({ "debt_auctions_conducted": 1 }))
        })
    }
}

fn event_from_effect(effect: &StateEffect) -> Option<SimEvent> {
    match effect {
        StateEffect::Financial(FinancialEffect::TransferFunds {
            from,
            to,
            amount,
            context,
        }) => Some(SimEvent::CashFlow(sim_core::CashFlowEvent {
            from_agent_id: *from,
            to_agent_id: *to,
            amount: *amount,
            reason: context.clone(),
            ts: chrono::Utc::now(),
        })),
        StateEffect::Financial(FinancialEffect::PayWages {
            employer,
            employee,
            amount,
        }) => Some(SimEvent::CashFlow(sim_core::CashFlowEvent {
            from_agent_id: *employer,
            to_agent_id: *employee,
            amount: *amount,
            reason: "WagePayment".to_string(),
            ts: chrono::Utc::now(),
        })),
        StateEffect::Financial(FinancialEffect::DvP { trade }) => {
            Some(SimEvent::MatchedTrade(sim_core::MatchedTradeEvent {
                trade_id: trade.trade_id,
                market_id: trade.market_id.clone(),
                buyer_id: trade.buyer,
                seller_id: trade.seller,
                quantity: trade.quantity,
                price: trade.price,
                ts: chrono::Utc::now(),
            }))
        }
        StateEffect::Financial(FinancialEffect::CreateInstrument {
            instrument,
            creditor,
            debtor,
            quantity,
        }) => Some(SimEvent::InstrumentLifecycle(
            sim_core::InstrumentLifecycleEvent::Created {
                instrument_id: instrument.id,
                creditor_id: *creditor,
                debtor_id: *debtor,
                quantity: *quantity,
                instrument_type: instrument.type_as_string().to_string(),
            },
        )),
        StateEffect::Financial(FinancialEffect::RemoveInstrument(instrument_id)) => Some(
            SimEvent::InstrumentLifecycle(sim_core::InstrumentLifecycleEvent::Removed {
                instrument_id: *instrument_id,
            }),
        ),
        StateEffect::Financial(FinancialEffect::RecordTransaction(tx)) => {
            Some(SimEvent::TransactionRecord(tx.clone()))
        }
        StateEffect::Financial(FinancialEffect::AdjustPosition {
            owner,
            instrument_id,
            delta_quantity,
            side,
            ..
        }) => Some(SimEvent::BalanceSheetUpdate(
            sim_core::BalanceSheetUpdateEvent {
                owner_id: *owner,
                instrument_id: *instrument_id,
                quantity_change: *delta_quantity,
                new_total_quantity: 0.0,
                side: side.clone(),
            },
        )),
        _ => None,
    }
}
