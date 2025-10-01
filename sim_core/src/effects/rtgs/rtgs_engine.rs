use crate::*;
use uuid::Uuid;

pub fn run_rtgs(state: &mut SimState) -> Result<Vec<StateEffect>, EffectError> {
    if !state.financial_system.rtgs_policy.enabled {
        return Ok(vec![]);
    }


    let finalization_effects = match state.financial_system.rtgs_policy.mode {
        RtgsMode::PureRTGS => run_pure_rtgs(state)?,
        RtgsMode::LsmNetting => run_lsm_rtgs_with_effects(state)?,
    };

    Ok(finalization_effects)
}

fn run_pure_rtgs(state: &mut SimState) -> Result<Vec<StateEffect>, EffectError> {
    let current_tick = state.ticknum;
    let mut finalization_effects = Vec::new();

    #[cfg(target_arch = "wasm32")]
    {
        let interbank_count = state
            .financial_system
            .rtgs
            .pending
            .iter()
            .filter(|p| matches!(p.context, TransactionContext::InterbankLoan { .. }))
            .count();
        if interbank_count > 0 {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                "💳 RTGS queue has {} interbank loan payments to process",
                interbank_count
            )));
        }
    }

    state.financial_system.rtgs.pending.sort_by(|a, b| {
        use PaymentPriority::*;
        let priority_order = |p: &PaymentPriority| match p {
            Urgent => 0,
            Normal => 1,
            Low => 2,
        };
        priority_order(&a.priority).cmp(&priority_order(&b.priority)).then(a.id.cmp(&b.id))
    });

    enum StepDecision {
        Advance,
        Settle,
        Expire,
    }

    loop {
        let mut progressed = false;
        let mut i = 0;

        while i < state.financial_system.rtgs.pending.len() {
            let decision = {
                let pi = &state.financial_system.rtgs.pending[i];
                if current_tick < pi.earliest_release_tick {
                    StepDecision::Advance
                } else if can_fund(state, pi)? || can_use_daylight_credit(state, pi)? {
                    StepDecision::Settle
                } else if current_tick >= pi.deadline_tick {
                    StepDecision::Expire
                } else {
                    StepDecision::Advance
                }
            };

            match decision {
                StepDecision::Advance => {
                    i += 1;
                }
                StepDecision::Settle => {
                    let pi = state.financial_system.rtgs.pending.remove(i);

                    apply_cash_movements_immediately(state, &pi)?;

                    if let TransactionContext::TradeSettlement { trade_id } = pi.context {
                        finalization_effects.push(StateEffect::Financial(FinancialEffect::DvPFinalize { trade_id }));
                    }

                    state.financial_system.rtgs.settled.push(pi);
                    progressed = true;
                }
                StepDecision::Expire => {
                    let expired_payment = state.financial_system.rtgs.pending.remove(i);

                    if let TransactionContext::TradeSettlement { trade_id } = expired_payment.context {
                        finalization_effects.push(StateEffect::Financial(FinancialEffect::DvPCancel { trade_id }));
                    }

                    state.financial_system.rtgs.rejected.push((expired_payment, "Deadline exceeded".to_string()));
                    progressed = true;
                }
            }
        }

        if !progressed {
            break;
        }
    }

    Ok(finalization_effects)
}

fn run_lsm_rtgs_with_effects(state: &mut SimState) -> Result<Vec<StateEffect>, EffectError> {
    bilateral_netting(state)?;
    run_pure_rtgs(state)
}

fn _run_lsm_rtgs(state: &mut SimState) -> Result<Vec<StateEffect>, EffectError> {
    bilateral_netting(state)?;
    run_pure_rtgs(state)
}

