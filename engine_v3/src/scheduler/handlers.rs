
use super::{PhaseContext, PhaseHandler, StepResult, StepTelemetry};
use crate::executor::SimulationEngine;
use domains::{ResolutionContext, ResolutionPhase};
use rand::prelude::*;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use sim_core::time::wall_clock_now;
use sim_core::types::core_utils::time::is_coupon_date;
use sim_core::types::markets::BondPricingTerms;
use sim_core::*;
use std::collections::HashMap as StdHashMap;
use tracing::instrument;
use uuid::Uuid;
use web_time::Instant;

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
pub struct BankBalanceSheetSummaryHandler;

impl PhaseHandler for BankBalanceSheetSummaryHandler {
    fn execute(&self, engine: &mut SimulationEngine, _ctx: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let state = &engine.state;
            let fs = &state.financial_system;

            tracing::info!("\n=== Bank Balance Sheet Summary (Date: {}) ===", state.current_date);

            for (bank_id, _bank) in &state.agents.banks {
                let bs = match fs.balance_sheets.get(bank_id) {
                    Some(bs) => bs,
                    None => continue,
                };

                let reserves: f64 = bs.assets.iter()
                    .filter_map(|(inst_id, pos)| {
                        fs.instruments.instruments.get(inst_id).and_then(|inst| {
                            if let InstrumentRuntime::Cash(cash) = inst.state() {
                                if cash.cash_type == CashType::CentralBankReserves {
                                    return Some(pos.quantity);
                                }
                            }
                            None
                        })
                    })
                    .sum();

                let (ib_loans_lent, repos_lent): (f64, f64) = bs.assets.iter()
                    .filter_map(|(inst_id, pos)| {
                        fs.instruments.instruments.get(inst_id).and_then(|inst| {
                            if let InstrumentRuntime::Credit(CreditState::OvernightCredit(credit)) = inst.state() {
                                match credit.credit_type {
                                    OvernightCreditType::InterbankLoan | OvernightCreditType::FedFunds => Some((pos.quantity, 0.0)),
                                    OvernightCreditType::Repo => Some((0.0, pos.quantity)),
                                }
                            } else {
                                None
                            }
                        })
                    })
                    .fold((0.0, 0.0), |(ib, repo), (ib_new, repo_new)| (ib + ib_new, repo + repo_new));

                let (ib_loans_borrowed, repos_borrowed): (f64, f64) = bs.liabilities.iter()
                    .filter_map(|(inst_id, pos)| {
                        fs.instruments.instruments.get(inst_id).and_then(|inst| {
                            if let InstrumentRuntime::Credit(CreditState::OvernightCredit(credit)) = inst.state() {
                                match credit.credit_type {
                                    OvernightCreditType::InterbankLoan | OvernightCreditType::FedFunds => Some((pos.quantity, 0.0)),
                                    OvernightCreditType::Repo => Some((0.0, pos.quantity)),
                                }
                            } else {
                                None
                            }
                        })
                    })
                    .fold((0.0, 0.0), |(ib, repo), (ib_new, repo_new)| (ib + ib_new, repo + repo_new));

                let total_assets: f64 = bs.assets.values().map(|pos| pos.quantity).sum();
                let total_liabilities: f64 = bs.liabilities.values().map(|pos| pos.quantity).sum();
                let equity = total_assets - total_liabilities;

                tracing::info!(
                    "  Bank {:?}: Reserves=${:.0}k | IB Loans: +${:.0}k/-${:.0}k | Repos: +${:.0}k/-${:.0}k | Assets=${:.0}k | Liab=${:.0}k | Equity=${:.0}k",
                    bank_id,
                    reserves / 1000.0,
                    ib_loans_lent / 1000.0,
                    ib_loans_borrowed / 1000.0,
                    repos_lent / 1000.0,
                    repos_borrowed / 1000.0,
                    total_assets / 1000.0,
                    total_liabilities / 1000.0,
                    equity / 1000.0
                );
            }

            Ok(StepTelemetry::default())
        })
    }
}

