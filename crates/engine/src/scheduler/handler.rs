use super::{StepContext, StepHandler, StepResult, TickStep};
use crate::{SimulationEngine};
use domains::{ResolutionContext, ResolutionPhase};
use sim_core::*;
use std::collections::HashMap;
use std::time::Instant;

pub struct UpkeepHandler;

impl StepHandler for UpkeepHandler {
    fn execute(
        &self, engine: &mut SimulationEngine, _context: &mut StepContext, _rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        let start = Instant::now();

        engine.state.advance_time();
        println!("[UPKEEP] Advanced to date: {}", engine.state.current_date);

        engine.update_agent_expectations();

        let upkeep_actions = engine.process_financial_updates();
        let (_, _, upkeep_effects) = engine.execute_actions(&upkeep_actions);

        if let Err(e) = engine.state.apply_effects(&upkeep_effects) {
            return StepResult::failure(
                start.elapsed().as_millis() as u64,
                format!("Failed to apply upkeep effects: {}", e),
            );
        }

        let metadata = serde_json::json!({
            "date": engine.state.current_date.format("%Y-%m-%d").to_string(),
            "upkeep_actions": upkeep_actions.len(),
            "upkeep_effects": upkeep_effects.len(),
            "tick_number": engine.state.ticknum
        });

        StepResult::success(start.elapsed().as_millis() as u64, metadata)
    }

    fn name(&self) -> &'static str {
        "UpkeepHandler"
    }
}

pub struct GatherIntentionsHandler;

impl StepHandler for GatherIntentionsHandler {
    fn execute(
        &self, engine: &mut SimulationEngine, context: &mut StepContext, rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        let start = Instant::now();

        let intentions = engine.gather_intentions(rng);

        let categorized = engine.domain_registry.categorize_intentions_by_phase(intentions.clone());

        if let Err(e) = context.store_intentions(intentions.clone()) {
            return StepResult::failure(
                start.elapsed().as_millis() as u64,
                format!("Failed to store intentions: {}", e),
            );
        }

        if let Err(e) = context.store_categorized_intentions(categorized.clone()) {
            return StepResult::failure(
                start.elapsed().as_millis() as u64,
                format!("Failed to store categorized intentions: {}", e),
            );
        }

        let metadata = serde_json::json!({
            "total_intentions": intentions.len(),
            "by_phase": categorized.iter().map(|(phase, intents)| {
                (format!("{:?}", phase), intents.len())
            }).collect::<HashMap<String, usize>>(),
            "by_agent_type": Self::categorize_intentions_by_agent(&intentions, &engine.state)
        });

        StepResult::success(start.elapsed().as_millis() as u64, metadata)
    }

    fn name(&self) -> &'static str {
        "GatherIntentionsHandler"
    }
}

impl GatherIntentionsHandler {
    fn categorize_intentions_by_agent(intentions: &[SimIntention], state: &SimState) -> serde_json::Value {
        let mut counts = HashMap::new();
        for intention in intentions {
            let agent_id = intention.agent_id();
            let agent_type = if state.agents.banks.contains_key(&agent_id) {
                "Bank"
            } else if state.agents.consumers.contains_key(&agent_id) {
                "Consumer"
            } else if state.agents.firms.contains_key(&agent_id) {
                "Firm"
            } else if agent_id == state.financial_system.government.id {
                "Government"
            } else if agent_id == state.financial_system.central_bank.id {
                "CentralBank"
            } else {
                "Other"
            };
            *counts.entry(agent_type).or_insert(0) += 1;
        }
        serde_json::to_value(counts).unwrap_or(serde_json::Value::Null)
    }
}

pub struct PhaseResolutionHandler {
    phase: ResolutionPhase,
}

impl PhaseResolutionHandler {
    pub fn new(phase: ResolutionPhase) -> Self {
        Self { phase }
    }
}

