use serde::{Deserialize, Serialize};
use crate::*;
use chrono::NaiveDate;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FinancialEffect {
    CreateInstrument(FinancialInstrument),
    UpdateInstrument { id: InstrumentId, new_principal: f64 },
    TransferInstrument { id: InstrumentId, new_creditor: AgentId },
    RemoveInstrument(InstrumentId),
    SwapInstrument { id: InstrumentId, new_debtor: AgentId, new_creditor: AgentId },
    RecordTransaction(Transaction),
    SplitAndTransferInstrument { id: InstrumentId, buyer: AgentId, quantity: u64 },
    AccrueInterest {
        instrument_id: InstrumentId,
        accrued_amount: f64,
        accrual_date: NaiveDate,
    },
    ResetAccruedInterest { instrument_id: InstrumentId },
    TransferFunds { from: AgentId, to: AgentId, amount: f64 },
    DepositFunds { depositor: AgentId, bank: AgentId, amount: f64 },
    WithdrawFunds { account_holder: AgentId, bank: AgentId, amount: f64 },
    PayWages { employer: AgentId, employee: AgentId, amount: f64 },
    InjectLiquidity { recipients: Vec<AgentId>, amount_per_recipient: f64 },
}

impl FinancialEffect {
    pub fn name(&self) -> &'static str {
        match self {
            FinancialEffect::CreateInstrument(_) => "CreateInstrument",
            FinancialEffect::UpdateInstrument { .. } => "UpdateInstrument",
            FinancialEffect::TransferInstrument { .. } => "TransferInstrument",
            FinancialEffect::RemoveInstrument(_) => "RemoveInstrument",
            FinancialEffect::SwapInstrument { .. } => "SwapInstrument",
            FinancialEffect::RecordTransaction(_) => "RecordTransaction",
            FinancialEffect::SplitAndTransferInstrument { .. } => "SplitAndTransferInstrument",
            FinancialEffect::AccrueInterest { .. } => "AccrueInterest",
            FinancialEffect::ResetAccruedInterest { .. } => "ResetAccruedInterest",
            FinancialEffect::TransferFunds { .. } => "TransferFunds",
            FinancialEffect::DepositFunds { .. } => "DepositFunds",
            FinancialEffect::WithdrawFunds { .. } => "WithdrawFunds",
            FinancialEffect::PayWages { .. } => "PayWages",
            FinancialEffect::InjectLiquidity { .. } => "InjectLiquidity",
        }
    }
}


impl StateEffectApplicator {
    pub fn apply_financial_effect(state: &mut SimState, effect: &FinancialEffect) -> Result<(), EffectError> {
        match effect {
            FinancialEffect::CreateInstrument(inst) => state
                .financial_system
                .create_or_consolidate_instrument(inst.clone())
                .map(|_| ())
                .map_err(EffectError::FinancialSystemError),
            
            FinancialEffect::UpdateInstrument { id, new_principal } => state
                .financial_system
                .update_instrument(id, *new_principal)
                .map_err(|e| EffectError::FinancialSystemError(e)),
            
            FinancialEffect::TransferInstrument { id, new_creditor } => state
                .financial_system
                .transfer_instrument(id, *new_creditor)
                .map_err(|e| EffectError::FinancialSystemError(e)),
            
            FinancialEffect::RemoveInstrument(id) => {
                state.financial_system.remove_instrument(id).map_err(|e| EffectError::FinancialSystemError(e))
            }
            
            FinancialEffect::SplitAndTransferInstrument { id, buyer, quantity } => state
                .financial_system
                .split_and_transfer_instrument(id, *buyer, *quantity)
                .map(|_| ())
                .map_err(EffectError::FinancialSystemError),
            
            FinancialEffect::SwapInstrument { id, new_debtor, new_creditor } => state
                .financial_system
                .swap_instrument(id, new_debtor, new_creditor)
                .map_err(|e| EffectError::FinancialSystemError(e)),
            
            FinancialEffect::RecordTransaction(tx) => {
                state.history.transactions.push(tx.clone());
                Ok(())
            }
            
            FinancialEffect::AccrueInterest { instrument_id, accrued_amount, accrual_date } => {
                Self::apply_accrue_interest(state, *instrument_id, *accrued_amount, *accrual_date)
            }
            
            FinancialEffect::ResetAccruedInterest { instrument_id } => {
                Self::apply_reset_accrued_interest(state, *instrument_id)
            }
            
            FinancialEffect::TransferFunds { from, to, amount } => {
                Self::apply_transfer_funds(state, *from, *to, *amount)
            }
            
            FinancialEffect::DepositFunds { depositor, bank, amount } => {
                Self::apply_deposit_funds(state, *depositor, *bank, *amount)
            }
            
            FinancialEffect::WithdrawFunds { account_holder, bank, amount } => {
                Self::apply_withdraw_funds(state, *account_holder, *bank, *amount)
            }
            
            FinancialEffect::PayWages { employer, employee, amount } => {
                Self::apply_transfer_funds(state, *employer, *employee, *amount)
            }
            
            FinancialEffect::InjectLiquidity { recipients, amount_per_recipient } => {
                Self::apply_inject_liquidity(state, recipients, *amount_per_recipient)
            }
        }
    }