#[derive(Debug)]
pub struct UpkeepHandler;

impl PhaseHandler for UpkeepHandler {
    fn execute(&self, engine: &mut SimulationEngine, _ctx: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            engine.state.advance_time();

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
impl PhaseHandler for GatherIntentionsHandler {
    fn execute(&self, engine: &mut SimulationEngine, context: &mut PhaseContext, rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let intentions = engine.gather_intentions(rng);
            let categorized = engine.domain_registry.categorize_intentions_by_phase(intentions.clone());

            context.tick_context.set_intentions(intentions);
            context.tick_context.set_categorized_intentions(categorized);

            let mut concatted = String::new();
            let total_intentions = if let Some(stored) = context.tick_context.intentions() {
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
impl PhaseHandler for PhaseResolutionHandler {
    fn name(&self) -> &'static str {
        match self.phase {
            ResolutionPhase::Independent => "PhaseResolutionHandler(Independent)",
            ResolutionPhase::Market => "PhaseResolutionHandler(Market)",
            ResolutionPhase::Dependent => "PhaseResolutionHandler(Dependent)",
        }
    }

    fn execute(&self, engine: &mut SimulationEngine, context: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let intentions = match context
                .tick_context
                .categorized_intentions()
                .and_then(|categorized| categorized.get(&self.phase))
            {
                Some(list) if !list.is_empty() => list,
                _ => return Ok(StepTelemetry::single("actions", 0usize).with_metric("effects", 0usize)),
            };

            let resolution_context = ResolutionContext { state: &engine.state, current_tick: engine.state.ticknum };
            let action_offset = context.tick_context.actions_len();
            let effect_offset = context.tick_context.effects_len();

            let (action_records, action_to_effect_indices, effects) =
                engine.resolve_and_execute_phase(intentions, &resolution_context, action_offset, effect_offset);

            let action_count = action_records.len();
            context.tick_context.actions_mut().extend(action_records.into_iter());

            let effect_count = effects.len();
            context.tick_context.effects_mut().extend(effects.into_iter());

            context.tick_context.action_to_effect_indices_mut().extend(action_to_effect_indices);

            Ok(StepTelemetry::single("actions", action_count).with_metric("effects", effect_count))
        })
    }
}
#[derive(Debug)]
pub struct ApplyMarketEffectsHandler;
impl PhaseHandler for ApplyMarketEffectsHandler {
    #[instrument(skip(self, engine, context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let market_effects: Vec<StateEffect> = context
                .tick_context
                .effects()
                .iter()
                .filter(|e| matches!(e, StateEffect::Market(_)))
                .cloned()
                .collect();

            engine.state.apply_effects(&market_effects).map_err(|e| e.to_string())?;

            let staged = std::mem::take(&mut engine.state.financial_system.exchange.recent_trades);
            if !staged.is_empty() {
                context.tick_context.trades_mut().extend(staged);
            }
            Ok(StepTelemetry::single("market_effects_applied", market_effects.len()))
        })
    }
}
#[derive(Debug)]
pub struct ClearMarketsHandler;
impl PhaseHandler for ClearMarketsHandler {
    #[instrument(skip(self, engine, context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let (market_trades, snapshots) = engine.clear_all_markets();
            if !market_trades.is_empty() {
                let now = wall_clock_now();
                let tape = &mut engine.state.financial_system.exchange.tape;
                for t in &market_trades {
                    tape.entry(t.market_id.clone()).or_default().push(TimedTrade { ts: now, trade: t.clone() });
                }
            }
            let trades_generated = market_trades.len();
            context.tick_context.trades_mut().extend(market_trades.into_iter());

            context.tick_context.set_market_snapshots(snapshots);
            Ok(StepTelemetry::single("trades_generated", trades_generated))
        })
    }
}
#[derive(Debug)]
pub struct GovCouponsHandler;

