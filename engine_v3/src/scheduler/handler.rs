use super::{StepContext, StepHandler, StepResult, StepTelemetry};
use crate::executor::SimulationEngine;
use domains::{ResolutionContext, ResolutionPhase};
use rand::prelude::*;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use sim_core::types::core_utils::time::is_coupon_date;
use sim_core::types::markets::BondPricingTerms;
use sim_core::*;
use std::collections::HashMap as StdHashMap;
use std::time::Instant;
use tracing::instrument;
use uuid::Uuid;

fn execute_step<F>(step_fn: F) -> StepResult
where
    F: FnOnce() -> Result<StepTelemetry, String>,
{
    let start = Instant::now();
    match step_fn() {
        Ok(telemetry) => StepResult::success(start.elapsed().as_millis() as u64, telemetry),
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
            //if let Err(e) = engine.state.financial_system.validate_accounting_identity() {
            //    return Err(format!("Accounting validation failed on tick {}: {}", engine.state.ticknum, e));
            //}
            let mut telemetry = StepTelemetry::new();
            telemetry.push_metric("current_date", engine.state.current_date.to_string());
            telemetry.push_metric("tick_number", engine.state.ticknum);
            telemetry.push_metric("session", format!("{:?}", engine.state.current_session));

            Ok(telemetry)
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

            context.set_intentions(intentions);
            context.set_categorized_intentions(categorized);

            let mut concatted = String::new();
            let total_intentions = if let Some(stored) = context.intentions() {
                for intention in stored {
                    concatted.push_str(&format!("{}; ", intention.name()));
                }
                stored.len()
            } else {
                0
            };

            Ok(StepTelemetry::single("total_intentions", total_intentions))
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
            let intentions = match context.categorized_intentions().and_then(|categorized| categorized.get(&self.phase))
            {
                Some(list) if !list.is_empty() => list,
                _ => return Ok(StepTelemetry::single("actions", 0usize).with_metric("effects", 0usize)),
            };

            let resolution_context = ResolutionContext { state: &engine.state, current_tick: engine.state.ticknum };
            let action_offset = context.actions_len();
            let effect_offset = context.effects_len();

            let (action_records, action_to_effect_indices, effects) =
                engine.resolve_and_execute_phase(intentions, &resolution_context, action_offset, effect_offset);

            let action_count = action_records.len();
            context.actions_mut().extend(action_records.into_iter());

            let effect_count = effects.len();
            context.effects_mut().extend(effects.into_iter());

            context.action_to_effect_indices_mut().extend(action_to_effect_indices);

            Ok(StepTelemetry::single("actions", action_count).with_metric("effects", effect_count))
        })
    }
}
#[derive(Debug)]
pub struct ApplyMarketEffectsHandler;
impl StepHandler for ApplyMarketEffectsHandler {
    #[instrument(skip(self, engine, context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let market_effects: Vec<StateEffect> =
                context.effects().iter().filter(|e| matches!(e, StateEffect::Market(_))).cloned().collect();

            engine.state.apply_effects(&market_effects).map_err(|e| e.to_string())?;

            let staged = std::mem::take(&mut engine.state.financial_system.exchange.recent_trades);
            if !staged.is_empty() {
                context.trades_mut().extend(staged);
            }
            Ok(StepTelemetry::single("market_effects_applied", market_effects.len()))
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
            if !market_trades.is_empty() {
                let now = std::time::SystemTime::now();
                let tape = &mut engine.state.financial_system.exchange.tape;
                for t in &market_trades {
                    tape.entry(t.market_id.clone()).or_default().push(TimedTrade { ts: now, trade: t.clone() });
                }
            }
            let trades_generated = market_trades.len();
            context.trades_mut().extend(market_trades.into_iter());

            context.set_market_snapshots(snapshots);
            Ok(StepTelemetry::single("trades_generated", trades_generated))
        })
    }
}
#[derive(Debug)]
pub struct GovCouponsHandler;