impl StepHandler for PhaseResolutionHandler {
    fn execute(
        &self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        let start = Instant::now();

        let intentions = match context.get_intentions_for_phase(self.phase) {
            Ok(intents) => intents,
            Err(e) => {
                return StepResult::failure(
                    start.elapsed().as_millis() as u64,
                    format!("Failed to get intentions for phase {:?}: {}", self.phase, e),
                );
            }
        };

        if intentions.is_empty() {
            let metadata = serde_json::json!({
                "phase": format!("{:?}", self.phase),
                "intentions": 0,
                "actions": 0,
                "effects": 0
            });
            return StepResult::success(start.elapsed().as_millis() as u64, metadata);
        }

        let resolution_context = ResolutionContext { state: &engine.state, current_tick: engine.state.ticknum };

        let action_offset = context.get_all_actions().map(|a| a.len()).unwrap_or(0);
        let effect_offset = context.get_all_effects().map(|e| e.len()).unwrap_or(0);

        let (action_records, action_to_effect_indices, effects) =
            engine.resolve_and_execute_phase(&intentions, &resolution_context, action_offset, effect_offset);

        if let Err(e) = context.store_phase_actions(self.phase, action_records.clone()) {
            return StepResult::failure(
                start.elapsed().as_millis() as u64,
                format!("Failed to store phase actions: {}", e),
            );
        }

        if let Err(e) = context.store_phase_effects(self.phase, effects.clone()) {
            return StepResult::failure(
                start.elapsed().as_millis() as u64,
                format!("Failed to store phase effects: {}", e),
            );
        }

        let mut all_actions = context.get_all_actions().unwrap_or_default();
        all_actions.extend(action_records.clone());
        let _ = context.store_all_actions(all_actions);

        let mut all_effects = context.get_all_effects().unwrap_or_default();
        all_effects.extend(effects.clone());
        let _ = context.store_all_effects(all_effects);

        let mut mapping = context.get_action_to_effect_indices().unwrap_or_default();
        for (action_idx, effect_indices) in action_to_effect_indices.clone() {
            mapping.insert(action_idx, effect_indices);
        }
        let _ = context.store_action_to_effect_indices(mapping);

        let metadata = serde_json::json!({
            "phase": format!("{:?}", self.phase),
            "intentions": intentions.len(),
            "actions": action_records.len(),
            "effects": effects.len(),
            "action_to_effect_mappings": action_to_effect_indices.clone().len()
        });

        StepResult::success(start.elapsed().as_millis() as u64, metadata)
    }

