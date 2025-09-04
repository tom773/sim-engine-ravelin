use crate::*;
use tracing::{Level, event};
use uuid::Uuid;
pub fn run_rtgs(state: &mut SimState) -> Result<(), EffectError> {
    if !state.financial_system.rtgs_policy.enabled {
        return Ok(());
    }

    match state.financial_system.rtgs_policy.mode {
        RtgsMode::PureRTGS => run_pure_rtgs(state),
        RtgsMode::LsmNetting => run_lsm_rtgs(state),
    }
}

fn run_pure_rtgs(state: &mut SimState) -> Result<(), EffectError> {
    let current_tick = state.ticknum;

    loop {
        let mut progressed = false;

        state.financial_system.rtgs.pending.sort_by(|a, b| {
            use PaymentPriority::*;
            let priority_order = |p: &PaymentPriority| match p {
                Urgent => 0,
                Normal => 1,
                Low => 2,
            };

            priority_order(&a.priority).cmp(&priority_order(&b.priority)).then(a.id.cmp(&b.id))
        });

        let mut i = 0;
        while i < state.financial_system.rtgs.pending.len() {
            let pi = state.financial_system.rtgs.pending[i].clone();
            if current_tick < pi.earliest_release_tick {
                i += 1;
                continue;
            }

            if can_fund(state, &pi)? || can_use_daylight_credit(state, &pi)? {
                apply_cash_movements_immediately(state, &pi)?;
                maybe_complete_dvp(state, &pi.context)?;

                let settled_payment = state.financial_system.rtgs.pending.remove(i);
                state.financial_system.rtgs.settled.push(settled_payment);
                progressed = true;
            } else {
                if current_tick >= pi.deadline_tick {
                    let expired_payment = state.financial_system.rtgs.pending.remove(i);
                    state.financial_system.rtgs.rejected.push((expired_payment, "Deadline exceeded".to_string()));
                    progressed = true;
                } else {
                    i += 1;
                }
            }
        }

        if !progressed {
            break;
        }
    }

    Ok(())
}

fn run_lsm_rtgs(state: &mut SimState) -> Result<(), EffectError> {
    bilateral_netting(state)?;

    run_pure_rtgs(state)
}

fn bilateral_netting(state: &mut SimState) -> Result<(), EffectError> {
    use std::collections::HashMap;

    let mut nets: HashMap<(AgentId, AgentId), f64> = HashMap::new();

    for payment in &state.financial_system.rtgs.pending {
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
        if !processed_pairs.contains(&pair) {
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
                context: TransactionContext::GenericTransfer,
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

    StateEffectApplicator::apply_adjust_position(
        state,
        pi.payer,
        payer_account_id,
        -pi.amount,
        &PositionSide::Asset,
        None,
    )?;
    StateEffectApplicator::apply_adjust_position(
        state,
        pi.payee,
        payee_account_id,
        pi.amount,
        &PositionSide::Asset,
        None,
    )?;
    StateEffectApplicator::apply_adjust_position(
        state,
        pi.from_bank,
        payer_account_id,
        -pi.amount,
        &PositionSide::Liability,
        None,
    )?;
    StateEffectApplicator::apply_adjust_position(
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
    if pi.to_bank == cb_id && pi.payee != cb_id {
        let from_bank_reserves = state.financial_system.find_bank_reserves_account(&pi.from_bank)
            .ok_or_else(|| EffectError::InvalidState(format!("From bank {} reserves not found", pi.from_bank)))?;

        let (payer_account_id, _) = state
            .financial_system
            .find_agent_liquid_account(&pi.payer)
            .ok_or_else(|| EffectError::InvalidState("Payer account not found".to_string()))?;

        let (payee_account_id, _) = state
            .financial_system
            .find_agent_liquid_account(&pi.payee)
            .ok_or_else(|| EffectError::InvalidState(format!("Payee {} account not found", &pi.payee)))?;

        StateEffectApplicator::apply_adjust_position(
            state,
            pi.payer,
            payer_account_id,
            -pi.amount,
            &PositionSide::Asset,
            None,
        )?;
        StateEffectApplicator::apply_adjust_position(
            state,
            pi.from_bank,
            payer_account_id,
            -pi.amount,
            &PositionSide::Liability,
            None,
        )?;

        StateEffectApplicator::apply_adjust_position(
            state,
            pi.from_bank,
            from_bank_reserves,
            -pi.amount,
            &PositionSide::Asset,
            None,
        )?;
        StateEffectApplicator::apply_adjust_position(
            state,
            cb_id,
            from_bank_reserves,
            -pi.amount,
            &PositionSide::Liability,
            None,
        )?;

        StateEffectApplicator::apply_adjust_position(
            state,
            pi.payee,
            payee_account_id,
            pi.amount,
            &PositionSide::Asset,
            None,
        )?;
        StateEffectApplicator::apply_adjust_position(
            state,
            cb_id,
            payee_account_id,
            pi.amount,
            &PositionSide::Liability,
            None,
        )?;

        return Ok(());
    }
    let from_bank_reserves = state.financial_system.find_bank_reserves_account(&pi.from_bank)
        .ok_or_else(|| EffectError::InvalidState("From bank reserves not found".to_string()))?;

    let to_bank_reserves = state.financial_system.find_bank_reserves_account(&pi.to_bank)
        .ok_or_else(|| EffectError::InvalidState("To bank reserves not found".to_string()))?;

    let (payer_account_id, _) = state
        .financial_system
        .find_agent_liquid_account(&pi.payer)
        .ok_or_else(|| EffectError::InvalidState("Payer account not found".to_string()))?;

    let (payee_account_id, _) = state
        .financial_system
        .find_agent_liquid_account(&pi.payee)
        .ok_or_else(|| EffectError::InvalidState("Payee account not found".to_string()))?;

    StateEffectApplicator::apply_adjust_position(
        state,
        pi.payer,
        payer_account_id,
        -pi.amount,
        &PositionSide::Asset,
        None,
    )?;
    StateEffectApplicator::apply_adjust_position(
        state,
        pi.from_bank,
        from_bank_reserves,
        -pi.amount,
        &PositionSide::Asset,
        None,
    )?;
    StateEffectApplicator::apply_adjust_position(
        state,
        pi.to_bank,
        to_bank_reserves,
        pi.amount,
        &PositionSide::Asset,
        None,
    )?;
    StateEffectApplicator::apply_adjust_position(
        state,
        pi.payee,
        payee_account_id,
        pi.amount,
        &PositionSide::Asset,
        None,
    )?;
    StateEffectApplicator::apply_adjust_position(
        state,
        pi.from_bank,
        payer_account_id,
        -pi.amount,
        &PositionSide::Liability,
        None,
    )?;
    StateEffectApplicator::apply_adjust_position(
        state,
        pi.to_bank,
        payee_account_id,
        pi.amount,
        &PositionSide::Liability,
        None,
    )?;

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
    maybe_complete_dvp(state, &payment.context)?;

    let settled_payment = state.financial_system.rtgs.pending.remove(payment_idx);
    event!(Level::INFO, "Settled payment: {:?}", settled_payment.clone().id);
    state.financial_system.rtgs.settled.push(settled_payment);

    Ok(())
}

fn maybe_complete_dvp(state: &mut SimState, context: &TransactionContext) -> Result<(), EffectError> {
    if let TransactionContext::TradeSettlement { trade_id } = context {
        complete_dvp_asset_leg(state, *trade_id)
    } else {
        Ok(())
    }
}

pub fn complete_dvp_asset_leg(_state: &mut SimState, _trade_id: Uuid) -> Result<(), EffectError> {
    Ok(())
}