impl StepHandler for GovCouponsHandler {
    fn execute(&self, engine: &mut SimulationEngine, _ctx: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let mut effects: Vec<StateEffect> = Vec::new();
            let state = &engine.state;
            let fs = &state.financial_system;
            let current_date = state.current_date;
            let gov_id = fs.government.id;

            let gov_bonds: Vec<(InstrumentId, BondState)> = fs
                .instruments
                .instruments
                .iter()
                .filter_map(|(id, inst)| {
                    if let InstrumentRuntime::Bond(bond) = inst.state() {
                        if bond.bond_type() == BondType::Government {
                            return Some((*id, bond.clone()));
                        }
                    }
                    None
                })
                .collect();

            for (inst_id, bond) in gov_bonds {
                let is_coupon = is_coupon_date(current_date, bond.clone());
                let is_maturity = current_date == bond.maturity_date;

                if !is_coupon && !is_maturity {
                    continue;
                }

                for (agent_id, account) in &fs.clearing_house.csd.custody_accounts {
                    if let Some(holding) = account.holdings.get(&inst_id) {
                        let quantity = holding.total_position();

                        let payee_bank = match fs.find_agent_liquid_account(agent_id) {
                            Some((_, bank_id)) => bank_id,
                            None => {
                                tracing::warn!(
                                    "Could not find liquid account for bond holder {}, skipping payment.",
                                    agent_id
                                );
                                continue;
                            }
                        };

                        if is_coupon {
                            let coupon_rate = bps_to_decimal(bond.archetype.coupon_rate_bps);
                            let frequency = bond.archetype.frequency_per_year.max(1) as f64;
                            let payment_per_bond = (bond.archetype.face_value * coupon_rate) / frequency;
                            let total_payment = (quantity * payment_per_bond).to_f64();

                            effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(PaymentInstruction {
                                id: Uuid::new_v4(),
                                from_bank: fs.central_bank.id,
                                to_bank: payee_bank,
                                payer: gov_id,
                                payee: *agent_id,
                                amount: total_payment,
                                context: TransactionContext::CouponPayment { instrument_id: inst_id },
                                priority: PaymentPriority::Normal,
                                earliest_release_tick: state.ticknum,
                                deadline_tick: state.ticknum + 10,
                            })));
                        }

                        if is_maturity {
                            let total_principal = (quantity * bond.archetype.face_value).to_f64();
                            effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(PaymentInstruction {
                                id: Uuid::new_v4(),
                                from_bank: fs.central_bank.id,
                                to_bank: payee_bank,
                                payer: gov_id,
                                payee: *agent_id,
                                amount: total_principal,
                                context: TransactionContext::PrincipalRepayment { instrument_id: inst_id },
                                priority: PaymentPriority::Urgent,
                                earliest_release_tick: state.ticknum,
                                deadline_tick: state.ticknum + 10,
                            })));
                        }
                    }
                }
            }

            let payment_count = effects.len();
            if payment_count > 0 {
                engine.state.apply_effects(&effects).map_err(|e| e.to_string())?;
            }
            Ok(StepTelemetry::single("payments_generated", payment_count))
        })
    }
}

#[derive(Debug)]
pub struct SettleTradesHandler;

impl StepHandler for SettleTradesHandler {
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let settlement_effects = {
                let trades = context.trades();
                if trades.is_empty() {
                    return Ok(
                        StepTelemetry::single("trades_processed", 0usize).with_metric("settlement_effects", 0usize)
                    );
                }
                engine.settle_trades(trades)
            };

            let effect_count = settlement_effects.len();
            context.effects_mut().extend(settlement_effects.into_iter());

            Ok(StepTelemetry::single("trades_processed", context.trades().len())
                .with_metric("settlement_effects", effect_count))
        })
    }
}
#[derive(Debug)]
pub struct CreditReconciliationHandler;