    fn apply_transfer_funds(state: &mut SimState, from: AgentId, to: AgentId, amount: f64) -> Result<(), EffectError> {
        let cb_id = state.financial_system.central_bank.id;
        
        let liquid_assets = state.financial_system.get_liquid_assets(&from);
        if liquid_assets < amount {
            return Err(EffectError::TransactionFailure(
                "TransferFunds".to_string(),
                format!("Insufficient liquid assets for agent {}: have ${:.2}, need ${:.2}", from, liquid_assets, amount)
            ));
        }

        let from_bs = state.financial_system.balance_sheets.get(&from)
            .ok_or_else(|| EffectError::AgentNotFound { id: from })?
            .clone(); // Clone to avoid borrow checker issues

        let (cash_id, cash_on_hand) = from_bs
            .assets
            .iter()
            .find(|(_, inst)| inst.details.as_any().is::<CashDetails>())
            .map(|(id, inst)| (Some(*id), inst.principal))
            .unwrap_or((None, 0.0));

        let amount_from_cash = cash_on_hand.min(amount);
        let amount_from_deposits = amount - amount_from_cash;

        if amount_from_cash > 1e-6 {
            if let Some(id) = cash_id {
                let new_cash_principal = cash_on_hand - amount_from_cash;
                if new_cash_principal < 1e-6 {
                    state.financial_system.remove_instrument(&id)
                        .map_err(EffectError::FinancialSystemError)?;
                } else {
                    state.financial_system.update_instrument(&id, new_cash_principal)
                        .map_err(EffectError::FinancialSystemError)?;
                }

                let recipient_cash = if state.agents.banks.contains_key(&to) {
                    reserves!(to, cb_id, amount_from_cash, state.current_date, state.financial_system.central_bank.policy_rate_bps+15.0)
                } else {
                    cash!(to, amount_from_cash, cb_id, state.current_date)
                };
                state.financial_system.create_or_consolidate_instrument(recipient_cash)
                    .map_err(EffectError::FinancialSystemError)?;
            }
        }

        if amount_from_deposits > 1e-6 {
            if let Some((dep_id, dep_inst)) = from_bs.assets.iter()
                .find(|(_, inst)| inst.details.as_any().is::<DemandDepositDetails>()) {
                
                let payer_bank_id = dep_inst.debtor;
                let new_deposit_principal = dep_inst.principal - amount_from_deposits;
                
                if new_deposit_principal < 1e-6 {
                    state.financial_system.remove_instrument(dep_id)
                        .map_err(EffectError::FinancialSystemError)?;
                } else {
                    state.financial_system.update_instrument(dep_id, new_deposit_principal)
                        .map_err(EffectError::FinancialSystemError)?;
                }

                if let Some(payer_bank_bs) = state.financial_system.balance_sheets.get(&payer_bank_id).cloned() {
                    if let Some((res_id, res_inst)) = payer_bank_bs.assets.iter()
                        .find(|(_, i)| i.details.as_any().is::<CentralBankReservesDetails>()) {
                        
                        let new_reserves = res_inst.principal - amount_from_deposits;
                        if new_reserves < 1e-6 {
                            state.financial_system.remove_instrument(res_id)
                                .map_err(EffectError::FinancialSystemError)?;
                        } else {
                            state.financial_system.update_instrument(res_id, new_reserves)
                                .map_err(EffectError::FinancialSystemError)?;
                        }
                    }
                }

                let recipient_bank_id = if state.agents.banks.contains_key(&to) {
                    to
                } else if let Some(c) = state.agents.consumers.get(&to) {
                    c.bank_id
                } else if let Some(f) = state.agents.firms.get(&to) {
                    f.bank_id
                } else {
                    return Err(EffectError::InvalidState(
                        format!("Could not determine bank for recipient {}", to)
                    ));
                };

                let recipient_reserves = reserves!(recipient_bank_id, cb_id, amount_from_deposits, state.current_date, state.financial_system.central_bank.policy_rate_bps+15.0);
                state.financial_system.create_or_consolidate_instrument(recipient_reserves)
                    .map_err(EffectError::FinancialSystemError)?;

                if !state.agents.banks.contains_key(&to) {
                    let bank_spread_bps = state.agents.banks.get(&recipient_bank_id)
                        .map(|b| b.deposit_spread_bps).unwrap_or(-50.0);
                    let policy_rate_bps = state.financial_system.central_bank.policy_rate_bps;
                    let deposit_rate_bps = (policy_rate_bps + bank_spread_bps).max(0.0);

                    let recipient_deposit = deposit!(to, recipient_bank_id, amount_from_deposits, deposit_rate_bps, state.current_date);
                    state.financial_system.create_or_consolidate_instrument(recipient_deposit)
                        .map_err(EffectError::FinancialSystemError)?;
                }
            }
        }

        Ok(())
    }