impl PhaseHandler for GovCouponsHandler {
    fn execute(&self, engine: &mut SimulationEngine, _ctx: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
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
                            None => continue,
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

impl PhaseHandler for SettleTradesHandler {
    fn execute(&self, engine: &mut SimulationEngine, context: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let all_trades = context.tick_context.trades();
            if all_trades.is_empty() {
                return Ok(
                    StepTelemetry::single("trades_processed", 0usize).with_metric("settlement_effects", 0usize)
                );
            }

            let already_settled: std::collections::HashSet<Uuid> = engine
                .state
                .financial_system
                .clearing_house
                .csd
                .pending_settlements
                .keys()
                .copied()
                .collect();

            let unsettled_trades: Vec<Trade> = all_trades
                .iter()
                .filter(|trade| !already_settled.contains(&trade.trade_id))
                .cloned()
                .collect();

            if unsettled_trades.is_empty() {
                return Ok(
                    StepTelemetry::single("trades_processed", 0usize).with_metric("settlement_effects", 0usize)
                );
            }

            let settlement_effects = engine.settle_trades(&unsettled_trades);
            let effect_count = settlement_effects.len();
            context.tick_context.effects_mut().extend(settlement_effects.into_iter());

            Ok(StepTelemetry::single("trades_processed", unsettled_trades.len())
                .with_metric("settlement_effects", effect_count))
        })
    }
}
#[derive(Debug)]
pub struct CreditReconciliationHandler;

impl PhaseHandler for CreditReconciliationHandler {
    fn execute(
        &self, engine: &mut SimulationEngine, _ctx: &mut PhaseContext, _rng: &mut dyn rand::RngCore,
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

impl PhaseHandler for CreditServicingHandler {
    fn execute(
        &self, engine: &mut SimulationEngine, _ctx: &mut PhaseContext, _rng: &mut dyn rand::RngCore,
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
pub struct InterbankLoanServicingHandler;

impl PhaseHandler for InterbankLoanServicingHandler {
    fn execute(&self, engine: &mut SimulationEngine, _ctx: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let mut effects: Vec<StateEffect> = Vec::new();
            let state = &engine.state;
            let fs = &state.financial_system;
            let current_date = state.current_date;

            let interbank_loans: Vec<(InstrumentId, OvernightCreditState, AgentId, AgentId)> = fs
                .instruments
                .instruments
                .iter()
                .filter_map(|(inst_id, inst)| {
                    if let InstrumentRuntime::Credit(CreditState::OvernightCredit(credit)) = inst.state() {
                        if credit.credit_type == OvernightCreditType::InterbankLoan {
                            return Some((*inst_id, credit.clone(), credit.lender, credit.borrower));
                        }
                    }
                    None
                })
                .collect();

            let loans_matured_count =
                interbank_loans.iter().filter(|(_id, credit, _, _)| current_date >= credit.maturity_date).count();

            for (inst_id, credit, lender_id, borrower_id) in interbank_loans {
                let is_maturity = current_date >= credit.maturity_date;

                if !is_maturity {
                    continue;
                }

                let principal = credit.amount.to_f64();
                let coupon_rate = bps_to_decimal(credit.rate_bps).to_f64().unwrap_or(0.0);
                let interest = principal * coupon_rate / 365.0;
                let total_payment = principal + interest;

                let (_borrower_account_id, borrower_bank) = match fs.find_agent_liquid_account(&borrower_id) {
                    Some(account) => account,
                    None => {
                        tracing::warn!("Cannot service interbank loan {}: borrower {} has no liquid account", inst_id, borrower_id);
                        continue;
                    }
                };

                let (_lender_account_id, lender_bank) = match fs.find_agent_liquid_account(&lender_id) {
                    Some(account) => account,
                    None => {
                        tracing::warn!("Cannot service interbank loan {}: lender {} has no liquid account", inst_id, lender_id);
                        continue;
                    }
                };

                effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(PaymentInstruction {
                    id: Uuid::new_v4(),
                    from_bank: borrower_bank,
                    to_bank: lender_bank,
                    payer: borrower_id,
                    payee: lender_id,
                    amount: total_payment,
                    context: TransactionContext::PrincipalRepayment { instrument_id: inst_id },
                    priority: PaymentPriority::Urgent,
                    earliest_release_tick: state.ticknum,
                    deadline_tick: state.ticknum + 10,
                })));

                effects.push(StateEffect::Financial(FinancialEffect::RedeemInstrument { instrument_id: inst_id }));
            }

            if !effects.is_empty() {
                engine.state.apply_effects(&effects).map_err(|e| e.to_string())?;
            }
            Ok(StepTelemetry::single("loans_matured", loans_matured_count))
        })
    }
}

