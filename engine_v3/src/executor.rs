use crate::registry::DomainRegistry;
use crate::scheduler::*;
use domains::prelude::*;
use ordered_float::NotNan;
use rand::RngCore;
use rand::prelude::ThreadRng;
use rust_decimal::prelude::*;
use sim_core::types::markets::BondPricingTerms;
use std::collections::HashMap;
use uuid::Uuid;

pub struct SimulationEngine {
    pub initial_state: Option<SimState>,
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
            initial_state: None,
            state,
            domain_registry: DomainRegistry::new(),
            decision_models: HashMap::new(),
            scheduler: TickScheduler::new(),
            scheduler_metrics: SchedulerMetrics::new(),
            event_log: Vec::new(),
        }
    }

    pub fn new_with_scheduler(state: SimState) -> Self {
        let initial_state = state.clone();
        let mut engine = Self::new(state);
        engine.initial_state = Some(initial_state);
        let scheduler = engine.create_scheduler();
        engine.scheduler = scheduler;
        engine
    }
    pub fn reset(&mut self) -> Result<(), String> {
        if let Some(ref initial) = self.initial_state {
            self.state = initial.clone();
            self.event_log.clear();
            self.scheduler_metrics = SchedulerMetrics::new();
            // Re-register decision models if needed
            Ok(())
        } else {
            Err("No initial state snapshot available".to_string())
        }
    }
    fn create_scheduler(&self) -> TickScheduler {
        let mut scheduler = TickScheduler::new();
        scheduler.register_handler(TickStep::Upkeep, UpkeepHandler);
        scheduler.register_handler(TickStep::GatherIntentions, GatherIntentionsHandler);
        scheduler.register_handler(
            TickStep::ResolveIndependentPhase,
            PhaseResolutionHandler { phase: ResolutionPhase::Independent },
        );
        scheduler
            .register_handler(TickStep::ResolveMarketPhase, PhaseResolutionHandler { phase: ResolutionPhase::Market });
        scheduler.register_handler(TickStep::ApplyMarketEffectsForPriceDiscovery, ApplyMarketEffectsHandler);
        scheduler.register_handler(
            TickStep::ResolveDependentPhase,
            PhaseResolutionHandler { phase: ResolutionPhase::Dependent },
        );
        scheduler.register_handler(TickStep::Auction, DebtAuctionsHandler);
        scheduler.register_handler(TickStep::ClearMarkets, ClearMarketsHandler);
        scheduler.register_handler(TickStep::ClearOvernightMarkets, ClearOvernightHandler);
        scheduler.register_handler(TickStep::SettleTrades, SettleTradesHandler);
        scheduler.register_handler(TickStep::ServiceCredit, CreditServicingHandler);
        scheduler.register_handler(TickStep::ServiceDeposits, DepositServicingHandler);
        scheduler.register_handler(TickStep::ServiceGovernmentDebt, GovCouponsHandler);
        scheduler.register_handler(TickStep::ApplyPaymentQueuing, ApplyPaymentQueuingHandler);
        scheduler.register_handler(TickStep::RunRTGS, RunRTGSHandler);
        scheduler.register_handler(TickStep::ReconcileCredit, CreditReconciliationHandler);
        scheduler.register_handler(TickStep::ApplyAllEffects, ApplyAllEffectsHandler);
        scheduler.register_handler(TickStep::UpdateHistory, UpdateHistoryHandler);
        scheduler
    }

    pub fn run_tick(&mut self, rng: &mut dyn RngCore) -> (TickExecutionResult, Vec<SimEvent>) {
        self.event_log.clear();
        println!("\n===Tick {} ({})===\n", self.state.ticknum, self.state.current_date);
        let scheduler = std::mem::take(&mut self.scheduler);
        let execution_result = scheduler.execute_tick(self, rng);
        self.scheduler = scheduler;
        self.scheduler_metrics.record_tick(&execution_result);
        self.update_agent_memory();
        if execution_result.success {
            self.state.ticknum += 1;
        }
        tracing::warn!("Failed Steps: {:?}", execution_result.failed_steps);
        (execution_result, std::mem::take(&mut self.event_log))
    }
    pub fn run_day(&mut self, rng: &mut dyn rand::RngCore) -> (Vec<TickExecutionResult>, Vec<SimEvent>) {
        self.event_log.clear();
        let mut results = Vec::with_capacity(3);
        let mut day_events = Vec::new();

        for session in Session::ALL {
            self.state.current_session = session;
            let scheduler = std::mem::take(&mut self.scheduler);
            let result = scheduler.execute_tick(self, rng);
            self.scheduler = scheduler;
            self.scheduler_metrics.record_tick(&result);
            results.push(result);
            day_events.extend(self.event_log.drain(..));
        }

        self.state.ticknum += 1;
        (results, day_events)
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

            let dres = self.domain_registry.execute_action(action, &self.state);
            let effects = if dres.success {
                dres.effects
            } else {
                let msg = dres.errors.join("; ");
                if !msg.contains("Missing core support") {
                    println!("[ACTION FAILED] - {:?}: {}", action.name(), msg);
                }
                vec![]
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

    pub fn update_agent_memory(&mut self) {
        let Some(last_tick) = self.state.history.tick_records.back() else { return };
        let goods_config = &self.state.config.goods;

        let mut posted_buys: HashMap<(AgentId, GoodId), f64> = HashMap::new();
        let mut posted_sells: HashMap<(AgentId, GoodId), f64> = HashMap::new();

        for record in &last_tick.actions {
            if let SimAction::Transaction(TransactionAction::PostMarketOrder {
                agent_id,
                market_id,
                side,
                quantity,
                ..
            }) = &record.action
            {
                if let Some(gid) = self.state.financial_system.exchange.symbol_to_good.get(market_id) {
                    match side {
                        Side::Bid => *posted_buys.entry((*agent_id, *gid)).or_default() += *quantity,
                        Side::Ask => *posted_sells.entry((*agent_id, *gid)).or_default() += *quantity,
                    }
                }
            }
        }

        let mut bought: HashMap<(AgentId, GoodId), f64> = HashMap::new();
        let mut sold: HashMap<(AgentId, GoodId), f64> = HashMap::new();

        for trade in &last_tick.trades {
            if let Some(gid) = self.state.financial_system.exchange.symbol_to_good.get(&trade.market_id) {
                *bought.entry((trade.buyer, *gid)).or_default() += trade.quantity;
                *sold.entry((trade.seller, *gid)).or_default() += trade.quantity;
            }
        }

        let mut consumer_updates = Vec::new();
        for (agent_id, gid) in posted_buys.keys().map(|(agent, good)| (*agent, *good)) {
            if self.state.agents.consumers.contains_key(&agent_id) {
                let filled_qty = bought.get(&(agent_id, gid)).copied().unwrap_or(0.0);
                let good_symbol = self.state.financial_system.exchange.good_to_symbol.get(&gid).cloned();
                let anchor = if let Some(symbol) = good_symbol {
                    self.state
                        .financial_system
                        .exchange
                        .fair_price_for_good(&gid)
                        .map(|m| m.to_f64())
                        .or_else(|| self.state.market_view(&symbol).and_then(|v| v.last_or_mid()))
                        .unwrap_or(1.0)
                } else {
                    1.0
                };
                consumer_updates.push((agent_id, gid, filled_qty, anchor));
            }
        }

        for (id, consumer) in self.state.agents.consumers.iter_mut() {
            for &(_, gid, filled_qty, anchor) in consumer_updates.iter().filter(|(agent_id, _, _, _)| agent_id == id) {
                let reservation = consumer
                    .adaptive
                    .reservation
                    .entry(gid)
                    .or_insert(anchor * (1.0 + goods_config.reservation_nudge_up));
                let cap = anchor * goods_config.reservation_cap_mult;

                if filled_qty <= 1e-9 {
                    *reservation = (*reservation * (1.0 + goods_config.reservation_nudge_up)).min(cap);
                } else {
                    *reservation = 0.5 * *reservation + 0.5 * (anchor * (1.0 - goods_config.reservation_nudge_down));
                }
            }
        }

        let alpha = goods_config.sell_through_alpha;
        for (id, firm) in self.state.agents.firms.iter_mut() {
            for ((agent, gid), posted_qty) in posted_sells.iter().filter(|((agent, _), _)| agent == id) {
                let sold_qty = sold.get(&(*agent, *gid)).unwrap_or(&0.0);
                let sell_through = if *posted_qty > 1e-9 { (sold_qty / *posted_qty).min(1.5) } else { 0.0 };

                let metrics = firm.behaviour.per_good.entry(*gid).or_default();
                metrics.sell_through_ema = alpha * sell_through + (1.0 - alpha) * metrics.sell_through_ema;
                metrics.sales_ema = alpha * sold_qty + (1.0 - alpha) * metrics.sales_ema;
            }
        }
    }
    pub fn log_orderbook_for_good(&self, good_name: &str) {
        let good_id = self
            .state
            .financial_system
            .goods
            .goods
            .iter()
            .find(|(_, g)| g.name.to_lowercase().contains(&good_name.to_lowercase()))
            .map(|(id, _)| *id);

        let Some(gid) = good_id else {
            tracing::warn!(good = good_name, "Good not found in registry");
            return;
        };

        let Some(symbol) = self.state.financial_system.exchange.good_to_symbol.get(&gid) else {
            tracing::warn!(good = good_name, ?gid, "No symbol found for good");
            return;
        };

        let Some(market) = self.state.financial_system.exchange.markets.get(symbol) else {
            tracing::warn!(good = good_name, ?gid, "No market found for good");
            return;
        };

        if let MarketType::Goods(goods_market) = market {
            let book = &goods_market.book;
            tracing::info!(
                "Order Book for {} - Best Bid: {:?} | Best Ask: {:?} | Bid Lvls: {:?} | Ask Lvls: {:?}",
                good_name,
                book.depth_summary().best_bid,
                book.depth_summary().best_ask,
                book.depth_summary().bid_levels,
                book.depth_summary().ask_levels
            );
        }
    }

    pub fn log_orderbooks(&self, goods: &[&str]) {
        for good in goods {
            self.log_orderbook_for_good(good);
        }
    }
    pub fn clear_all_markets(&mut self) -> (Vec<Trade>, HashMap<Symbol, MarketView>) {
        let mut all_trades = Vec::new();

        let market_symbols: Vec<Symbol> = self.state.financial_system.exchange.markets.keys().cloned().collect();

        let pre_snapshots: HashMap<Symbol, MarketView> =
            market_symbols.iter().map(|s| (s.clone(), self.state.market_view(s).unwrap_or_default())).collect();
        for symbol in &market_symbols {
            let trades = match self.state.financial_system.exchange.markets.get_mut(symbol) {
                Some(MarketType::Goods(market)) => {
                    let trades = market.clear_and_match(symbol);
                    market.book.bids.clear();
                    market.book.asks.clear();
                    trades
                }
                Some(MarketType::Financial(market)) => market.book.clear_and_match(symbol),
                Some(MarketType::Labour(_)) => vec![],
                None => vec![],
            };
            all_trades.extend(trades);
        }

        (all_trades, pre_snapshots)
    }

    pub fn settle_trades(&self, trades: &[Trade]) -> Vec<StateEffect> {
        trades
            .iter()
            .flat_map(|trade| {
                let dres = self.domain_registry.settle_trade(trade, &self.state);
                if dres.success {
                    dres.effects
                } else {
                    for e in dres.errors {
                        println!("[ERROR] Failed to settle trade: {}", e);
                    }
                    vec![]
                }
            })
            .collect()
    }

    pub fn match_labour_markets(&mut self, rng: &mut dyn RngCore) -> Vec<StateEffect> {
        use rand::seq::SliceRandom;
        let mut effects = Vec::new();
        let beta = 0.5;

        let mut labour_markets = Vec::new();
        for (symbol, market) in self.state.financial_system.exchange.markets.iter_mut() {
            if let MarketType::Labour(labour_market) = market {
                labour_markets.push((symbol.clone(), labour_market));
            }
        }

        for (_symbol, market) in labour_markets.iter_mut() {
            market.job_applications.retain(|app| {
                self.state.agents.consumers.get(&app.consumer_id).map_or(false, |c| c.employed_by.is_none())
            });

            market.job_offers.shuffle(rng);
            market.job_applications.shuffle(rng);

            let mut matched_app_indices = std::collections::HashSet::new();

            for offer in &mut market.job_offers {
                if offer.quantity == 0 {
                    continue;
                }

                for (app_idx, app) in market.job_applications.iter().enumerate() {
                    if matched_app_indices.contains(&app_idx) {
                        continue;
                    }

                    let vj = offer.value_per_hour;
                    let ri = app.reservation_wage;

                    let nash_wage = (1.0 - beta) * ri + beta * vj;

                    if nash_wage >= ri && nash_wage <= vj {
                        let contract = EmploymentContract {
                            employee_id: app.consumer_id,
                            wage_rate: nash_wage,
                            hours: offer.hours_required,
                            start_date: self.state.current_date,
                            next_pay_date: self.state.current_date + chrono::Duration::days(7),
                            pay_interval_days: 7,
                        };

                        effects.push(StateEffect::Agent(AgentEffect::EstablishEmployment {
                            firm_id: offer.firm_id,
                            consumer_id: app.consumer_id,
                            contract,
                        }));

                        offer.quantity -= 1;
                        matched_app_indices.insert(app_idx);

                        if offer.quantity == 0 {
                            break;
                        }
                    }
                }
            }

            let mut i = 0;
            while i < market.job_applications.len() {
                if matched_app_indices.contains(&i) {
                    market.job_applications.swap_remove(i);
                } else {
                    i += 1;
                }
            }
            market.job_offers.retain(|o| o.quantity > 0);
        }
        effects
    }

    pub fn update_market_history(&mut self, trades: &[Trade], views: &HashMap<Symbol, MarketView>) {
        let current_date = self.state.current_date;
        let history = &mut self.state.history;

        for (symbol, snapshot) in views {
            let market_trades: Vec<&Trade> = trades.iter().filter(|t| &t.market_id == symbol).collect();
            let close = market_trades.last().map(|t| t.price.to_f64()).or(snapshot.last);

            let (best_bid, best_ask) = match self.state.financial_system.exchange.markets.get(symbol) {
                Some(MarketType::Financial(m)) => {
                    (m.book.best_bid().map(|p| p.to_f64()), m.book.best_ask().map(|p| p.to_f64()))
                }
                Some(MarketType::Goods(m)) => {
                    (m.book.best_bid().map(|p| p.to_f64()), m.book.best_ask().map(|p| p.to_f64()))
                }
                _ => (None, None),
            };

            let prices = market_trades.iter().map(|t| t.price);
            let qty_today: f64 = market_trades.iter().map(|t| t.quantity).sum();
            let trn_today: f64 = market_trades.iter().map(|t| t.price.to_f64() * t.quantity).sum();

            let tick = MarketTick {
                date: current_date,
                open: market_trades.first().map(|t| t.price.to_f64()),
                high: prices.clone().max().map(|m| m.to_f64()),
                low: prices.min().map(|m| m.to_f64()),
                close,
                volume: qty_today,
                turnover: trn_today,
                last_price: close,
                last_qty: market_trades.last().map(|t| t.quantity),
                best_bid,
                best_ask,
                spread: best_bid.zip(best_ask).map(|(b, a)| (a - b).max(0.0)),
            };

            history.market_ticks.entry(symbol.clone()).or_default().push_back(tick);
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

    pub fn consume_effects<F>(
        &mut self, context: &mut crate::scheduler::StepContext, mut filter: F,
    ) -> Result<usize, String>
    where
        F: FnMut(&StateEffect) -> bool,
    {
        let mut all_effects = context.get_all_effects().unwrap_or_default();

        let mut effects_to_apply = Vec::new();
        let mut remaining_effects = Vec::new();

        for effect in all_effects.drain(..) {
            if filter(&effect) {
                effects_to_apply.push(effect);
            } else {
                remaining_effects.push(effect);
            }
        }

        if effects_to_apply.is_empty() {
            return Ok(0);
        }

        self.state.apply_effects(&effects_to_apply).map_err(|e| e.to_string())?;

        context.store("all_effects", &remaining_effects)?;

        Ok(effects_to_apply.len())
    }
    pub fn refresh_pricing_feeds(&mut self) {
        {
            let fs = &mut self.state.financial_system;

            if let Ok(mut pr) = fs.pricing_feeds.policy_rate_bps.write() {
                *pr = fs.central_bank.policy_rate_bps.to_f64().unwrap_or(69.0);
            }
            if let Ok(mut d) = fs.pricing_feeds.current_date.write() {
                *d = self.state.current_date;
            }

            use std::collections::BTreeMap;
            let mut points: BTreeMap<NotNan<f64>, f64> = BTreeMap::new();
            if let Ok(mut yc) = fs.pricing_feeds.yield_curve.write() {
                yc.date = self.state.current_date;
                if let Some(treasury_ids) = fs.exchange.index.by_bond_type.get(&BondType::Government) {
                    for inst_id in treasury_ids {
                        if let Some(inst) = fs.instruments.instruments.get(inst_id) {
                            if let Some(symbol) = fs.exchange.inst_to_symbol.get(inst_id) {
                                if let Some(market) = fs.exchange.markets.get(symbol) {
                                    if let MarketType::Financial(fin_market) = market {
                                        if let InstrumentRuntime::Bond(b) = &inst.state() {
                                            if let Some(mid) = fin_market.book.mid_price() {
                                                let tenor = b.remaining_tenor_years(self.state.current_date);
                                                let tmp = GovTermStructurePricer::new(
                                                    BondPricingTerms::from(b),
                                                    TermStructureMethod::Bootstrapped,
                                                    fs.pricing_feeds.clone(),
                                                );
                                                if let Some(y) = tmp.yield_from_price(inst_id, mid) {
                                                    points.insert(NotNan::new(tenor.max(0.1)).unwrap(), y);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                yc.points = points;
            }
        }
        let prev_wage = self.state.financial_system.pricing_feeds.goods.read().ok().map(|g| g.avg_wage).unwrap_or(0.0);
        let mut avg_wage = 0.0;
        let mut n_w = 0usize;
        for (_, f) in &self.state.agents.firms {
            if f.wage_rate.is_finite() && f.wage_rate > 0.0 {
                avg_wage += f.wage_rate;
                n_w += 1;
            }
        }
        if n_w > 0 {
            avg_wage /= n_w as f64;
        }

        use std::collections::HashMap;
        let mut unit_cost_sum: HashMap<GoodId, (f64 /*sum qty*/, f64 /*sum qty*cost*/)> = HashMap::new();
        let mut inv_qty: HashMap<GoodId, f64> = HashMap::new();

        for (_id, inst) in &self.state.financial_system.instruments.instruments {
            if let InstrumentRuntime::RealAsset(RealAssetState::Inventory { goods, .. }) = &inst.state() {
                for (gid, item) in goods {
                    let q = item.quantity.max(0.0);
                    let c = item.unit_cost.to_f64();
                    let e = unit_cost_sum.entry(*gid).or_insert((0.0, 0.0));
                    e.0 += q;
                    e.1 += q * c;
                    *inv_qty.entry(*gid).or_insert(0.0) += q;
                }
            }
        }

        let mut sales_per_day: HashMap<GoodId, f64> = HashMap::new();
        let ticks = self.state.history.market_ticks.clone();
        for (symbol, ticks) in &ticks {
            if let Some(gid) = self.state.financial_system.exchange.symbol_to_good.get(symbol) {
                let mut cnt = 0usize;
                let mut vol = 0.0;
                for t in ticks.iter().rev().take(7) {
                    if let Some(v) = Some(t.volume) {
                        vol += v;
                        cnt += 1;
                    }
                }
                let avg = if cnt > 0 { vol / cnt as f64 } else { 0.0 };
                sales_per_day.insert(*gid, avg);
            }
        }

        if let Ok(mut gm) = self.state.financial_system.pricing_feeds.goods.write() {
            gm.last_avg_wage = prev_wage;
            gm.avg_wage = avg_wage;
            gm.per_good.clear();
            for (gid, (sum_q, sum_qc)) in unit_cost_sum {
                let wuc = if sum_q > 1e-9 { sum_qc / sum_q } else { 0.0 };
                let qty = inv_qty.get(&gid).copied().unwrap_or(0.0);
                let sales = sales_per_day.get(&gid).copied().unwrap_or(0.0);
                let base_markup = 0.20;
                gm.per_good.insert(
                    gid,
                    GoodMetric {
                        weighted_unit_cost: wuc,
                        inventory_qty: qty,
                        avg_daily_sales: sales,
                        supply_shock: 1.0,
                        base_markup,
                    },
                );
            }
        }
    }
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