fn bilateral_netting(state: &mut SimState) -> Result<(), EffectError> {
    use std::collections::HashMap;

    let mut nets: HashMap<(AgentId, AgentId), f64> = HashMap::new();
    let gov_id = state.financial_system.government.id;

    for payment in &state.financial_system.rtgs.pending {
        let is_gov = payment.payer == gov_id || payment.payee == gov_id;
        let is_dvp = matches!(payment.context, TransactionContext::TradeSettlement { .. });
        if is_gov || is_dvp {
            continue;
        }

        let pair = (payment.from_bank, payment.to_bank);
        *nets.entry(pair).or_insert(0.0) += payment.amount;
    }

    let mut processed_pairs = std::collections::HashSet::new();
    let mut to_settle = Vec::new();

    for ((from_bank, to_bank), amount_forward) in &nets {
        if processed_pairs.contains(&(*from_bank, *to_bank)) {
            continue;
        }

        if let Some(amount_reverse) = nets.get(&(*to_bank, *from_bank)) {
            processed_pairs.insert((*from_bank, *to_bank));
            processed_pairs.insert((*to_bank, *from_bank));

            if amount_forward > amount_reverse {
                to_settle.push((*from_bank, *to_bank, amount_forward - amount_reverse));
            } else if amount_reverse > amount_forward {
                to_settle.push((*to_bank, *from_bank, amount_reverse - amount_forward));
            }
        }
    }

    let mut remaining_payments = Vec::new();
    for payment in std::mem::take(&mut state.financial_system.rtgs.pending) {
        let pair = (payment.from_bank, payment.to_bank);
        let is_gov = payment.payer == gov_id || payment.payee == gov_id;
        let is_dvp = matches!(payment.context, TransactionContext::TradeSettlement { .. });

        if is_gov || is_dvp || !processed_pairs.contains(&pair) {
            remaining_payments.push(payment);
        }
    }

    for (from_bank, to_bank, net_amount) in to_settle {
        if net_amount > 0.0 {
            remaining_payments.push(PaymentInstruction {
                id: Uuid::new_v4(),
                from_bank,
                to_bank,
                payer: from_bank,
                payee: to_bank,
                amount: net_amount,
                context: TransactionContext::GenericTransfer { from: from_bank, to: to_bank, amount: net_amount },
                priority: PaymentPriority::Normal,
                earliest_release_tick: state.ticknum,
                deadline_tick: state.ticknum + 10,
            });
        }
    }

    state.financial_system.rtgs.pending = remaining_payments;
    Ok(())
}

fn apply_cash_movements_immediately(state: &mut SimState, pi: &PaymentInstruction) -> Result<(), EffectError> {
    let cb_id = state.financial_system.central_bank.id;
    let gov_id = state.financial_system.government.id;

    if pi.payer == gov_id || pi.payee == gov_id {
        return apply_tga_transfer(state, pi);
    }

    if pi.payer == cb_id && pi.to_bank == cb_id && pi.payee != cb_id {
        let payee_reserves = state
            .financial_system
            .find_bank_reserves_account(&pi.payee)
            .ok_or_else(|| EffectError::InvalidState(format!("Payee {} has no reserves account", pi.payee)))?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            pi.payee,
            payee_reserves,
            pi.amount,
            &PositionSide::Asset,
            None,
        )?;
        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            cb_id,
            payee_reserves,
            pi.amount,
            &PositionSide::Liability,
            None,
        )?;

        return Ok(());
    }
    if let TransactionContext::CouponPayment { instrument_id } = pi.context {
        if let Some(inst) = state.financial_system.instruments.instruments.get(&instrument_id) {
            if let InstrumentRuntime::Cash(details) = inst.state() {
                if matches!(details.cash_type, CashType::DemandDeposit | CashType::SavingsDeposit) {
                    let bank_id = details.issuer;

                    StateEffectApplicator::apply_cash_position_adjustment(
                        state,
                        pi.payee,
                        instrument_id,
                        pi.amount,
                        &PositionSide::Asset,
                        None,
                    )?;
                    StateEffectApplicator::apply_cash_position_adjustment(
                        state,
                        bank_id,
                        instrument_id,
                        pi.amount,
                        &PositionSide::Liability,
                        None,
                    )?;

                    return Ok(());
                }
            }
        }
    }
    if pi.from_bank == pi.to_bank { apply_same_bank_transfer(state, pi) } else { apply_interbank_transfer(state, pi) }
}

