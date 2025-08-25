use crate::broadcast::SurrealDbWriter;
use crate::*;
use chrono::{Datelike, NaiveDate};
use domains::prelude::*;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct SimulationEngine {
    pub state: SimState,
    pub domain_registry: DomainRegistry,
    pub decision_models: HashMap<AgentId, Box<dyn DecisionModel>>,
    pub db_writer: Option<SurrealDbWriter>,
}

impl SimulationEngine {
    pub fn new(state: SimState) -> Self {
        Self { state, domain_registry: DomainRegistry::new(), decision_models: HashMap::new(), db_writer: None }
    }

    pub async fn connect_to_db(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let writer = SurrealDbWriter::connect().await?;
        self.db_writer = Some(writer);
        Ok(())
    }

    pub fn run_initialization(&mut self) {}

    fn gather_intentions(&self, rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let mut all_intentions = Vec::new();

        for agent_id in self.state.agents.all_agent_ids() {
            if let Some(model) = self.decision_models.get(&agent_id) {
                if let Some(agent) = self.state.agents.get_agent_as_any(&agent_id) {
                    all_intentions.extend(model.decide(agent, &self.state, rng));
                }
            }
        }

        let government = &self.state.financial_system.government;
        if let Some(model) = self.decision_models.get(&government.id) {
            all_intentions.extend(model.decide(government, &self.state, rng));
        }

        all_intentions
    }