impl StepHandler for CreditReconciliationHandler {
    fn execute(
        &self, engine: &mut SimulationEngine, _ctx: &mut StepContext, _rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        execute_step(|| {
            let state = &mut engine.state;
            let fs = &mut state.financial_system;
            let settled: std::collections::HashSet<Uuid> = fs.rtgs.settled.iter().map(|p| p.id).collect();
            let rejected: std::collections::HashSet<Uuid> = fs.rtgs.rejected.iter().map(|(pi, _)| pi.id).collect();

            let mut effects: Vec<StateEffect> = Vec::new();
            let mut loans_to_enforce: Vec<Uuid> = Vec::new();

            for (loan_id, sched) in fs.credit_registry.payment_schedules.iter_mut() {
                for sp in sched.scheduled_payments.iter_mut() {
                    if sp.status != PaymentStatus::Scheduled {
                        continue;
                    }

                    let both_present = sp.interest_payment_id.is_some() && sp.principal_payment_id.is_some();
                    if !both_present {
                        continue;
                    }

                    let i_id = sp.interest_payment_id.unwrap();
                    let p_id = sp.principal_payment_id.unwrap();

                    let paid_interest = settled.contains(&i_id);
                    let paid_principal = settled.contains(&p_id);
                    let missed = rejected.contains(&i_id) || rejected.contains(&p_id);

                    if paid_interest || paid_principal {
                        let principal_paid = if paid_principal { sp.principal_amount } else { Money::ZERO };
                        let interest_paid = if paid_interest { sp.interest_amount } else { Money::ZERO };
                        effects.push(StateEffect::Credit(CreditEffect::ProcessLoanPayment {
                            loan_id: *loan_id,
                            principal_paid,
                            interest_paid,
                            fees_paid: Money::ZERO,
                            payment_date: state.current_date,
                        }));
                    }

                    if paid_interest && paid_principal {
                        sp.status = PaymentStatus::Paid;
                        sp.paid_date = Some(state.current_date);
                        sp.dpd_days = 0;
                    } else if missed {
                        sp.status = PaymentStatus::Missed;
                        sp.dpd_days += 1;
                    }
                }

                let dpd_max = sched.scheduled_payments.iter().map(|sp| sp.dpd_days).max().unwrap_or(0);
                let stage = if dpd_max >= 90 {
                    ImpairmentStage::Stage3NonPerforming
                } else if dpd_max >= 30 {
                    ImpairmentStage::Stage2Underperforming
                } else {
                    ImpairmentStage::Stage1Performing
                };
                effects.push(StateEffect::Credit(CreditEffect::RecordImpairment {
                    loan_id: *loan_id,
                    stage,
                    provision: Money::ZERO,
                }));
                if stage == ImpairmentStage::Stage3NonPerforming {
                    loans_to_enforce.push(*loan_id);
                }
            }

            for loan_id in loans_to_enforce {
                if let Some(liens) = fs.credit_registry.liens_by_loan.get(&loan_id) {
                    for lien_id in liens {
                        effects.push(StateEffect::Credit(CreditEffect::UpdateLienStatus {
                            lien_id: *lien_id,
                            new_status: LienStatus::Enforced,
                        }));
                    }
                }
            }

            engine.state.apply_effects(&effects).map_err(|e| e.to_string())?;
            Ok(StepTelemetry::single("credit_effects", effects.len()))
        })
    }
}

#[derive(Debug)]
pub struct CreditServicingHandler;

