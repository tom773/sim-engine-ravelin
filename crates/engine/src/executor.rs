use crate::*;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::collections::HashMap;
use chrono::{Datelike, NaiveDate};
pub struct SimulationEngine {
    pub state: SimState,
    pub domain_registry: DomainRegistry,
    pub decision_models: HashMap<AgentId, Box<dyn DecisionModel>>,
}

impl SimulationEngine {
    pub fn new(state: SimState) -> Self {
        Self { state, domain_registry: DomainRegistry::new(), decision_models: HashMap::new() }
    }

    pub fn run_initialization(&mut self) {
        for agent_id in self.state.agents.banks.keys() {
            if let Some(bs) = self.state.financial_system.get_bs_by_id(agent_id) {
                println!("[INITIALIZATION] Bank {:?} has balance sheet: {:#?}", agent_id, bs);
            }
        }
    }

    fn collect_actions(&self, rng: &mut dyn RngCore) -> Vec<SimAction> {
        let mut all_actions = Vec::new();

        for agent_id in self.state.agents.all_agent_ids() {
            if let Some(model) = self.decision_models.get(&agent_id) {
                if let Some(agent) = self.state.agents.get_agent_as_any(&agent_id) {
                    all_actions.extend(model.decide(agent, &self.state, rng));
                }
            }
        }

        let government = &self.state.financial_system.government;
        if let Some(model) = self.decision_models.get(&government.id) {
            all_actions.extend(model.decide(government, &self.state, rng));
        }
        all_actions
    }

    pub fn tick(&mut self, rng: &mut dyn RngCore) -> TickResult {
        // New: Update agent expectations at the start of the tick (Point 4)
        self.update_agent_expectations();

        let mut actions = self.process_financial_updates();
        actions.extend(self.collect_actions(rng));

        // Execute actions (this includes posting bids from PurchaseAtBest)
        let effects = self.execute_actions(&actions);
        if let Err(e) = self.state.apply_effects(&effects) {
            println!("[ERROR] applying action effects: {}", e);
        }

        // Clear Markets
        // Modified: clear_markets now returns trades and snapshots (Point 1)
        let (trades, snapshots) = self.state.financial_system.exchange.clear_markets();

        // New: Process trades and snapshots into MarketTicks and update history (Point 1)
        self.update_market_history(&trades, &snapshots);

        // Settle the resulting trades
        let settlement_effects = self.settle_trades(&trades);
        if let Err(e) = self.state.apply_effects(&settlement_effects) {
            println!("[ERROR] applying settlement effects: {}", e);
        }

        self.state.advance_time();

        TickResult { tick_number: self.state.ticknum, actions, effects, trades }
    }

    // New method: update_agent_expectations (Point 4)
    fn update_agent_expectations(&mut self) {
        // Define the learning rate (alpha) for adaptive expectations.
        let alpha = 0.1; 
        
        // We need a temporary copy of the state for the view functions, 
        // as we are already mutably borrowing self.state.agents.
        // Note: For large simulations, cloning the state might be inefficient.
        // A better pattern might involve calculating views first, storing them, and then iterating mutably.
        let state_view = self.state.clone();
        
        for consumer in self.state.agents.consumers.values_mut() {
            consumer.update_expectations(&state_view, alpha);
        }
    }

