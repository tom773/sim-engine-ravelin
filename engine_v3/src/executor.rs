use crate::registry::DomainRegistry;
use crate::scheduler::*;
use chrono::{Datelike, NaiveDate};
use domains::prelude::*;
use rand::RngCore;
use rand::prelude::{SliceRandom, ThreadRng};
use std::collections::HashMap;
use uuid::Uuid;

pub struct SimulationEngine {
    pub state: SimState,
    pub domain_registry: DomainRegistry,
    pub decision_models: HashMap<AgentId, Box<dyn DecisionModel>>,
    pub scheduler: TickScheduler,
    pub scheduler_metrics: SchedulerMetrics,
    pub event_log: Vec<SimEvent>,
}

impl SimulationEngine {
    pub fn new(state: SimState) -> Self {
        Self {
            state,
            domain_registry: DomainRegistry::new(),
            decision_models: HashMap::new(),
            scheduler: TickScheduler::new(),
            scheduler_metrics: SchedulerMetrics::new(),
            event_log: Vec::new(),
        }
    }

    pub fn new_with_scheduler(state: SimState) -> Self {
        let mut engine = Self::new(state);
        let scheduler = engine.create_scheduler();
        engine.scheduler = scheduler;
        engine
    }

    fn create_scheduler(&self) -> TickScheduler {
        let mut scheduler = TickScheduler::new();
        scheduler.register_handler(TickStep::Upkeep, UpkeepHandler);
        scheduler.register_handler(TickStep::GatherIntentions, GatherIntentionsHandler);
        scheduler.register_handler(
            TickStep::ResolveIndependentPhase,
            PhaseResolutionHandler { phase: ResolutionPhase::Independent },
        );
        scheduler.register_handler(TickStep::ApplyInstrumentCreation, ApplyInstrumentCreationHandler);
        scheduler.register_handler(
            TickStep::ResolveMarketPhase, 
            PhaseResolutionHandler { phase: ResolutionPhase::Market }
        );
        scheduler.register_handler(TickStep::ApplyMarketEffectsForPriceDiscovery, ApplyMarketEffectsHandler);
        scheduler.register_handler(
            TickStep::ResolveDependentPhase,
            PhaseResolutionHandler { phase: ResolutionPhase::Dependent },
        );
        scheduler.register_handler(TickStep::Auction, DebtAuctionsHandler);
        scheduler.register_handler(TickStep::ClearMarkets, ClearMarketsHandler);
        scheduler.register_handler(TickStep::StartSettlement, StartSettlementHandler);
        scheduler.register_handler(TickStep::RunRTGS, RunRTGSHandler);
        scheduler.register_handler(TickStep::FinalizeSettlement, FinalizeSettlementHandler); 
        scheduler.register_handler(TickStep::ApplyAllEffects, ApplyAllEffectsHandler);
        scheduler.register_handler(TickStep::UpdateHistory, UpdateHistoryHandler);
        scheduler.print_execution_plan();
        scheduler
    }

    pub fn run_tick(&mut self, rng: &mut dyn RngCore) -> (TickExecutionResult, Vec<SimEvent>) {
        self.event_log.clear();

        let scheduler = std::mem::take(&mut self.scheduler);
        let execution_result = scheduler.execute_tick(self, rng);
        self.scheduler = scheduler;
        self.scheduler_metrics.record_tick(&execution_result);
        if execution_result.success {
            self.state.ticknum += 1;
        }

        (execution_result, std::mem::take(&mut self.event_log))
    }

    pub fn gather_intentions(&self, rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let mut all_intentions = Vec::new();
        for agent_id in self.state.agents.all_agent_ids() {
            if let (Some(model), Some(agent)) =
                (self.decision_models.get(&agent_id), self.state.agents.get_agent_as_any(&agent_id))
            {
                all_intentions.extend(model.decide(agent, &self.state, rng));
            }
        }
        if let Some(model) = self.decision_models.get(&self.state.financial_system.government.id) {
            all_intentions.extend(model.decide(&self.state.financial_system.government, &self.state, rng));
        }
        all_intentions
    }