    fn apply_deposit_funds(state: &mut SimState, depositor: AgentId, bank: AgentId, amount: f64) -> Result<(), EffectError> {
        let cash_assets = state.financial_system.get_cash_assets(&depositor);
        if cash_assets < amount {
            return Err(EffectError::TransactionFailure(
                "DepositFunds".to_string(),
                format!("Insufficient cash for agent {}: have ${:.2}, need ${:.2}", depositor, cash_assets, amount)
            ));
        }

        let cb_id = state.financial_system.central_bank.id;
        
        let bank_spread_bps = state.agents.banks.get(&bank)
            .map(|b| b.deposit_spread_bps).unwrap_or(-50.0);
        let policy_rate_bps = state.financial_system.central_bank.policy_rate_bps;
        let deposit_rate_bps = (policy_rate_bps + bank_spread_bps).max(0.0);
        
        let deposit = deposit!(depositor, bank, amount, deposit_rate_bps, state.current_date);
        state.financial_system.create_or_consolidate_instrument(deposit)
            .map_err(EffectError::FinancialSystemError)?;

        if let Some(depositor_bs) = state.financial_system.balance_sheets.get(&depositor).cloned() {
            if let Some((cash_id, cash_inst)) = depositor_bs.assets.iter()
                .find(|(_, i)| i.details.as_any().is::<CashDetails>()) {
                
                let new_cash = cash_inst.principal - amount;
                if new_cash < 1e-6 {
                    state.financial_system.remove_instrument(cash_id)
                        .map_err(EffectError::FinancialSystemError)?;
                } else {
                    state.financial_system.update_instrument(cash_id, new_cash)
                        .map_err(EffectError::FinancialSystemError)?;
                }
            }
        }

        let reserves = reserves!(bank, cb_id, amount, state.current_date, state.financial_system.central_bank.policy_rate_bps+15.0);
        state.financial_system.create_or_consolidate_instrument(reserves)
            .map_err(EffectError::FinancialSystemError)?;

        Ok(())
    }