    // New method: update_market_history (Point 1 Implementation)
    fn update_market_history(&mut self, trades: &[Trade], snapshots: &HashMap<MarketId, MarketSnapshot>) {
        let current_date = self.state.current_date;
        let history = &mut self.state.history;

        // 1. Group trades by market
        let mut trades_by_market: HashMap<MarketId, Vec<&Trade>> = HashMap::new();
        for trade in trades {
            trades_by_market.entry(trade.market_id.clone()).or_default().push(trade);
        }

        // 2. Process markets with trades (Calculate OHLC, Volume, Turnover)
        for (market_id, market_trades) in trades_by_market {
            let mut volume = 0.0;
            let mut turnover = 0.0;
            let mut high = f64::MIN;
            let mut low = f64::MAX;
            // Trades are time-ordered within the tick
            let open = market_trades.first().unwrap().price;
            let close = market_trades.last().unwrap().price;

            for trade in &market_trades {
                volume += trade.quantity;
                turnover += trade.quantity * trade.price;
                high = high.max(trade.price);
                low = low.min(trade.price);
            }

            // Use the snapshot taken BEFORE matching for the prevailing bid/ask/spread at the time of clearing
            let snapshot = snapshots.get(&market_id);
            let (best_bid, best_ask, spread) = snapshot.map_or((None, None, None), |s| (s.best_bid, s.best_ask, s.spread));

            let tick = MarketTick {
                date: current_date,
                last_price: Some(close),
                last_qty: market_trades.last().map(|t| t.quantity),
                best_bid,
                best_ask,
                spread,
                volume,
                turnover,
                open: Some(open),
                high: Some(high),
                low: Some(low),
                close: Some(close),
            };

            history.market_ticks.entry(market_id).or_default().push_back(tick);
        }

        // 3. Process markets without trades (using snapshots only)
        for (market_id, snapshot) in snapshots {
            // Check if we already processed this market (because it had trades)
            // We check if the latest tick for this market is from today.
            if !history.market_ticks.contains_key(market_id) || history.market_ticks.get(market_id).unwrap().back().map_or(true, |t| t.date != current_date) {
                 // Get the previous close price if available to carry forward OHLC
                let previous_close = history.market_ticks.get(market_id)
                                         .and_then(|ticks| ticks.back())
                                         .and_then(|tick| tick.close);

                let tick = MarketTick {
                    date: current_date,
                    last_price: None,
                    last_qty: None,
                    best_bid: snapshot.best_bid,
                    best_ask: snapshot.best_ask,
                    spread: snapshot.spread,
                    volume: 0.0,
                    turnover: 0.0,
                    // If no trades, OHLC are the previous close (if available)
                    open: previous_close,
                    high: previous_close,
                    low: previous_close,
                    close: previous_close,
                };
                history.market_ticks.entry(market_id.clone()).or_default().push_back(tick);
            }
        }
    }


    fn execute_actions(&self, actions: &[SimAction]) -> Vec<StateEffect> {
        let mut all_effects = Vec::new();
        for action in actions {
            let effects = self.domain_registry.execute(action, &self.state);
            all_effects.extend(effects);
        }
        all_effects
    }

    // Updated to use the general settle_trade method
    fn settle_trades(&self, trades: &[Trade]) -> Vec<StateEffect> {
        let mut all_effects = Vec::new();
        for trade in trades {
            // Use the TradingDomain for settlement (handles goods and financial)
            let result = self.domain_registry.settle_trade(trade, &self.state);

            if result.success {
                all_effects.extend(result.effects);
                // Point 7: Record trades in history explicitly during settlement
                all_effects.push(StateEffect::Market(MarketEffect::ExecuteTrade(trade.clone())));
            } else {
                println!("[Executor] Trade settlement failed: {:?}", result.errors);
            }
        }
        all_effects
    }
    fn process_financial_updates(&self) -> Vec<SimAction> {
        let mut actions = Vec::new();
        let current_date = self.state.current_date;

        for (instrument_id, instrument) in &self.state.financial_system.instruments {
            if self.is_interest_bearing(instrument) {
                actions.push(SimAction::Settlement(SettlementAction::AccrueInterest {
                    instrument_id: *instrument_id,
                }));
            }

            if self.is_interest_payment_date(current_date) && instrument.accrued_interest > 0.0 {
                actions.push(SimAction::Settlement(SettlementAction::PayInterest {
                    instrument_id: *instrument_id,
                }));
            }
            
            if let Some(bond_details) = instrument.details.as_any().downcast_ref::<BondDetails>() {
                if self.is_coupon_payment_date(current_date, instrument, bond_details) {
                     actions.push(SimAction::Settlement(SettlementAction::ProcessCouponPayment { instrument_id: *instrument_id }));
                }
            }
        }

        actions
    }

    fn is_interest_bearing(&self, instrument: &FinancialInstrument) -> bool {
        instrument.details.as_any().is::<DemandDepositDetails>()
            || instrument.details.as_any().is::<SavingsDepositDetails>()
            || instrument.details.as_any().is::<BondDetails>()
    }

    fn is_interest_payment_date(&self, date: NaiveDate) -> bool {
        let next_day = date + chrono::Duration::days(1);
        date.month() != next_day.month()
    }
    
    fn is_coupon_payment_date(&self, date: NaiveDate, instrument: &FinancialInstrument, bond_details: &BondDetails) -> bool {
        let months_between_payments = (12 / bond_details.frequency) as u32;
        let months_since_origination = (date.year() - instrument.originated_date.year()) * 12 + (date.month() as i32 - instrument.originated_date.month() as i32);
        
        instrument.originated_date.day() == date.day() &&
        months_since_origination > 0 &&
        months_since_origination as u32 % months_between_payments == 0
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TickResult {
    pub tick_number: u32,
    pub actions: Vec<SimAction>,
    pub effects: Vec<StateEffect>,
    pub trades: Vec<Trade>,
}