    pub fn process_financial_updates(&self) -> Vec<SimAction> {
        let mut actions = Vec::new();
        let current_date = self.state.current_date;
        for (instrument_id, instrument) in &self.state.financial_system.instruments {
            match &instrument.instrument_type {
                InstrumentType::Cash(_) => {
                    if is_last_day_of_month(current_date) {
                        actions.push(SimAction::Settlement(SettlementAction::PayInterest {
                            instrument_id: *instrument_id,
                        }));
                    }
                }
                InstrumentType::Bond(details) => {
                    if is_coupon_date(current_date, details) {
                        actions.push(SimAction::Settlement(SettlementAction::ProcessCouponPayment {
                            instrument_id: *instrument_id,
                        }));
                    }
                }
                _ => {}
            }
        }
        actions
    }

    pub fn execute_actions(
        &self, actions: &[SimAction],
    ) -> (Vec<ActionRecord>, HashMap<usize, Vec<usize>>, Vec<StateEffect>) {
        let mut all_effects = Vec::new();
        let mut action_records = Vec::with_capacity(actions.len());
        let mut action_to_effect_indices = HashMap::new();

        for (action_idx, action) in actions.iter().enumerate() {
            let agent_id = action.agent_id();
            let (agent_type, agent_name) = self.get_agent_info(&agent_id);
            action_records.push(ActionRecord {
                id: Uuid::new_v4(),
                action: action.clone(),
                agent_id,
                agent_type,
                agent_name,
            });

            let effects = match self.domain_registry.execute_action(action, &self.state) {
                Ok(effects) => effects,
                Err(e) => {
                    if !e.contains("Missing core support") {
                        println!("[ACTION FAILED] - {:?}: {}", action.name(), e);
                    }
                    vec![]
                }
            };

            if !effects.is_empty() {
                let effect_start_idx = all_effects.len();
                let effect_indices: Vec<usize> = (effect_start_idx..effect_start_idx + effects.len()).collect();
                action_to_effect_indices.insert(action_idx, effect_indices);
                all_effects.extend(effects);
            }
        }
        (action_records, action_to_effect_indices, all_effects)
    }

    pub fn resolve_and_execute_phase(
        &self, intentions: &[SimIntention], context: &ResolutionContext, action_offset: usize, effect_offset: usize,
    ) -> (Vec<ActionRecord>, HashMap<usize, Vec<usize>>, Vec<StateEffect>) {
        let mut phase_actions = Vec::new();
        for intention in intentions {
            let result = self.domain_registry.resolve_intention(intention, context);
            if !result.success {
                continue;
            }
            phase_actions.extend(result.actions);
        }
        let (action_records, mut action_to_effect_indices, effects) = self.execute_actions(&phase_actions);
        for indices in action_to_effect_indices.values_mut() {
            for index in indices {
                *index += effect_offset;
            }
        }
        let adjusted_indices: HashMap<usize, Vec<usize>> =
            action_to_effect_indices.into_iter().map(|(k, v)| (k + action_offset, v)).collect();
        (action_records, adjusted_indices, effects)
    }

    pub fn clear_all_markets(&mut self) -> (Vec<Trade>, HashMap<MarketId, MarketView>) {
        let mut all_trades = Vec::new();

        let market_ids: Vec<MarketId> = self
            .state
            .financial_system
            .exchange
            .goods_markets
            .keys()
            .map(|id| MarketId::Goods(*id))
            .chain(self.state.financial_system.exchange.markets.keys().map(|id| MarketId::Financial(*id)))
            .collect();

        for market_id in &market_ids {
            let trades = match market_id {
                MarketId::Goods(id) => self
                    .state
                    .financial_system
                    .exchange
                    .goods_markets
                    .get_mut(id)
                    .map(|m| m.clear_and_match(market_id))
                    .unwrap_or_default(),
                MarketId::Financial(id) => self
                    .state
                    .financial_system
                    .exchange
                    .markets
                    .get_mut(id)
                    .map(|m| m.book.clear_and_match(market_id))
                    .unwrap_or_default(),
                MarketId::Labour(_) => vec![],
            };
            all_trades.extend(trades);
        }

        let all_snapshots: HashMap<MarketId, MarketView> =
            market_ids.into_iter().map(|id| (id.clone(), self.state.market_view(&id).unwrap_or_default())).collect();

        (all_trades, all_snapshots)
    }