#[derive(Debug)]
pub struct RepoServicingHandler;

impl PhaseHandler for RepoServicingHandler {
    fn execute(&self, engine: &mut SimulationEngine, _ctx: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let mut effects: Vec<StateEffect> = Vec::new();
            let state = &engine.state;
            let fs = &state.financial_system;
            let current_date = state.current_date;

            let repo_agreements: Vec<(InstrumentId, OvernightCreditState, AgentId, AgentId)> = fs
                .instruments
                .instruments
                .iter()
                .filter_map(|(inst_id, inst)| {
                    if let InstrumentRuntime::Credit(CreditState::OvernightCredit(credit)) = inst.state() {
                        if credit.credit_type == OvernightCreditType::Repo {
                            return Some((*inst_id, credit.clone(), credit.lender, credit.borrower));
                        }
                    }
                    None
                })
                .collect();

            let repos_matured_count =
                repo_agreements.iter().filter(|(_id, credit, _, _)| current_date >= credit.maturity_date).count();

            for (inst_id, credit, lender_id, borrower_id) in repo_agreements {
                let is_maturity = current_date >= credit.maturity_date;

                if !is_maturity {
                    continue;
                }

                let principal = credit.amount.to_f64();
                let coupon_rate = bps_to_decimal(credit.rate_bps).to_f64().unwrap_or(0.0);
                let interest = principal * coupon_rate / 365.0;
                let total_repayment = principal + interest;

                let collateral_id = credit.collateral.expect("Repo must have collateral");
                let collateral_qty = credit.collateral_quantity.expect("Repo must have collateral quantity");

                let (_borrower_account_id, borrower_bank) = match fs.find_agent_liquid_account(&borrower_id) {
                    Some(account) => account,
                    None => continue,
                };

                let (_lender_account_id, lender_bank) = match fs.find_agent_liquid_account(&lender_id) {
                    Some(account) => account,
                    None => continue,
                };

                effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(PaymentInstruction {
                    id: Uuid::new_v4(),
                    from_bank: borrower_bank,
                    to_bank: lender_bank,
                    payer: borrower_id,
                    payee: lender_id,
                    amount: total_repayment,
                    context: TransactionContext::PrincipalRepayment { instrument_id: inst_id },
                    priority: PaymentPriority::Urgent,
                    earliest_release_tick: state.ticknum,
                    deadline_tick: state.ticknum + 10,
                })));

                let trade_id = Uuid::new_v4();
                let collateral_settlement = SettlementInstruction {
                    instruction_id: Uuid::new_v4(),
                    trade_id,
                    seller: lender_id,
                    buyer: borrower_id,
                    instrument_id: collateral_id,
                    quantity: collateral_qty,
                    cash_amount: 0.0,
                    settlement_date: current_date,
                    status: SettlementStatus::Pending,
                };

                effects
                    .push(StateEffect::Financial(FinancialEffect::RecordSettlementInstruction(collateral_settlement)));

                effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(PaymentInstruction {
                    id: Uuid::new_v4(),
                    from_bank: borrower_bank,
                    to_bank: lender_bank,
                    payer: borrower_id,
                    payee: lender_id,
                    amount: 0.0,
                    context: TransactionContext::TradeSettlement { trade_id },
                    priority: PaymentPriority::Urgent,
                    earliest_release_tick: state.ticknum,
                    deadline_tick: state.ticknum + 10,
                })));

                effects.push(StateEffect::Financial(FinancialEffect::RedeemInstrument { instrument_id: inst_id }));
            }

            if repos_matured_count > 0 {
                engine.state.apply_effects(&effects).map_err(|e| e.to_string())?;
            }
            Ok(StepTelemetry::single("repos_matured", repos_matured_count))
        })
    }
}

