use crate::*;

pub fn can_fund(state: &SimState, pi: &PaymentInstruction) -> Result<bool, EffectError> {
   let cb = state.financial_system.central_bank.id;
   let gov = state.financial_system.government.id;
   if pi.payer == cb && pi.payee == gov {
       return Ok(true); // seigniorage funding
   }
    let (payer_account_id, _) = match state.financial_system.find_agent_liquid_account(&pi.payer) {
        Some(account) => account,
        None => return Ok(false),
    };

    let balance_sheet = state.financial_system.balance_sheets
        .get(&pi.payer)
        .ok_or_else(|| EffectError::AgentNotFound { id: pi.payer })?;

    let current_balance = balance_sheet.assets
        .get(&payer_account_id)
        .map(|pos| pos.quantity)
        .unwrap_or(0.0);

    Ok(current_balance >= pi.amount)
}

pub fn can_use_daylight_credit(state: &SimState, pi: &PaymentInstruction) -> Result<bool, EffectError> {
    let daylight_policy = match &state.financial_system.rtgs_policy.daylight {
        Some(policy) => policy,
        None => return Ok(false),
    };

    // For now, simplified - just check if bank reserves can support it
    let from_bank_reserves = match state.financial_system.find_bank_reserves_account(&pi.from_bank) {
        Some(reserves_id) => reserves_id,
        None => return Ok(false),
    };

    let balance_sheet = match state.financial_system.balance_sheets.get(&pi.from_bank) {
        Some(bs) => bs,
        None => return Ok(false),
    };

    let current_reserves = balance_sheet.assets
        .get(&from_bank_reserves)
        .map(|pos| pos.quantity)
        .unwrap_or(0.0);

    // Allow daylight if reserves + daylight limit >= payment amount
    Ok(current_reserves + daylight_policy.per_bank_limit >= pi.amount)
}