    pub fn settle_trades(&self, trades: &[Trade]) -> Vec<StateEffect> {
        trades
            .iter()
            .flat_map(|trade| {
                self.domain_registry.settle_trade(trade, &self.state).unwrap_or_else(|e| {
                    println!("[ERROR] Failed to settle trade: {}", e);
                    vec![]
                })
            })
            .collect()
    }

    pub fn match_labour_markets(&mut self, rng: &mut dyn RngCore) -> Vec<StateEffect> {
        let mut effects = Vec::new();
        for market in self.state.financial_system.exchange.labour_markets.values_mut() {
            market.job_offers.shuffle(rng);
            for offer in &mut market.job_offers {
                if offer.quantity == 0 {
                    continue;
                }
                if let Some(app_idx) =
                    market.job_applications.iter().position(|app| app.reservation_wage <= offer.wage_rate)
                {
                    let app = market.job_applications.remove(app_idx);
                    let contract = EmploymentContract {
                        employee_id: app.consumer_id,
                        wage_rate: offer.wage_rate,
                        hours: offer.hours_required,
                        start_date: self.state.current_date,
                    };
                    effects.push(StateEffect::Agent(AgentEffect::EstablishEmployment {
                        firm_id: offer.firm_id,
                        consumer_id: app.consumer_id,
                        contract,
                    }));
                    offer.quantity -= 1;
                }
            }
        }
        effects
    }

    pub fn update_market_history(&mut self, trades: &[Trade], snapshots: &HashMap<MarketId, MarketView>) {
        let current_date = self.state.current_date;
        let history = &mut self.state.history;
        for (market_id, snapshot) in snapshots {
            let market_trades: Vec<&Trade> = trades.iter().filter(|t| &t.market_id == market_id).collect();
            let close = market_trades.last().map(|t| t.price.to_f64()).or(snapshot.last);

            let prices = market_trades.iter().map(|t| t.price);

            let high = prices.clone().max().map(|m| m.to_f64());
            let low = prices.min().map(|m| m.to_f64());

            let tick = MarketTick {
                date: current_date,
                open: market_trades.first().map(|t| t.price.to_f64()),
                high,
                low,
                close,
                volume: snapshot.volume,
                turnover: snapshot.turnover,
                last_price: snapshot.last,
                last_qty: None,
                best_bid: snapshot.mid,
                best_ask: snapshot.mid,
                spread: snapshot.spread,
            };
            history.market_ticks.entry(market_id.clone()).or_default().push_back(tick);
        }
    }

    pub fn get_agent_info(&self, agent_id: &AgentId) -> (String, Option<String>) {
        let agent_type = self.state.get_agent_type_string(agent_id).unwrap_or("Unknown").to_string();

        let agent_name = if let Some(bank) = self.state.agents.banks.get(agent_id) {
            Some(bank.name.clone())
        } else if self.state.agents.consumers.contains_key(agent_id) {
            Some(format!("Consumer {}", &agent_id.to_string()[..8]))
        } else if let Some(firm) = self.state.agents.firms.get(agent_id) {
            Some(firm.name.clone())
        } else if *agent_id == self.state.financial_system.government.id {
            Some("Government".to_string())
        } else if *agent_id == self.state.financial_system.central_bank.id {
            Some("Central Bank".to_string())
        } else {
            None
        };

        (agent_type, agent_name)
    }
}

fn is_last_day_of_month(date: NaiveDate) -> bool {
    date.month() != (date + chrono::Duration::days(1)).month()
}
fn is_coupon_date(date: NaiveDate, bond: &BondDetails) -> bool {
    if bond.frequency == 0 {
        return false;
    }
    let months_since_issue =
        ((date.year() - bond.issue_date.year()) * 12 + date.month() as i32 - bond.issue_date.month() as i32) as u32;
    date.day() == bond.issue_date.day() && months_since_issue > 0 && months_since_issue % (12 / bond.frequency) == 0
}

pub fn run_simulation(engine: &mut SimulationEngine) -> Vec<TickExecutionResult> {
    let mut rng = ThreadRng::default();
    let total_ticks = engine.state.config.iterations;
    let mut results = Vec::with_capacity(total_ticks as usize);
    for i in 0..total_ticks {
        println!("[RUNNER] Executing Tick {}/{} ({})", i + 1, total_ticks, engine.state.current_date);
        let result = engine.run_tick(&mut rng);
        if !result.0.success {
            results.push(result.0);
            break;
        }
        results.push(result.0);
    }
    results
}