impl StepHandler for CreditServicingHandler {
    fn execute(
        &self, engine: &mut SimulationEngine, _ctx: &mut StepContext, _rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        execute_step(|| {
            let state = &mut engine.state;
            let fs = &mut state.financial_system;

            let mut effects: Vec<StateEffect> = Vec::new();
            for (inst_id, inst) in fs.instruments.instruments.clone() {
                if let InstrumentRuntime::Credit(CreditState::Loan(loan_state)) = inst.state().clone() {
                    let servicer = LoanServicer::new(state.current_date);
                    let mut accrual_state = loan_state.clone();
                    let interest = servicer.accrue_interest(&mut accrual_state);
                    if interest > Money::ZERO {
                        effects.push(StateEffect::Credit(CreditEffect::AccrueLoanInterest {
                            loan_id: loan_state.loan_id,
                            interest_amount: interest,
                            accrual_date: state.current_date,
                        }));
                    }

                    let payer = loan_state.borrower;
                    let payee = loan_state.lender;
                    let (from_bank, to_bank) = {
                        let (_, fb) = fs
                            .find_agent_liquid_account(&payer)
                            .ok_or_else(|| format!("Borrower {:?} has no liquid account", payer))?;
                        let (_, tb) = fs
                            .find_agent_liquid_account(&payee)
                            .or_else(|| fs.find_any_bank_account())
                            .ok_or_else(|| "No banks to receive loan payment".to_string())?;
                        (fb, tb)
                    };

                    let schedule = fs
                        .credit_registry
                        .payment_schedules
                        .entry(loan_state.loan_id)
                        .or_insert_with(|| LoanServicer::new(state.current_date).generate_schedule(&loan_state));

                    for sp in schedule.scheduled_payments.iter_mut() {
                        if sp.status == PaymentStatus::Scheduled && sp.payment_date <= state.current_date {
                            let mk_pi = |amount: Money, ctx: TransactionContext| -> PaymentInstruction {
                                PaymentInstruction {
                                    id: Uuid::new_v4(),
                                    from_bank,
                                    to_bank,
                                    payer,
                                    payee,
                                    amount: amount.to_f64(),
                                    context: ctx,
                                    priority: PaymentPriority::Normal,
                                    earliest_release_tick: state.ticknum,
                                    deadline_tick: state.ticknum + 10,
                                }
                            };

                            let i_pi =
                                mk_pi(sp.interest_amount, TransactionContext::CouponPayment { instrument_id: inst_id });
                            let p_pi = mk_pi(
                                sp.principal_amount,
                                TransactionContext::PrincipalRepayment { instrument_id: inst_id },
                            );

                            sp.interest_payment_id = Some(i_pi.id);
                            sp.principal_payment_id = Some(p_pi.id);

                            effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(i_pi)));
                            effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(p_pi)));
                        }
                    }
                }
            }

            engine.state.apply_effects(&effects).map_err(|e| e.to_string())?;
            Ok(StepTelemetry::single("serviced_loans", effects.len()))
        })
    }
}

#[derive(Debug)]
pub struct DepositServicingHandler;

impl StepHandler for DepositServicingHandler {
    fn execute(&self, engine: &mut SimulationEngine, _ctx: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            if !is_last_day_of_month(engine.state.current_date) {
                return Ok(StepTelemetry::single("payments_generated", 0usize));
            }

            let mut effects: Vec<StateEffect> = Vec::new();
            let state = &engine.state;
            let fs = &state.financial_system;
            let year_frac = 1.0 / 12.0;

            for (agent_id, bs) in &fs.balance_sheets {
                for (inst_id, pos) in &bs.assets {
                    if let Some(inst) = fs.instruments.instruments.get(inst_id) {
                        if let InstrumentRuntime::Cash(cash) = inst.state() {
                            let is_interest_bearing = matches!(
                                cash.cash_type,
                                CashType::DemandDeposit | CashType::SavingsDeposit | CashType::CentralBankReserves
                            );

                            if is_interest_bearing && cash.interest_bps > dec!(0.0) && pos.quantity > 0.0 {
                                let annual_rate = bps_to_decimal(cash.interest_bps).to_f64().unwrap_or(0.0);
                                let interest_amount = pos.quantity * annual_rate * year_frac;

                                if interest_amount < 0.01 {
                                    continue;
                                }
                                let (from_bank, to_bank) = match cash.cash_type {
                                    CashType::CentralBankReserves => (fs.central_bank.id, fs.central_bank.id),
                                    CashType::DemandDeposit | CashType::SavingsDeposit => (cash.issuer, cash.issuer),
                                    _ => continue,
                                };

                                let pi = PaymentInstruction {
                                    id: Uuid::new_v4(),
                                    from_bank,
                                    to_bank,
                                    payer: cash.issuer,
                                    payee: *agent_id,
                                    amount: interest_amount,
                                    context: TransactionContext::CouponPayment { instrument_id: *inst_id },
                                    priority: PaymentPriority::Normal,
                                    earliest_release_tick: state.ticknum,
                                    deadline_tick: state.ticknum + 10,
                                };
                                effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(pi)));
                            }
                        }
                    }
                }
            }

            let payment_count = effects.len();
            if payment_count > 0 {
                engine.state.apply_effects(&effects).map_err(|e| e.to_string())?;
            }
            Ok(StepTelemetry::single("payments_generated", payment_count))
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
                    ) | StateEffect::Inventory(_)
                )
            })?;
            Ok(StepTelemetry::single("payments_and_settlements_queued", count))
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

            context.effects_mut().extend(finalization_effects.into_iter());

            let final_pending = engine.state.financial_system.rtgs.pending.len();
            let settled_this_tick = initial_pending - final_pending;

            Ok(StepTelemetry::single("payments_settled", settled_this_tick)
                .with_metric("payments_remaining", final_pending))
        })
    }
}
#[derive(Debug)]
pub struct ApplyAllEffectsHandler;
impl StepHandler for ApplyAllEffectsHandler {
    #[instrument(skip(self, engine, context, rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let labour_effects = engine.match_labour_markets(rng);
            let total_effects = {
                let effects = context.effects_mut();
                effects.extend(labour_effects.into_iter());
                effects.len()
            };