    fn validates_preconditions(&self, context: &StepContext) -> Result<(), String> {
        if !context.step_completed_successfully(TickStep::GatherIntentions) {
            return Err("GatherIntentions step must complete successfully first".to_string());
        }

        match self.phase {
            ResolutionPhase::Market => {
                if !context.step_completed_successfully(TickStep::ResolveIndependentPhase) {
                    return Err("Independent phase must complete before Market phase".to_string());
                }
            }
            ResolutionPhase::Dependent => {
                if !context.step_completed_successfully(TickStep::ApplyMarketEffectsForPriceDiscovery) {
                    return Err("Market effects must be applied before Dependent phase".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        match self.phase {
            ResolutionPhase::Independent => "IndependentPhaseHandler",
            ResolutionPhase::Market => "MarketPhaseHandler",
            ResolutionPhase::Dependent => "DependentPhaseHandler",
        }
    }
}

pub struct ApplyMarketEffectsHandler;

impl StepHandler for ApplyMarketEffectsHandler {
    fn execute(
        &self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        let start = Instant::now();

        let market_effects = match context.get_phase_effects(ResolutionPhase::Market) {
            Ok(effects) => effects,
            Err(e) => {
                return StepResult::failure(
                    start.elapsed().as_millis() as u64,
                    format!("Failed to get market effects: {}", e),
                );
            }
        };

        let market_state_effects: Vec<StateEffect> =
            market_effects.into_iter().filter(|e| matches!(e, StateEffect::Market(_))).collect();

        if !market_state_effects.is_empty() {
            if let Err(e) = engine.state.apply_effects(&market_state_effects) {
                return StepResult::failure(
                    start.elapsed().as_millis() as u64,
                    format!("Failed to apply market effects: {}", e),
                );
            }
        }

        let metadata = serde_json::json!({
            "market_effects_applied": market_state_effects.len()
        });

        StepResult::success(start.elapsed().as_millis() as u64, metadata)
    }

    fn validates_preconditions(&self, context: &StepContext) -> Result<(), String> {
        if !context.step_completed_successfully(TickStep::ResolveMarketPhase) {
            return Err("Market phase must complete successfully first".to_string());
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "ApplyMarketEffectsHandler"
    }
}

pub struct ClearMarketsHandler;

impl StepHandler for ClearMarketsHandler {
    fn execute(
        &self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        let start = Instant::now();

        let (trades, snapshots) = engine.state.financial_system.exchange.clear_markets(
            engine.state.ticknum as i64,
            &engine.state.financial_system.instruments, // Just pass the instruments
        );

        if let Err(e) = context.store_trades(trades.clone()) {
            return StepResult::failure(start.elapsed().as_millis() as u64, format!("Failed to store trades: {}", e));
        }

        if let Err(e) = context.store_market_snapshots(snapshots.clone()) {
            return StepResult::failure(
                start.elapsed().as_millis() as u64,
                format!("Failed to store market snapshots: {}", e),
            );
        }

        let metadata = serde_json::json!({
            "trades_generated": trades.len(),
            "markets_cleared": snapshots.len(),
            "total_volume": trades.iter().map(|t| t.quantity).sum::<f64>(),
            "total_value": trades.iter().map(|t| t.quantity * t.price).sum::<f64>()
        });

        StepResult::success(start.elapsed().as_millis() as u64, metadata)
    }

    fn validates_preconditions(&self, context: &StepContext) -> Result<(), String> {
        if !context.step_completed_successfully(TickStep::ResolveDependentPhase) {
            return Err("Dependent phase must complete successfully first".to_string());
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "ClearMarketsHandler"
    }
}

pub struct SettleTradesHandler;

impl StepHandler for SettleTradesHandler {
    fn execute(
        &self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        let start = Instant::now();

        let trades = match context.get_trades() {
            Ok(trades) => trades,
            Err(e) => {
                return StepResult::failure(start.elapsed().as_millis() as u64, format!("Failed to get trades: {}", e));
            }
        };

        let settlement_effects = engine.settle_trades(trades.clone());

        let mut all_effects = context.get_all_effects().unwrap_or_default();
        all_effects.extend(settlement_effects.clone());
        let _ = context.store_all_effects(all_effects);

        let metadata = serde_json::json!({
            "trades_settled": trades.len(),
            "settlement_effects": settlement_effects.len()
        });

        StepResult::success(start.elapsed().as_millis() as u64, metadata)
    }

    fn validates_preconditions(&self, context: &StepContext) -> Result<(), String> {
        if !context.step_completed_successfully(TickStep::ClearMarkets) {
            return Err("Markets must be cleared before settling trades".to_string());
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "SettleTradesHandler"
    }
}

pub struct ApplyAllEffectsHandler;

impl StepHandler for ApplyAllEffectsHandler {
    fn execute(
        &self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        let start = Instant::now();

        let all_effects = match context.get_all_effects() {
            Ok(effects) => effects,
            Err(e) => {
                return StepResult::failure(
                    start.elapsed().as_millis() as u64,
                    format!("Failed to get all effects: {}", e),
                );
            }
        };

        let labour_effects = engine.state.financial_system.exchange.clear_labour_markets(&engine.state.clone());
        let mut final_effects = all_effects;
        final_effects.extend(labour_effects.clone());

        if let Err(e) = engine.state.apply_effects(&final_effects) {
            return StepResult::failure(
                start.elapsed().as_millis() as u64,
                format!("Failed to apply all effects: {}", e),
            );
        }

        let _ = context.store_all_effects(final_effects.clone());

        let metadata = serde_json::json!({
            "total_effects_applied": final_effects.len(),
            "labour_effects": labour_effects.len()
        });

        StepResult::success(start.elapsed().as_millis() as u64, metadata)
    }

    fn validates_preconditions(&self, context: &StepContext) -> Result<(), String> {
        if !context.step_completed_successfully(TickStep::SettleTrades) {
            return Err("Trades must be settled before applying all effects".to_string());
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "ApplyAllEffectsHandler"
    }
}

pub struct UpdateHistoryHandler;

impl StepHandler for UpdateHistoryHandler {
    fn execute(
        &self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        let start = Instant::now();

        let trades = context.get_trades().unwrap_or_default();
        let snapshots = HashMap::new();

        engine.update_market_history(&trades, &snapshots);

        engine.state.financial_system.update_yield_curve(engine.state.current_date);

        let all_actions = context.get_all_actions().unwrap_or_default();
        let all_effects = context.get_all_effects().unwrap_or_default();
        let action_to_effect_indices = context.get_action_to_effect_indices().unwrap_or_default();
        let intentions = context.get_intentions().unwrap_or_default();

        let tick_record = TickRecord {
            tick_number: engine.state.ticknum,
            date: engine.state.current_date,
            intentions,
            actions: all_actions,
            effects: all_effects,
            action_to_effect_indices,
            trades: trades.clone(),
        };

        engine.state.history.add_tick_record(tick_record.clone());

        crate::dbg_evt!(tick_record);

        let metadata = serde_json::json!({
            "trades_recorded": trades.len(),
            "history_updated": true
        });

        StepResult::success(start.elapsed().as_millis() as u64, metadata)
    }

    fn validates_preconditions(&self, context: &StepContext) -> Result<(), String> {
        if !context.step_completed_successfully(TickStep::ApplyAllEffects) {
            return Err("All effects must be applied before updating history".to_string());
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "UpdateHistoryHandler"
    }
}

pub struct PersistDataHandler;

impl StepHandler for PersistDataHandler {
    fn execute(
        &self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        let start = Instant::now();

        if let Some(writer) = &engine.db_writer {
            let all_actions = context.get_all_actions().unwrap_or_default();
            let all_effects = context.get_all_effects().unwrap_or_default();
            let action_to_effect_indices = context.get_action_to_effect_indices().unwrap_or_default();
            let trades = context.get_trades().unwrap_or_default();

            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    engine
                        .persist_tick_batch(
                            writer,
                            engine.state.current_date,
                            &all_actions,
                            &all_effects,
                            &action_to_effect_indices,
                            &trades,
                        )
                        .await
                })
            });

            if let Err(e) = result {
                println!("[WARNING] Failed to persist tick data: {}", e);
                let metadata = serde_json::json!({ "persisted": false, "error": e.to_string() });
                return StepResult::success(start.elapsed().as_millis() as u64, metadata);
            }

            let metadata = serde_json::json!({
                "persisted": true,
                "actions": all_actions.len(),
                "effects": all_effects.len(),
                "trades": trades.len()
            });
            StepResult::success(start.elapsed().as_millis() as u64, metadata)
        } else {
            let metadata = serde_json::json!({ "persisted": false, "reason": "No database writer available" });
            StepResult::success(start.elapsed().as_millis() as u64, metadata)
        }
    }

    fn validates_preconditions(&self, context: &StepContext) -> Result<(), String> {
        if !context.step_completed_successfully(TickStep::UpdateHistory) {
            return Err("History must be updated before persisting data".to_string());
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "PersistDataHandler"
    }
}