#[derive(Debug)]
pub struct DepositServicingHandler;

impl PhaseHandler for DepositServicingHandler {
    fn execute(&self, engine: &mut SimulationEngine, _ctx: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
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
impl PhaseHandler for ApplyPaymentQueuingHandler {
    fn execute(&self, engine: &mut SimulationEngine, context: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let count = engine.consume_effects(context.tick_context, |effect| {
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

impl PhaseHandler for RunRTGSHandler {
    #[instrument(skip(self, engine, context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let initial_pending = engine.state.financial_system.rtgs.pending.len();
            let initial_settled = engine.state.financial_system.rtgs.settled.len();

            let finalization_effects =
                run_rtgs(&mut engine.state).map_err(|e| format!("RTGS execution failed: {:?}", e))?;

            let final_settled = engine.state.financial_system.rtgs.settled.len();
            let settled_this_session = final_settled - initial_settled;

            if settled_this_session > 0 {
                tracing::info!("RTGS settled {} payments this session (total this tick: {})",
                    settled_this_session, final_settled);
            }

            context.tick_context.effects_mut().extend(finalization_effects.into_iter());
            let final_pending = engine.state.financial_system.rtgs.pending.len();
            let settled_this_run = initial_pending - final_pending;

            Ok(StepTelemetry::single("payments_settled", settled_this_run)
                .with_metric("payments_remaining", final_pending)
                .with_metric("total_settled_this_tick", final_settled))
        })
    }
}
#[derive(Debug)]
pub struct ApplyAllEffectsHandler;
impl PhaseHandler for ApplyAllEffectsHandler {
    #[instrument(skip(self, engine, context, rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut PhaseContext, rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let labour_effects = engine.match_labour_markets(rng);
            let total_effects = {
                let effects = context.tick_context.effects_mut();
                effects.extend(labour_effects.into_iter());
                effects.len()
            };

            let actions = context.tick_context.actions();
            let intentions = context.tick_context.intentions_slice();
            let effects_snapshot = context.tick_context.effects();
            let mapping = context.tick_context.action_to_effect_indices_ref();

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
            engine.state.apply_effects(context.tick_context.effects()).map_err(|e| e.to_string())?;

            Ok(StepTelemetry::single("total_effects_applied", total_effects))
        })
    }
}
#[derive(Debug)]
pub struct UpdateHistoryHandler;
impl PhaseHandler for UpdateHistoryHandler {
    #[instrument(skip(self, engine, context, _rng))]
    fn execute(&self, engine: &mut SimulationEngine, context: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let trades = context.tick_context.trades();
            let snapshots = context.tick_context.market_snapshots_ref();
            engine.update_market_history(trades, snapshots);
            engine.refresh_pricing_feeds();
            let action_to_effect_indices =
                context.tick_context.take_action_to_effect_indices().into_iter().collect::<StdHashMap<_, _>>();

            let tick_record = TickRecord {
                tick_number: engine.state.ticknum,
                date: engine.state.current_date,
                intentions: context.tick_context.take_intentions(),
                actions: context.tick_context.take_actions(),
                effects: context.tick_context.take_effects(),
                action_to_effect_indices,
                trades: context.tick_context.take_trades(),
                events: engine.event_log.clone(),
            };
            engine.state.history.add_tick_record(tick_record);
            Ok(StepTelemetry::default())
        })
    }
}

#[derive(Debug)]
pub struct DebtAuctionsHandler;
impl PhaseHandler for DebtAuctionsHandler {
    fn execute(&self, engine: &mut SimulationEngine, context: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
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
                let now = wall_clock_now();
                let tape = &mut engine.state.financial_system.exchange.tape;
                for t in &all_auction_trades {
                    tape.entry(t.market_id.clone()).or_default().push(TimedTrade { ts: now, trade: t.clone() });
                }

                context.tick_context.trades_mut().extend(all_auction_trades.into_iter());
            }