            let actions = context.actions();
            let intentions = context.intentions_slice();
            let effects_snapshot = context.effects();
            let mapping = context.action_to_effect_indices_ref();

            let mut new_events: Vec<SimEvent> = Vec::new();

            new_events.extend(intentions.iter().cloned().map(SimEvent::Intention));
            new_events.extend(actions.iter().cloned().map(SimEvent::Action));
            new_events.extend(effects_snapshot.iter().cloned().map(SimEvent::Effect));

            for (action_idx, action_record) in actions.iter().enumerate() {
                if let Some(effect_indices) = mapping.get(&action_idx) {
                    let financial_effects: Vec<FinancialEffect> = effect_indices
                        .iter()
                        .filter_map(|&effect_idx| {
                            effects_snapshot.get(effect_idx).and_then(|eff| match eff {
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

            engine.state.apply_effects(context.effects()).map_err(|e| e.to_string())?;

            Ok(StepTelemetry::single("total_effects_applied", total_effects))
        })
    }
}
#[derive(Debug)]
pub struct UpdateHistoryHandler;
impl StepHandler for UpdateHistoryHandler {
    #[instrument(skip(self, engine, context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let trades = context.trades();
            let snapshots = context.market_snapshots_ref();
            engine.update_market_history(trades, snapshots);
            engine.refresh_pricing_feeds();
            let action_to_effect_indices: StdHashMap<usize, Vec<usize>> =
                context.action_to_effect_indices_ref().iter().map(|(k, v)| (*k, v.clone())).collect();

            let tick_record = TickRecord {
                tick_number: engine.state.ticknum,
                date: engine.state.current_date,
                intentions: context.intentions_slice().to_vec(),
                actions: context.actions().to_vec(),
                effects: context.effects().to_vec(),
                action_to_effect_indices,
                trades: trades.to_vec(),
                events: engine.event_log.clone(),
            };
            engine.state.history.add_tick_record(tick_record);
            Ok(StepTelemetry::default())
        })
    }
}