fn apply_same_bank_transfer(state: &mut SimState, pi: &PaymentInstruction) -> Result<(), EffectError> {
    let (payer_account_id, _) = state
        .financial_system
        .find_agent_liquid_account(&pi.payer)
        .ok_or_else(|| EffectError::InvalidState("Payer account not found".to_string()))?;

    let (payee_account_id, _) = state
        .financial_system
        .find_agent_liquid_account(&pi.payee)
        .ok_or_else(|| EffectError::InvalidState("Payee account not found".to_string()))?;

    StateEffectApplicator::apply_cash_position_adjustment(
        state,
        pi.payer,
        payer_account_id,
        -pi.amount,
        &PositionSide::Asset,
        None,
    )?;

    StateEffectApplicator::apply_cash_position_adjustment(
        state,
        pi.payee,
        payee_account_id,
        pi.amount,
        &PositionSide::Asset,
        None,
    )?;

    StateEffectApplicator::apply_cash_position_adjustment(
        state,
        pi.from_bank,
        payer_account_id,
        -pi.amount,
        &PositionSide::Liability,
        None,
    )?;

    StateEffectApplicator::apply_cash_position_adjustment(
        state,
        pi.to_bank,
        payee_account_id,
        pi.amount,
        &PositionSide::Liability,
        None,
    )?;

    Ok(())
}