            Ok(StepTelemetry::single("auctions_processed", true))
        })
    }
}

fn generate_fallback_repo_quotes(state: &SimState) -> Vec<ONQuote> {
    let mut quotes = Vec::new();
    let fs = &state.financial_system;
    let policy_rate_bps = fs.central_bank.policy_rate_bps;
    let typical_spread = dec!(20);

    for (bank_id, _bank) in &state.agents.banks {
        let bank_bs = match fs.balance_sheets.get(bank_id) {
            Some(bs) => bs,
            None => continue,
        };

        let total_deposits: f64 = bank_bs
            .liabilities
            .iter()
            .filter_map(|(id, pos)| {
                fs.instruments.instruments.get(id).and_then(|inst| {
                    if let InstrumentRuntime::Cash(d) = inst.state() {
                        if matches!(d.cash_type, CashType::DemandDeposit | CashType::SavingsDeposit) {
                            return Some(pos.quantity);
                        }
                    }
                    None
                })
            })
            .sum();

        let required_reserves = total_deposits * fs.central_bank.reserve_requirement;
        let desired_buffer = total_deposits * 0.02;
        let target_reserve_level = required_reserves + desired_buffer;
        let current_reserves = fs.get_bank_reserves(bank_id).unwrap_or(0.0);
        let reserve_gap = current_reserves - target_reserve_level;

        if reserve_gap < -100.0 {
            let amount_needed = -reserve_gap;

            let has_govt_bonds = bank_bs.assets.iter().any(|(inst_id, _)| {
                fs.instruments.instruments.get(inst_id).map_or(false, |inst| {
                    if let InstrumentRuntime::Bond(bond_state) = inst.state() {
                        bond_state.bond_type() == BondType::Government
                    } else {
                        false
                    }
                })
            });

            if has_govt_bonds {
                let repo_borrow_rate = policy_rate_bps + typical_spread;
                quotes.push(ONQuote {
                    venue: OvernightVenue::RepoGC1D,
                    agent: *bank_id,
                    side: ONQuoteSide::Borrow,
                    notional: amount_needed,
                    limit_rate_bps: repo_borrow_rate,
                    haircut: Some(dec!(0.02)),
                    preferred_collateral: None,
                    min_fill: 100.0,
                    ts: 0,
                });
            }
        } else if reserve_gap > 100.0 {
            let amount_to_lend = reserve_gap * 0.75;
            if amount_to_lend > 100.0 {
                let repo_lend_rate = policy_rate_bps - typical_spread + dec!(5);
                quotes.push(ONQuote {
                    venue: OvernightVenue::RepoGC1D,
                    agent: *bank_id,
                    side: ONQuoteSide::Lend,
                    notional: amount_to_lend,
                    limit_rate_bps: repo_lend_rate,
                    haircut: Some(dec!(0.02)),
                    preferred_collateral: Some(vec![]),
                    min_fill: 100.0,
                    ts: 0,
                });
            }
        }
    }

    quotes
}

#[derive(Debug)]
pub struct ClearOvernightHandler;

