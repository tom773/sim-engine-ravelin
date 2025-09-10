use crate::registry::DomainRegistry;
use crate::scheduler::*;
use chrono::{Datelike, NaiveDate};
use domains::prelude::*;
use ordered_float::NotNan;
use rand::RngCore;
use rand::prelude::ThreadRng;
use rust_decimal::prelude::*;
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
        scheduler
            .register_handler(TickStep::ResolveMarketPhase, PhaseResolutionHandler { phase: ResolutionPhase::Market });
        scheduler.register_handler(TickStep::ApplyMarketEffectsForPriceDiscovery, ApplyMarketEffectsHandler);
        scheduler.register_handler(
            TickStep::ResolveDependentPhase,
            PhaseResolutionHandler { phase: ResolutionPhase::Dependent },
        );
        scheduler.register_handler(TickStep::Auction, DebtAuctionsHandler);
        scheduler.register_handler(TickStep::ClearMarkets, ClearMarketsHandler);
        scheduler.register_handler(TickStep::SettleTrades, SettleTradesHandler);
        scheduler.register_handler(TickStep::ServiceCredit, CreditServicingHandler); // NEW
        scheduler.register_handler(TickStep::ApplyPaymentQueuing, ApplyPaymentQueuingHandler);
        scheduler.register_handler(TickStep::RunRTGS, RunRTGSHandler);
        scheduler.register_handler(TickStep::ReconcileCredit, CreditReconciliationHandler); // NEW
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
        if execution_result.success {
            self.state.ticknum += 1;
        }

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
        use rand::seq::SliceRandom;
        let mut effects = Vec::new();

        for (_id, market) in self.state.financial_system.exchange.labour_markets.iter_mut() {
            market.job_applications.retain(|app| {
                self.state.agents.consumers.get(&app.consumer_id).map_or(false, |c| c.employed_by.is_none())
            });

            market.job_offers.shuffle(rng);

            for offer in &mut market.job_offers {
                while offer.quantity > 0 {
                    let idx = market.job_applications.iter().position(|app| {
                        app.reservation_wage <= offer.wage_rate
                            && self
                                .state
                                .agents
                                .consumers
                                .get(&app.consumer_id)
                                .map_or(false, |c| c.employed_by.is_none())
                    });
                    let Some(app_idx) = idx else { break };

                    let app = market.job_applications.swap_remove(app_idx);
                    let wage = match market.wage_rule {
                        WageRule::Posted => offer.wage_rate,
                        WageRule::Nash { beta } => (1.0 - beta) * offer.wage_rate + beta * app.reservation_wage,
                    }
                    .max(app.reservation_wage);
                    let contract = EmploymentContract {
                        employee_id: app.consumer_id,
                        wage_rate: wage,
                        hours: offer.hours_required.min(offer.hours_required),
                        start_date: self.state.current_date,
                        next_pay_date: self.state.current_date + chrono::Duration::days(14),
                        pay_interval_days: 14,
                    };

                    effects.push(StateEffect::Agent(AgentEffect::EstablishEmployment {
                        firm_id: offer.firm_id,
                        consumer_id: app.consumer_id,
                        contract,
                    }));

                    offer.quantity -= 1;
                }
            }
            market.job_offers.retain(|o| o.quantity > 0);
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
                    if let (Some(inst), Some(market)) = (fs.instruments.get(inst_id), fs.exchange.markets.get(inst_id))
                    {
                        if let InstrumentType::Debt(DebtInstrument::Bond(b)) = &inst.instrument_type {
                            if let Some(mid) = market.book.mid_price() {
                                let tenor = b.remaining_tenor_years(self.state.current_date);
                                let spec = BondSpec {
                                    face: b.face_value,
                                    coupon_bps: b.coupon_rate_bps,
                                    freq_per_year: b.frequency,
                                    issue: b.issue_date,
                                    maturity: b.maturity_date,
                                };
                                let tmp = GovTermStructurePricer::new(
                                    spec,
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
            yc.points = points;
        }

        let prev_wage = fs.pricing_feeds.goods.read().ok().map(|g| g.avg_wage).unwrap_or(0.0);
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

        for (_id, inst) in &fs.instruments {
            if let InstrumentType::RealAsset(RealAssetType::Inventory { goods, .. }) = &inst.instrument_type {
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
        for (mid, ticks) in &self.state.history.market_ticks {
            if let MarketId::Goods(gid) = mid {
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

        if let Ok(mut gm) = fs.pricing_feeds.goods.write() {
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

fn _is_last_day_of_month(date: NaiveDate) -> bool {
    date.month() != (date + chrono::Duration::days(1)).month()
}
fn _is_coupon_date(date: NaiveDate, bond: &BondDetails) -> bool {
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