    fn apply_withdraw_funds(state: &mut SimState, account_holder: AgentId, bank: AgentId, amount: f64) -> Result<(), EffectError> {
        let deposits_at_bank = state.financial_system.get_deposits_at_bank(&account_holder, &bank);
        if deposits_at_bank < amount {
            return Err(EffectError::TransactionFailure(
                "WithdrawFunds".to_string(),
                format!("Insufficient deposits for agent {}: have ${:.2}, need ${:.2}", account_holder, deposits_at_bank, amount)
            ));
        }

        let bank_liquidity = state.financial_system.get_liquid_assets(&bank);
        if bank_liquidity < amount {
            return Err(EffectError::TransactionFailure(
                "WithdrawFunds".to_string(),
                format!("Insufficient bank liquidity: have ${:.2}, need ${:.2}", bank_liquidity, amount)
            ));
        }

        let cb_id = state.financial_system.central_bank.id;

        if let Some(account_holder_bs) = state.financial_system.balance_sheets.get(&account_holder).cloned() {
            if let Some((deposit_id, deposit)) = account_holder_bs.assets.iter()
                .find(|(_, inst)| inst.debtor == bank && inst.details.as_any().is::<DemandDepositDetails>()) {
                
                let new_principal = deposit.principal - amount;
                if new_principal < 1e-6 {
                    state.financial_system.remove_instrument(deposit_id)
                        .map_err(EffectError::FinancialSystemError)?;
                } else {
                    state.financial_system.update_instrument(deposit_id, new_principal)
                        .map_err(EffectError::FinancialSystemError)?;
                }

                let cash = cash!(account_holder, amount, cb_id, state.current_date);
                state.financial_system.create_or_consolidate_instrument(cash)
                    .map_err(EffectError::FinancialSystemError)?;

                if let Some(bank_bs) = state.financial_system.balance_sheets.get(&bank).cloned() {
                    if let Some((res_id, res_inst)) = bank_bs.assets.iter()
                        .find(|(_, i)| i.details.as_any().is::<CentralBankReservesDetails>()) {
                        
                        let new_reserves = res_inst.principal - amount;
                        if new_reserves < 1e-6 {
                            state.financial_system.remove_instrument(res_id)
                                .map_err(EffectError::FinancialSystemError)?;
                        } else {
                            state.financial_system.update_instrument(res_id, new_reserves)
                                .map_err(EffectError::FinancialSystemError)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn apply_inject_liquidity(state: &mut SimState, recipients: &[AgentId], amount_per_recipient: f64) -> Result<(), EffectError> {
        let cb_id = state.financial_system.central_bank.id;
        
        for recipient in recipients {
            let cash = cash!(*recipient, amount_per_recipient, cb_id, state.current_date);
            state.financial_system.create_or_consolidate_instrument(cash)
                .map_err(EffectError::FinancialSystemError)?;
        }
        
        Ok(())
    }

    fn apply_accrue_interest(state: &mut SimState, instrument_id: InstrumentId, accrued_amount: f64, accrual_date: chrono::NaiveDate) -> Result<(), EffectError> {
        if let Some(instrument) = state.financial_system.instruments.get_mut(&instrument_id) {
            instrument.accrued_interest += accrued_amount;
            instrument.last_accrual_date = accrual_date;
            
            if let Some(creditor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.creditor) {
                if let Some(asset) = creditor_bs.assets.get_mut(&instrument_id) {
                    asset.accrued_interest += accrued_amount;
                    asset.last_accrual_date = accrual_date;
                }
            }
            if let Some(debtor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.debtor) {
                if let Some(liability) = debtor_bs.liabilities.get_mut(&instrument_id) {
                    liability.accrued_interest += accrued_amount;
                    liability.last_accrual_date = accrual_date;
                }
            }
            Ok(())
        } else {
            Err(EffectError::InstrumentNotFound { id: instrument_id })
        }
    }

    fn apply_reset_accrued_interest(state: &mut SimState, instrument_id: InstrumentId) -> Result<(), EffectError> {
        if let Some(instrument) = state.financial_system.instruments.get_mut(&instrument_id) {
            instrument.accrued_interest = 0.0;
            
            if let Some(creditor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.creditor) {
                if let Some(asset) = creditor_bs.assets.get_mut(&instrument_id) {
                    asset.accrued_interest = 0.0;
                }
            }
            if let Some(debtor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.debtor) {
                if let Some(liability) = debtor_bs.liabilities.get_mut(&instrument_id) {
                    liability.accrued_interest = 0.0;
                }
            }
            Ok(())
        } else {
            Err(EffectError::InstrumentNotFound { id: instrument_id })
        }
    }
}