impl PhaseHandler for ClearOvernightHandler {
    fn execute(
        &self, engine: &mut crate::executor::SimulationEngine, _context: &mut PhaseContext,
        _rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        execute_step(|| {
            use domains::Domain;
            use sim_core::types::markets::overnight_clearing::clear_fedfunds;

            let fedfunds_quotes = engine.state.financial_system.funding_markets.take_fedfunds();
            let ff_result = clear_fedfunds(fedfunds_quotes);

            let banking_domain = domains::banking_domain::BankingDomain {};
            let mut all_effects = Vec::new();

            for trade in &ff_result.matches {
                let action = SimAction::Banking(BankingAction::ExecuteInterbankLoan {
                    lender_id: trade.lender,
                    borrower_id: trade.borrower,
                    amount: trade.amount,
                    rate_bps: trade.rate_bps,
                });

                let result = banking_domain.execute(&action, &engine.state);
                if result.success {
                    all_effects.extend(result.effects);
                }
            }

            engine.state.apply_effects(&all_effects).map_err(|e| e.to_string())?;

            let mut auto_repo_quotes = generate_fallback_repo_quotes(&engine.state);
            let manual_repo_quotes = engine.state.financial_system.funding_markets.take_repo_gc1d();
            auto_repo_quotes.extend(manual_repo_quotes);

            let repo_result =
                sim_core::types::markets::overnight_clearing::clear_repo_gc1d(auto_repo_quotes, &engine.state, 2.0);

            let mut repo_effects = Vec::new();
            for trade in &repo_result.matches {
                let collateral = trade.collateral.as_ref().unwrap();
                let action = SimAction::Banking(BankingAction::ExecuteRepoAgreement {
                    lender_id: trade.lender,
                    borrower_id: trade.borrower,
                    amount: trade.amount,
                    rate_bps: trade.rate_bps,
                    collateral_id: collateral.instrument_id,
                    collateral_qty: collateral.quantity,
                    haircut_pct: collateral.haircut_pct,
                });

                let result = banking_domain.execute(&action, &engine.state);
                if result.success {
                    repo_effects.extend(result.effects);
                }
            }

            engine.state.apply_effects(&repo_effects).map_err(|e| e.to_string())?;

            if ff_result.matches.len() > 0 || repo_result.matches.len() > 0 {
                tracing::info!(
                    "Overnight markets: FedFunds {} matches ${:.0}, Repo {} matches ${:.0}",
                    ff_result.matches.len(),
                    ff_result.clearing_stats.total_volume,
                    repo_result.matches.len(),
                    repo_result.clearing_stats.total_volume
                );
            }

            Ok(StepTelemetry::single("fedfunds_cleared", ff_result.clearing_stats.num_matches)
                .with_metric("fedfunds_volume", ff_result.clearing_stats.total_volume as usize)
                .with_metric("repos_cleared", repo_result.clearing_stats.num_matches)
                .with_metric("repos_volume", repo_result.clearing_stats.total_volume as usize))
        })
    }
}

#[derive(Debug)]
pub struct ReserveCalcHandler;

impl PhaseHandler for ReserveCalcHandler {
    fn execute(&self, engine: &mut SimulationEngine, _context: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let state = &engine.state;
            let fs = &state.financial_system;
            let cb_id = fs.central_bank.id;
            let reserve_requirement = fs.central_bank.reserve_requirement;

            let mut reserve_diagnostics = Vec::new();
            let mut total_reserves = 0.0;
            let mut total_deposits = 0.0;
            let mut banks_short = 0;
            let mut banks_excess = 0;

            for (agent_id, bs) in &fs.balance_sheets {
                if *agent_id == cb_id || *agent_id == fs.government.id {
                    continue;
                }

                let mut agent_deposits = 0.0;
                for (inst_id, pos) in &bs.liabilities {
                    if let Some(inst) = fs.instruments.instruments.get(inst_id) {
                        if let InstrumentRuntime::Cash(cash) = inst.state() {
                            if matches!(cash.cash_type, CashType::DemandDeposit | CashType::SavingsDeposit) {
                                agent_deposits += pos.quantity * pos.book_value_per_unit.to_f64();
                            }
                        }
                    }
                }

                let mut agent_reserves = 0.0;
                for (inst_id, pos) in &bs.assets {
                    if let Some(inst) = fs.instruments.instruments.get(inst_id) {
                        if let InstrumentRuntime::Cash(cash) = inst.state() {
                            if cash.cash_type == CashType::CentralBankReserves {
                                agent_reserves += pos.quantity * pos.book_value_per_unit.to_f64();
                            }
                        }
                    }
                }

                if agent_deposits > 0.0 {
                    let required_reserves = agent_deposits * reserve_requirement;
                    let shortfall = (required_reserves - agent_reserves).max(0.0);
                    let excess = (agent_reserves - required_reserves).max(0.0);

                    if shortfall > 0.01 {
                        banks_short += 1;
                    } else if excess > 0.01 {
                        banks_excess += 1;
                    }

                    total_reserves += agent_reserves;
                    total_deposits += agent_deposits;

                    reserve_diagnostics.push((*agent_id, agent_deposits, agent_reserves, required_reserves));
                }
            }

            let system_reserve_ratio = if total_deposits > 0.0 {
                total_reserves / total_deposits
            } else {
                0.0
            };

            Ok(StepTelemetry::single("total_reserves", total_reserves as usize)
                .with_metric("total_deposits", total_deposits as usize)
                .with_metric("system_reserve_ratio", (system_reserve_ratio * 10000.0) as usize)
                .with_metric("banks_short", banks_short)
                .with_metric("banks_excess", banks_excess))
        })
    }
}