    pub fn tick(&mut self, rng: &mut dyn RngCore) -> TickResult {
        self.run_upkeep();

        let intentions = self.gather_intentions(rng);
        let (action_records, action_to_effect_indices, all_effects, trades) =
            self.resolve_and_execute_intentions(intentions);
        self.state.financial_system.update_yield_curve(self.state.current_date);

        let tick_record = TickRecord {
            tick_number: self.state.ticknum,
            date: self.state.current_date,
            actions: action_records.clone(),
            effects: all_effects.clone(),
            action_to_effect_indices: action_to_effect_indices.clone(),
            trades: trades.clone(),
        };

        if let Some(writer) = &self.db_writer {
            let _ = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    if let Err(e) = self
                        .persist_tick_batch(writer, &action_records, &all_effects, &action_to_effect_indices, &trades)
                        .await
                    {
                        println!("[ERROR] Failed to write to SurrealDB: {}", e);
                    }
                })
            });
        }

        self.state.history.add_tick_record(tick_record);
        self.state.ticknum += 1;

        TickResult {
            tick_number: self.state.ticknum,
            actions: action_records.iter().map(|r| r.action.clone()).collect(),
            effects: all_effects,
            trades,
        }
    }

    fn resolve_and_execute_intentions(
        &mut self, intentions: Vec<SimIntention>,
    ) -> (Vec<ActionRecord>, HashMap<usize, Vec<usize>>, Vec<StateEffect>, Vec<Trade>) {
        let categorized = self.domain_registry.categorize_intentions_by_phase(intentions);

        let mut all_action_records = Vec::new();
        let mut all_action_to_effect_indices = HashMap::new();
        let mut all_effects = Vec::new();

        for phase in [ResolutionPhase::Independent, ResolutionPhase::Market, ResolutionPhase::Dependent] {
            if let Some(phase_intentions) = categorized.get(&phase) {
                let (phase_records, phase_indices, phase_effects) = {
                    let context = ResolutionContext { state: &self.state, current_tick: self.state.ticknum };
                    self.resolve_and_execute_phase(
                        phase_intentions,
                        &context,
                        all_action_records.len(),
                        all_effects.len(),
                    )
                };

                if phase == ResolutionPhase::Market {
                    self.apply_market_effects_for_price_discovery(&phase_effects);
                }

                all_action_records.extend(phase_records);
                for (k, v) in phase_indices {
                    all_action_to_effect_indices.insert(k, v);
                }
                all_effects.extend(phase_effects);
            }
        }

        let (trades, snapshots) = self.state.financial_system.exchange.clear_markets(self.state.ticknum as i64);
        self.update_market_history(&trades, &snapshots);

        let settlement_effects = self.settle_trades(trades.clone());
        all_effects.extend(settlement_effects);

        let labour_effects = self.state.financial_system.exchange.clear_labour_markets(&self.state.clone());
        all_effects.extend(labour_effects);

        if let Err(e) = self.state.apply_effects(&all_effects) {
            println!("[ERROR] applying all effects: {}", e);
        }

        (all_action_records, all_action_to_effect_indices, all_effects, trades)
    }

    fn resolve_and_execute_phase(
        &self, intentions: &[SimIntention], context: &ResolutionContext, action_offset: usize, effect_offset: usize,
    ) -> (Vec<ActionRecord>, HashMap<usize, Vec<usize>>, Vec<StateEffect>) {
        let mut phase_actions = Vec::new();

        for intention in intentions {
            let result = self.domain_registry.resolve_intention(intention, context);

            if !result.success {
                println!("[WARNING] Failed to resolve intention: {:?}", result.errors);
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

    fn apply_market_effects_for_price_discovery(&mut self, effects: &[StateEffect]) {
        let market_effects: Vec<StateEffect> =
            effects.iter().filter(|e| matches!(e, StateEffect::Market(_))).cloned().collect();

        if !market_effects.is_empty() {
            if let Err(e) = self.state.apply_effects(&market_effects) {
                println!("[ERROR] applying market effects for price discovery: {}", e);
            }
        }
    }

    fn execute_actions(
        &self, actions: &[SimAction],
    ) -> (Vec<ActionRecord>, HashMap<usize, Vec<usize>>, Vec<StateEffect>) {
        let mut all_effects = Vec::new();
        let mut action_records = Vec::with_capacity(actions.len());
        let mut action_to_effect_indices = HashMap::new();

        for (action_idx, action) in actions.iter().enumerate() {
            let agent_id = action.agent_id();
            let (agent_type, agent_name) = self.get_agent_info(&agent_id);
            action_records.push(ActionRecord { action: action.clone(), agent_id, agent_type, agent_name });

            let effects = match self.domain_registry.execute_action(action, &self.state) {
                Ok(effects) => effects,
                Err(e) => {
                    println!("[FAILED ACTION] - {:?}: {}", action.name(), e);
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

    fn run_upkeep(&mut self) {
        self.state.advance_time();
        self.update_agent_expectations();

        let upkeep_actions = self.process_financial_updates();
        let (_, _, upkeep_effects) = self.execute_actions(&upkeep_actions);

        if let Err(e) = self.state.apply_effects(&upkeep_effects) {
            println!("[ERROR] applying upkeep effects: {}", e);
        }
    }

    fn get_agent_info(&self, agent_id: &AgentId) -> (String, Option<String>) {
        if let Some(bank) = self.state.agents.banks.get(agent_id) {
            ("Bank".to_string(), Some(bank.name.clone()))
        } else if let Some(_consumer) = self.state.agents.consumers.get(agent_id) {
            ("Consumer".to_string(), Some(format!("Consumer {}", &agent_id.to_string()[..8])))
        } else if let Some(firm) = self.state.agents.firms.get(agent_id) {
            ("Firm".to_string(), Some(firm.name.clone()))
        } else if *agent_id == self.state.financial_system.government.id {
            ("Government".to_string(), Some("Government".to_string()))
        } else if *agent_id == self.state.financial_system.central_bank.id {
            ("CentralBank".to_string(), Some("Central Bank".to_string()))
        } else {
            ("Unknown".to_string(), None)
        }
    }

    async fn persist_tick_batch(
        &self, writer: &SurrealDbWriter, action_records: &[ActionRecord], effects: &[StateEffect],
        action_to_effect_indices: &HashMap<usize, Vec<usize>>, trades: &[Trade],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let instrument_snapshots: Vec<(InstrumentId, &FinancialInstrument)> =
            self.state.financial_system.instruments.iter().map(|(id, inst)| (*id, inst)).collect();

        let mut agent_snapshots = Vec::new();
        for (bank_id, _bank) in &self.state.agents.banks {
            if let Some(bs) = self.state.financial_system.get_bs_by_id(bank_id) {
                agent_snapshots.push((*bank_id, "Bank".to_string(), bs));
            }
        }
        for (consumer_id, _consumer) in &self.state.agents.consumers {
            if let Some(bs) = self.state.financial_system.get_bs_by_id(consumer_id) {
                agent_snapshots.push((*consumer_id, "Consumer".to_string(), bs));
            }
        }
        for (firm_id, _firm) in &self.state.agents.firms {
            if let Some(bs) = self.state.financial_system.get_bs_by_id(firm_id) {
                agent_snapshots.push((*firm_id, "Firm".to_string(), bs));
            }
        }
        let gov_id = self.state.financial_system.government.id;
        if let Some(bs) = self.state.financial_system.get_bs_by_id(&gov_id) {
            agent_snapshots.push((gov_id, "Government".to_string(), bs));
        }
        let cb_id = self.state.financial_system.central_bank.id;
        if let Some(bs) = self.state.financial_system.get_bs_by_id(&cb_id) {
            agent_snapshots.push((cb_id, "CentralBank".to_string(), bs));
        }

        writer
            .write_tick_batch(
                self.state.ticknum,
                action_records,
                effects,
                action_to_effect_indices,
                trades,
                &instrument_snapshots,
                &agent_snapshots,
            )
            .await?;

        if self.state.ticknum % 100 == 0 {
            writer.cleanup_old_data(1000).await?;
        }

        Ok(())
    }

    fn update_agent_expectations(&mut self) {
        let alpha = 0.1;
        let state_view = self.state.clone();
        for consumer in self.state.agents.consumers.values_mut() {
            consumer.update_expectations(&state_view, alpha);
        }
    }

    fn update_market_history(&mut self, trades: &[Trade], snapshots: &HashMap<MarketId, MarketSnapshot>) {
        let current_date = self.state.current_date;
        let history = &mut self.state.history;
        let mut trades_by_market: HashMap<MarketId, Vec<&Trade>> = HashMap::new();
        for trade in trades {
            trades_by_market.entry(trade.market_id.clone()).or_default().push(trade);
        }

        for (market_id, market_trades) in trades_by_market {
            let mut volume = 0.0;
            let mut turnover = 0.0;
            let mut high = f64::MIN;
            let mut low = f64::MAX;
            let open = market_trades.first().unwrap().price;
            let close = market_trades.last().unwrap().price;
            for trade in &market_trades {
                volume += trade.quantity;
                turnover += trade.quantity * trade.price;
                high = high.max(trade.price);
                low = low.min(trade.price);
            }
            let snapshot = snapshots.get(&market_id);
            let (best_bid, best_ask, spread) =
                snapshot.map_or((None, None, None), |s| (s.best_bid, s.best_ask, s.spread));
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

        for (market_id, snapshot) in snapshots {
            if !history.market_ticks.contains_key(market_id)
                || history.market_ticks.get(market_id).unwrap().back().map_or(true, |t| t.date != current_date)
            {
                let previous_close =
                    history.market_ticks.get(market_id).and_then(|ticks| ticks.back()).and_then(|tick| tick.close);
                let tick = MarketTick {
                    date: current_date,
                    last_price: None,
                    last_qty: None,
                    best_bid: snapshot.best_bid,
                    best_ask: snapshot.best_ask,
                    spread: snapshot.spread,
                    volume: 0.0,
                    turnover: 0.0,
                    open: previous_close,
                    high: previous_close,
                    low: previous_close,
                    close: previous_close,
                };
                history.market_ticks.entry(market_id.clone()).or_default().push_back(tick);
            }
        }
    }

    fn settle_trades(&self, trades: Vec<Trade>) -> Vec<StateEffect> {
        let mut all_effects = Vec::new();
        for trade in trades {
            println!("[INFO] Settling trade: {:?}", trade);
            let settlement_effects = self.create_settlement_effects(&trade);
            all_effects.extend(settlement_effects);
            all_effects.push(StateEffect::Market(MarketEffect::ExecuteTrade(trade.clone())));
        }
        all_effects
    }

    fn create_settlement_effects(&self, trade: &Trade) -> Vec<StateEffect> {
        let mut effects = vec![];
        match &trade.market_id {
            MarketId::Goods(good_id) => {
                let total_payment = trade.price * trade.quantity;
                effects.extend(self.create_payment_transfer_effects(trade.buyer, trade.seller, total_payment));
                effects.push(StateEffect::Inventory(InventoryEffect::RemoveInventory {
                    owner: trade.seller,
                    good_id: *good_id,
                    quantity: trade.quantity,
                }));
                effects.push(StateEffect::Inventory(InventoryEffect::AddInventory {
                    owner: trade.buyer,
                    good_id: *good_id,
                    quantity: trade.quantity,
                    unit_cost: trade.price,
                }));
            }
            MarketId::Financial(FinancialMarketId::Treasury { tenor }) => {
                if trade.seller == self.state.financial_system.government.id {
                    let total_cost = trade.price * trade.quantity;
                    effects.extend(self.create_payment_transfer_effects(trade.buyer, trade.seller, total_cost));
                    const FACE_VALUE: f64 = 1000.0;
                    let coupon_rate = self.state.financial_system.central_bank.policy_rate_bps;
                    let maturity_date = tenor.add_to_date(self.state.current_date);

                    let principal = total_cost;

                    let mut new_bond = bond!(
                        trade.buyer,
                        trade.seller,
                        principal,
                        coupon_rate,
                        maturity_date,
                        FACE_VALUE,
                        BondType::Government,
                        2,
                        *tenor,
                        self.state.current_date
                    );

                    if let Some(details) = new_bond.details.as_any_mut().downcast_mut::<BondDetails>() {
                        details.quantity = trade.quantity as u64;
                    }

                    effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(new_bond)));
                } else {
                    if let Some(seller_bs) = self.state.financial_system.get_bs_by_id(&trade.seller) {
                        for (inst_id, inst) in &seller_bs.assets {
                            if let Some(bond_details) = inst.details.as_any().downcast_ref::<BondDetails>() {
                                if bond_details.bond_type == BondType::Government
                                    && bond_details.tenor == *tenor
                                    && bond_details.quantity >= trade.quantity as u64
                                {
                                    effects.push(StateEffect::Financial(FinancialEffect::SplitAndTransferInstrument {
                                        id: *inst_id,
                                        buyer: trade.buyer,
                                        quantity: trade.quantity as u64,
                                    }));
                                    let total_payment = trade.price * trade.quantity;
                                    effects.extend(self.create_payment_transfer_effects(
                                        trade.buyer,
                                        trade.seller,
                                        total_payment,
                                    ));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            MarketId::Financial(FinancialMarketId::FederalFundsOvernight) => {
                let loan_amount = trade.quantity;
                let overnight_rate_bps = self.state.financial_system.central_bank.policy_rate_bps;
                effects.extend(self.create_reserves_transfer_effects(trade.seller, trade.buyer, loan_amount));
                let fed_funds_loan = FinancialInstrument {
                    id: InstrumentId(uuid::Uuid::new_v4()),
                    creditor: trade.seller,
                    debtor: trade.buyer,
                    principal: loan_amount,
                    details: Box::new(LoanDetails {
                        loan_type: LoanType::FederalFunds,
                        interest_rate_bps: overnight_rate_bps,
                        maturity_date: self.state.current_date + chrono::Duration::days(1),
                        collateral: None,
                    }),
                    originated_date: self.state.current_date,
                    accrued_interest: 0.0,
                    last_accrual_date: self.state.current_date,
                };
                effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(fed_funds_loan)));
                println!(
                    "[INFO] Federal funds trade executed: ${:.2} from {} to {}",
                    loan_amount, trade.seller, trade.buyer
                );
            }
            MarketId::Financial(FinancialMarketId::TreasuryRepoOvernight) => {
                let repo_amount = trade.quantity;
                let repo_rate_bps = self.state.financial_system.central_bank.policy_rate_bps - 10.0;
                effects.extend(self.create_payment_transfer_effects(trade.seller, trade.buyer, repo_amount));
                let repo_agreement = FinancialInstrument {
                    id: InstrumentId(uuid::Uuid::new_v4()),
                    creditor: trade.seller,
                    debtor: trade.buyer,
                    principal: repo_amount,
                    details: Box::new(LoanDetails {
                        loan_type: LoanType::Repo,
                        interest_rate_bps: repo_rate_bps,
                        maturity_date: self.state.current_date + chrono::Duration::days(1),
                        collateral: Some(CollateralInfo {
                            collateral_type: "US Treasury".to_string(),
                            value: repo_amount * 1.02,
                        }),
                    }),
                    originated_date: self.state.current_date,
                    accrued_interest: 0.0,
                    last_accrual_date: self.state.current_date,
                };
                effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(repo_agreement)));
                println!(
                    "[INFO] Treasury repo trade executed: ${:.2} from {} to {}",
                    repo_amount, trade.seller, trade.buyer
                );
            }
            MarketId::Financial(FinancialMarketId::DiscountWindow)
            | MarketId::Financial(FinancialMarketId::StandingRepoFacility)
            | MarketId::Financial(FinancialMarketId::OvernightReverseRepo) => {
                println!("[INFO] Central bank facility trade executed (settlement logic TBD): {:?}", trade.market_id);
            }
            _ => {}
        }
        effects
    }

    fn create_reserves_transfer_effects(&self, from: AgentId, to: AgentId, amount: f64) -> Vec<StateEffect> {
        let mut effects = vec![];
        let cb_id = self.state.financial_system.central_bank.id;
        if let Some(from_bs) = self.state.financial_system.get_bs_by_id(&from) {
            if let Some((reserves_id, reserves_inst)) =
                from_bs.assets.iter().find(|(_, inst)| inst.details.as_any().is::<CentralBankReservesDetails>())
            {
                let new_reserves = reserves_inst.principal - amount;
                if new_reserves < 1e-6 {
                    effects.push(StateEffect::Financial(FinancialEffect::RemoveInstrument(*reserves_id)));
                } else {
                    effects.push(StateEffect::Financial(FinancialEffect::UpdateInstrument {
                        id: *reserves_id,
                        new_principal: new_reserves,
                    }));
                }
            }
        }
        effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(reserves!(
            to,
            cb_id,
            amount,
            self.state.current_date,
            self.state.financial_system.central_bank.policy_rate_bps + 15.0
        ))));
        effects
    }

    fn create_payment_transfer_effects(&self, from: AgentId, to: AgentId, amount: f64) -> Vec<StateEffect> {
        let mut effects = vec![];
        let cb_id = self.state.financial_system.central_bank.id;
        let from_bs = match self.state.financial_system.get_bs_by_id(&from) {
            Some(bs) => bs,
            None => return vec![],
        };
        let (cash_id, cash_on_hand) = from_bs
            .assets
            .iter()
            .find(|(_, inst)| inst.details.as_any().is::<CashDetails>())
            .map(|(id, inst)| (Some(*id), inst.principal))
            .unwrap_or((None, 0.0));

        let amount_from_cash = cash_on_hand.min(amount);
        let amount_remaining_for_deposit = amount - amount_from_cash;

        if amount_from_cash > 1e-6 {
            if let Some(id) = cash_id {
                let new_principal = cash_on_hand - amount_from_cash;
                if new_principal < 1e-6 {
                    effects.push(StateEffect::Financial(FinancialEffect::RemoveInstrument(id)));
                } else {
                    effects.push(StateEffect::Financial(FinancialEffect::UpdateInstrument { id, new_principal }));
                }
                if self.state.agents.banks.contains_key(&to) {
                    effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(reserves!(
                        to,
                        cb_id,
                        amount_from_cash,
                        self.state.current_date,
                        self.state.financial_system.central_bank.policy_rate_bps + 15.0
                    ))));
                } else {
                    effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(cash!(
                        to,
                        amount_from_cash,
                        cb_id,
                        self.state.current_date
                    ))));
                }
            }
        }
        if amount_remaining_for_deposit > 1e-6 {
            if let Some((dep_id, dep_inst)) =
                from_bs.assets.iter().find(|(_, inst)| inst.details.as_any().is::<DemandDepositDetails>())
            {
                let payer_bank_id = dep_inst.debtor;
                let new_deposit_principal = dep_inst.principal - amount_remaining_for_deposit;
                if new_deposit_principal < 1e-6 {
                    effects.push(StateEffect::Financial(FinancialEffect::RemoveInstrument(*dep_id)));
                } else {
                    effects.push(StateEffect::Financial(FinancialEffect::UpdateInstrument {
                        id: *dep_id,
                        new_principal: new_deposit_principal,
                    }));
                }
                if let Some((res_id, res_inst)) =
                    self.state.financial_system.get_bs_by_id(&payer_bank_id).and_then(|bs| {
                        bs.assets.iter().find(|(_, i)| i.details.as_any().is::<CentralBankReservesDetails>())
                    })
                {
                    let new_reserves = res_inst.principal - amount_remaining_for_deposit;
                    if new_reserves < 1e-6 {
                        effects.push(StateEffect::Financial(FinancialEffect::RemoveInstrument(*res_id)));
                    } else {
                        effects.push(StateEffect::Financial(FinancialEffect::UpdateInstrument {
                            id: *res_id,
                            new_principal: new_reserves,
                        }));
                    }
                }
                let payee_bank_id = if self.state.agents.banks.contains_key(&to) {
                    to
                } else if let Some(c) = self.state.agents.get_consumer(&to) {
                    c.bank_id
                } else if let Some(f) = self.state.agents.get_firm(&to) {
                    f.bank_id
                } else {
                    AgentId::default()
                };

                if payee_bank_id != AgentId::default() {
                    effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(reserves!(
                        payee_bank_id,
                        cb_id,
                        amount_remaining_for_deposit,
                        self.state.current_date,
                        self.state.financial_system.central_bank.policy_rate_bps + 15.0
                    ))));

                    if !self.state.agents.banks.contains_key(&to) && payee_bank_id != AgentId::default() {
                        let bank_spread_bps =
                            self.state.agents.get_bank(&payee_bank_id).map(|b| b.deposit_spread_bps).unwrap_or(-50.0);
                        let policy_rate_bps = self.state.financial_system.central_bank.policy_rate_bps;
                        let dep_rate_bps = (policy_rate_bps + bank_spread_bps).max(0.0);
                        effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(deposit!(
                            to,
                            payee_bank_id,
                            amount_remaining_for_deposit,
                            dep_rate_bps,
                            self.state.current_date
                        ))));
                    }
                } else {
                    effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(cash!(
                        to,
                        amount_remaining_for_deposit,
                        cb_id,
                        self.state.current_date
                    ))));
                }
            }
        }
        effects
    }

    fn process_financial_updates(&self) -> Vec<SimAction> {
        let mut actions = Vec::new();
        let current_date = self.state.current_date;
        for (instrument_id, instrument) in &self.state.financial_system.instruments {
            if self.is_interest_bearing(instrument) {
                actions.push(SimAction::Settlement(SettlementAction::AccrueInterest { instrument_id: *instrument_id }));
            }
            if self.is_interest_payment_date(current_date) && instrument.accrued_interest > 0.0 {
                actions.push(SimAction::Settlement(SettlementAction::PayInterest { instrument_id: *instrument_id }));
            }
            if let Some(bond_details) = instrument.details.as_any().downcast_ref::<BondDetails>() {
                if self.is_coupon_payment_date(current_date, instrument, bond_details) {
                    actions.push(SimAction::Settlement(SettlementAction::ProcessCouponPayment {
                        instrument_id: *instrument_id,
                    }));
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

    fn is_coupon_payment_date(
        &self, date: NaiveDate, instrument: &FinancialInstrument, bond_details: &BondDetails,
    ) -> bool {
        let months_between_payments = (12 / bond_details.frequency) as u32;
        let months_since_origination = (date.year() - instrument.originated_date.year()) * 12
            + (date.month() as i32 - instrument.originated_date.month() as i32);
        instrument.originated_date.day() == date.day()
            && months_since_origination > 0
            && months_since_origination as u32 % months_between_payments == 0
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TickResult {
    pub tick_number: u32,
    pub actions: Vec<SimAction>,
    pub effects: Vec<StateEffect>,
    pub trades: Vec<Trade>,
}