fn apply_interbank_transfer(state: &mut SimState, pi: &PaymentInstruction) -> Result<(), EffectError> {
    let cb_id = state.financial_system.central_bank.id;
    let gov_id = state.financial_system.government.id;

    if pi.payer == gov_id || pi.payee == gov_id {
        return apply_tga_transfer(state, pi);
    }

    if pi.to_bank == cb_id && pi.payee != cb_id {
        let from_bank_reserves = state
            .financial_system
            .find_bank_reserves_account(&pi.from_bank)
            .ok_or_else(|| EffectError::InvalidState(format!("From bank {} reserves not found", pi.from_bank)))?;

        let (payer_account_id, _) = state
            .financial_system
            .find_agent_liquid_account(&pi.payer)
            .ok_or_else(|| EffectError::InvalidState("Payer account not found".to_string()))?;

        let (payee_account_id, _) = state
            .financial_system
            .find_agent_liquid_account(&pi.payee)
            .ok_or_else(|| EffectError::InvalidState(format!("Payee {} account not found", &pi.payee)))?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            pi.payer,
            payer_account_id,
            -pi.amount,
            &PositionSide::Asset,
            None,
        )?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            pi.from_bank,
            payer_account_id,
            -pi.amount,
            &PositionSide::Liability,
            None,
        )?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            pi.from_bank,
            from_bank_reserves,
            -pi.amount,
            &PositionSide::Asset,
            None,
        )?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            cb_id,
            from_bank_reserves,
            -pi.amount,
            &PositionSide::Liability,
            None,
        )?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            pi.payee,
            payee_account_id,
            pi.amount,
            &PositionSide::Asset,
            None,
        )?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            cb_id,
            payee_account_id,
            pi.amount,
            &PositionSide::Liability,
            None,
        )?;

        return Ok(());
    }

    let is_bank_to_bank = pi.payer == pi.from_bank && pi.payee == pi.to_bank;

    if is_bank_to_bank {
        let from_bank_reserves = state
            .financial_system
            .find_bank_reserves_account(&pi.from_bank)
            .ok_or_else(|| EffectError::InvalidState("From bank reserves not found".to_string()))?;

        let to_bank_reserves = state
            .financial_system
            .find_bank_reserves_account(&pi.to_bank)
            .ok_or_else(|| EffectError::InvalidState("To bank reserves not found".to_string()))?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            pi.from_bank,
            from_bank_reserves,
            -pi.amount,
            &PositionSide::Asset,
            None,
        )?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            cb_id,
            from_bank_reserves,
            -pi.amount,
            &PositionSide::Liability,
            None,
        )?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            pi.to_bank,
            to_bank_reserves,
            pi.amount,
            &PositionSide::Asset,
            None,
        )?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            cb_id,
            to_bank_reserves,
            pi.amount,
            &PositionSide::Liability,
            None,
        )?;

        return Ok(());
    }

    let from_bank_reserves = state
        .financial_system
        .find_bank_reserves_account(&pi.from_bank)
        .ok_or_else(|| EffectError::InvalidState("From bank reserves not found".to_string()))?;

    let to_bank_reserves = state
        .financial_system
        .find_bank_reserves_account(&pi.to_bank)
        .ok_or_else(|| EffectError::InvalidState("To bank reserves not found".to_string()))?;

    let (payer_account_id, _) = state
        .financial_system
        .find_agent_liquid_account(&pi.payer)
        .ok_or_else(|| EffectError::InvalidState("Payer account not found".to_string()))?;

    let (payee_account_id, _) = state
        .financial_system
        .find_agent_liquid_account(&pi.payee)
        .ok_or_else(|| EffectError::InvalidState("Payee account not found".to_string()))?;

    StateEffectApplicator::apply_cash_position_adjustment(
        state,
        pi.payer,
        payer_account_id,
        -pi.amount,
        &PositionSide::Asset,
        None,
    )?;

    StateEffectApplicator::apply_cash_position_adjustment(
        state,
        pi.from_bank,
        payer_account_id,
        -pi.amount,
        &PositionSide::Liability,
        None,
    )?;

    StateEffectApplicator::apply_cash_position_adjustment(
        state,
        pi.from_bank,
        from_bank_reserves,
        -pi.amount,
        &PositionSide::Asset,
        None,
    )?;

    StateEffectApplicator::apply_cash_position_adjustment(
        state,
        cb_id,
        from_bank_reserves,
        -pi.amount,
        &PositionSide::Liability,
        None,
    )?;

    StateEffectApplicator::apply_cash_position_adjustment(
        state,
        pi.to_bank,
        to_bank_reserves,
        pi.amount,
        &PositionSide::Asset,
        None,
    )?;

    StateEffectApplicator::apply_cash_position_adjustment(
        state,
        cb_id,
        to_bank_reserves,
        pi.amount,
        &PositionSide::Liability,
        None,
    )?;

    StateEffectApplicator::apply_cash_position_adjustment(
        state,
        pi.payee,
        payee_account_id,
        pi.amount,
        &PositionSide::Asset,
        None,
    )?;

    StateEffectApplicator::apply_cash_position_adjustment(
        state,
        pi.to_bank,
        payee_account_id,
        pi.amount,
        &PositionSide::Liability,
        None,
    )?;

    Ok(())
}
fn apply_central_bank_payment(state: &mut SimState, pi: &PaymentInstruction) -> Result<(), EffectError> {
    let fs = &mut state.financial_system;
    let cb_id = fs.central_bank.id;

    if !state.agents.banks.contains_key(&pi.payee) && pi.payee != fs.government.id {
        return Err(EffectError::InvalidState(format!(
            "Central Bank can only pay IORB to commercial banks. Payee: {}",
            pi.payee
        )));
    }
    let bank_id = pi.payee;
    let reserve_inst_id = fs
        .find_agent_liquid_account(&bank_id)
        .ok_or_else(|| EffectError::InvalidState(format!("Bank {:?} has no reserves account", bank_id)))?
        .0;

    let bank_bs = fs.balance_sheets.get_mut(&bank_id).ok_or(EffectError::AgentNotFound { id: bank_id })?;

    let bank_res_pos = bank_bs.assets.entry(reserve_inst_id).or_default();
    bank_res_pos.quantity += pi.amount;
    bank_bs.income_statement.add_interest_income(pi.amount);

    let cb_bs = fs.balance_sheets.get_mut(&cb_id).ok_or(EffectError::AgentNotFound { id: cb_id })?;

    let cb_res_pos = cb_bs.liabilities.entry(reserve_inst_id).or_default();
    cb_res_pos.quantity += pi.amount;

    Ok(())
}
fn apply_tga_transfer(state: &mut SimState, pi: &PaymentInstruction) -> Result<(), EffectError> {
    let cb_id = state.financial_system.central_bank.id;
    let gov_id = state.financial_system.government.id;
    if pi.payer == cb_id {
        return apply_central_bank_payment(state, pi);
    }
    let (tga_id, _) = state
        .financial_system
        .find_government_tga_account()
        .ok_or_else(|| EffectError::InvalidState("TGA not found".to_string()))?;

    if pi.payer == gov_id {
        let (payee_account_id, payee_bank) = state
            .financial_system
            .find_agent_liquid_account(&pi.payee)
            .ok_or_else(|| EffectError::InvalidState("Payee account not found".to_string()))?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            gov_id,
            tga_id,
            -pi.amount,
            &PositionSide::Asset,
            None,
        )?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            cb_id,
            tga_id,
            -pi.amount,
            &PositionSide::Liability,
            None,
        )?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            pi.payee,
            payee_account_id,
            pi.amount,
            &PositionSide::Asset,
            None,
        )?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            payee_bank,
            payee_account_id,
            pi.amount,
            &PositionSide::Liability,
            None,
        )?;

        if payee_bank != cb_id {
            let bank_reserves = state
                .financial_system
                .find_bank_reserves_account(&payee_bank)
                .ok_or_else(|| EffectError::InvalidState("Bank reserves not found".to_string()))?;

            StateEffectApplicator::apply_cash_position_adjustment(
                state,
                payee_bank,
                bank_reserves,
                pi.amount,
                &PositionSide::Asset,
                None,
            )?;

            StateEffectApplicator::apply_cash_position_adjustment(
                state,
                cb_id,
                bank_reserves,
                pi.amount,
                &PositionSide::Liability,
                None,
            )?;
        }
    } else if pi.payee == gov_id {
        if pi.payer == cb_id {
            StateEffectApplicator::apply_cash_position_adjustment(
                state,
                gov_id,
                tga_id,
                pi.amount,
                &PositionSide::Asset,
                None,
            )?;
            StateEffectApplicator::apply_cash_position_adjustment(
                state,
                cb_id,
                tga_id,
                pi.amount,
                &PositionSide::Liability,
                None,
            )?;
            return Ok(());
        }

        let (payer_account_id, payer_bank) = state
            .financial_system
            .find_agent_liquid_account(&pi.payer)
            .ok_or_else(|| EffectError::InvalidState("Payer account not found".to_string()))?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            pi.payer,
            payer_account_id,
            -pi.amount,
            &PositionSide::Asset,
            None,
        )?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            payer_bank,
            payer_account_id,
            -pi.amount,
            &PositionSide::Liability,
            None,
        )?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            gov_id,
            tga_id,
            pi.amount,
            &PositionSide::Asset,
            None,
        )?;

        StateEffectApplicator::apply_cash_position_adjustment(
            state,
            cb_id,
            tga_id,
            pi.amount,
            &PositionSide::Liability,
            None,
        )?;

        if payer_bank != cb_id {
            let bank_reserves = state
                .financial_system
                .find_bank_reserves_account(&payer_bank)
                .ok_or_else(|| EffectError::InvalidState("Bank reserves not found".to_string()))?;

            StateEffectApplicator::apply_cash_position_adjustment(
                state,
                payer_bank,
                bank_reserves,
                -pi.amount,
                &PositionSide::Asset,
                None,
            )?;

            StateEffectApplicator::apply_cash_position_adjustment(
                state,
                cb_id,
                bank_reserves,
                -pi.amount,
                &PositionSide::Liability,
                None,
            )?;
        }
    }

    Ok(())
}

pub fn settle_one_payment(state: &mut SimState, payment_id: PaymentId) -> Result<(), EffectError> {
    let payment_idx = state
        .financial_system
        .rtgs
        .pending
        .iter()
        .position(|p| p.id == payment_id)
        .ok_or_else(|| EffectError::InvalidState("Payment not found".to_string()))?;

    let payment = state.financial_system.rtgs.pending[payment_idx].clone();
    apply_cash_movements_immediately(state, &payment)?;

    let settled_payment = state.financial_system.rtgs.pending.remove(payment_idx);
    state.financial_system.rtgs.settled.push(settled_payment);

    Ok(())
}