#[derive(Debug)]
pub struct DebtAuctionsHandler;
impl StepHandler for DebtAuctionsHandler {
    fn execute(&self, engine: &mut SimulationEngine, context: &mut StepContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let open_auctions: Vec<_> = engine
                .state
                .financial_system
                .exchange
                .open_auctions
                .iter()
                .filter(|(_, a)| a.status == AuctionStatus::Open)
                .map(|(id, _)| *id)
                .collect();

            if open_auctions.is_empty() {
                return Ok(StepTelemetry::single("auctions_processed", false));
            }

            let mut all_auction_trades: Vec<Trade> = Vec::new();

            for auction_id in open_auctions {
                let auction = engine
                    .state
                    .financial_system
                    .exchange
                    .open_auctions
                    .get(&auction_id)
                    .ok_or_else(|| format!("Auction {} not found", auction_id))?;

                let inst_id = auction.instrument_id;
                let quantity_offered = auction.quantity_offered;

                let mut trades_this_auction = match engine
                    .state
                    .financial_system
                    .exchange
                    .conduct_dutch_auction(&auction_id, &engine.state.financial_system.instruments.instruments)
                {
                    Ok(trades) => trades,
                    Err(e) => {
                        tracing::error!("Failed to conduct auction {}: {}", auction_id, e);
                        continue;
                    }
                };

                let inst_symbol = engine
                    .state
                    .financial_system
                    .exchange
                    .inst_to_symbol
                    .get(&inst_id)
                    .cloned()
                    .unwrap_or_else(|| Symbol(format!("UNKNOWN_{}", inst_id)));

                let sold_qty: u32 =
                    trades_this_auction.iter().filter(|t| t.market_id == inst_symbol).map(|t| t.quantity as u32).sum();

                let leftover = (quantity_offered - sold_qty).max(0);
                if leftover > 0 {
                    let fallback_price = if let Some(inst) =
                        engine.state.financial_system.instruments.instruments.get(&inst_id)
                    {
                        if let InstrumentRuntime::Bond(b) = &inst.state() {
                            let y_backstop =
                                (engine.state.financial_system.central_bank.policy_rate_bps - dec!(50)).max(dec!(0));
                            let pricer = GovTermStructurePricer::new(
                                BondPricingTerms::from(b),
                                TermStructureMethod::default(),
                                engine.state.financial_system.pricing_feeds.clone(),
                            );
                            let fallback_price = pricer
                                .price_from_yield(&inst_id, bps_to_decimal(y_backstop).to_f64().unwrap_or_default())
                                .unwrap_or(Money::from(1000));
                            fallback_price
                        } else {
                            Money::from(1000)
                        }
                    } else {
                        Money::from(1000)
                    };

                    let last_price = trades_this_auction
                        .iter()
                        .rev()
                        .find(|t| t.market_id == inst_symbol)
                        .map(|t| t.price)
                        .unwrap_or(fallback_price);

                    trades_this_auction.push(Trade {
                        trade_id: Uuid::new_v4(),
                        market_id: inst_symbol,
                        buyer: engine.state.financial_system.central_bank.id,
                        seller: engine.state.financial_system.government.id,
                        quantity: leftover as f64,
                        price: last_price,
                    });
                }
                all_auction_trades.extend(trades_this_auction);
            }

            if !all_auction_trades.is_empty() {
                let now = std::time::SystemTime::now();
                let tape = &mut engine.state.financial_system.exchange.tape;
                for t in &all_auction_trades {
                    tape.entry(t.market_id.clone()).or_default().push(TimedTrade { ts: now, trade: t.clone() });
                }

                context.trades_mut().extend(all_auction_trades.into_iter());
            }

            Ok(StepTelemetry::single("auctions_processed", true))
        })
    }
}

#[derive(Debug)]
pub struct ClearOvernightHandler;

impl StepHandler for ClearOvernightHandler {
    fn execute(
        &self, engine: &mut crate::executor::SimulationEngine, _context: &mut super::StepContext,
        _rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        execute_step(|| {
            let initial_fedfunds = engine.state.financial_system.funding_markets.fedfunds_on.len();
            let initial_repo = engine.state.financial_system.funding_markets.repo_gc1d.len();

            tracing::info!("Clearing overnight funding markets: {} fedfunds, {} repos", initial_fedfunds, initial_repo);
            Ok(StepTelemetry::single("fedfunds_cleared", initial_fedfunds).with_metric("repos_cleared", initial_repo))
        })
    }
}