#[derive(Debug)]
pub struct CorridorFacilitiesHandler;

impl PhaseHandler for CorridorFacilitiesHandler {
    fn execute(&self, engine: &mut SimulationEngine, _context: &mut PhaseContext, _rng: &mut dyn RngCore) -> StepResult {
        execute_step(|| {
            let state = &mut engine.state;
            let fs = &state.financial_system;
            let cb_id = fs.central_bank.id;
            let policy_rate_bps = fs.central_bank.policy_rate_bps;

            let _deposit_facility_rate_bps = (policy_rate_bps - dec!(50)).max(dec!(0));
            let _lending_facility_rate_bps = policy_rate_bps + dec!(100);

            let effects: Vec<StateEffect> = Vec::new();
            let mut deposit_facility_usage = 0.0;
            let mut lending_facility_usage = 0.0;
            let mut agents_using_deposit = 0;
            let mut agents_using_lending = 0;

            for (agent_id, bs) in fs.balance_sheets.iter() {
                if *agent_id == cb_id || *agent_id == fs.government.id {
                    continue;
                }

                let mut agent_deposits = 0.0;
                for (inst_id, pos) in &bs.liabilities {
                    if let Some(inst) = fs.instruments.instruments.get(inst_id) {
                        if let InstrumentRuntime::Cash(cash) = inst.state() {
                            if matches!(cash.cash_type, CashType::DemandDeposit | CashType::SavingsDeposit) {
                                agent_deposits += pos.quantity * pos.book_value_per_unit.to_f64();
                            }
                        }
                    }
                }

                let mut agent_reserves = 0.0;
                for (inst_id, pos) in &bs.assets {
                    if let Some(inst) = fs.instruments.instruments.get(inst_id) {
                        if let InstrumentRuntime::Cash(cash) = inst.state() {
                            if cash.cash_type == CashType::CentralBankReserves {
                                agent_reserves += pos.quantity * pos.book_value_per_unit.to_f64();
                            }
                        }
                    }
                }

                if agent_deposits > 0.0 {
                    let reserve_requirement = fs.central_bank.reserve_requirement;
                    let required_reserves = agent_deposits * reserve_requirement;
                    let excess = agent_reserves - required_reserves;

                    if excess > required_reserves * 0.1 {
                        let facility_amount = (excess * 0.5).max(0.0);
                        if facility_amount > 1.0 {
                            deposit_facility_usage += facility_amount;
                            agents_using_deposit += 1;
                            tracing::debug!("Agent {:?} using deposit facility: ${:.2}", agent_id, facility_amount);
                        }
                    }

                    let shortfall = required_reserves - agent_reserves;
                    if shortfall > 0.01 {
                        let facility_amount = shortfall * 1.0;
                        lending_facility_usage += facility_amount;
                        agents_using_lending += 1;
                        tracing::debug!("Agent {:?} using lending facility: ${:.2}", agent_id, facility_amount);
                    }
                }
            }

            if !effects.is_empty() {
                engine.state.apply_effects(&effects).map_err(|e| e.to_string())?;
            }

            Ok(StepTelemetry::single("deposit_facility_usage", deposit_facility_usage as usize)
                .with_metric("lending_facility_usage", lending_facility_usage as usize)
                .with_metric("agents_using_deposit", agents_using_deposit)
                .with_metric("agents_using_lending", agents_using_lending))
        })
    }
}
