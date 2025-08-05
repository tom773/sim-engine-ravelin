use sim_prelude::*;
use crate::BankingResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankingOperations;

impl BankingOperations {
    pub fn new() -> Self {
        Self
    }

    pub fn execute_deposit(&self, depositor: AgentId, bank: AgentId, amount: f64, state: &SimState) -> BankingResult {
        let mut effects = vec![];

        let deposit_rate = state.financial_system.central_bank.policy_rate - 0.02;
        let deposit = deposit!(depositor, bank, amount, deposit_rate, state.current_date);
        effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(deposit)));

        let transfer_effects = self.create_transfer_effects(depositor, bank, amount, state);
        effects.extend(transfer_effects);

        BankingResult { success: !effects.is_empty(), effects, errors: vec![] }
    }

    pub fn execute_withdraw(&self, account_holder: AgentId, bank: AgentId, amount: f64, state: &SimState) -> BankingResult {
        let mut effects = vec![];

        if let Some((deposit_id, deposit)) = state.financial_system.get_bs_by_id(&account_holder)
            .and_then(|bs| bs.assets.iter().find(|(_, inst)| inst.debtor == bank && inst.details.as_any().is::<DemandDepositDetails>()))
        {
            let new_principal = deposit.principal - amount;
            if new_principal < 1e-6 {
                effects.push(StateEffect::Financial(FinancialEffect::RemoveInstrument(*deposit_id)));
            } else {
                effects.push(StateEffect::Financial(FinancialEffect::UpdateInstrument { id: *deposit_id, new_principal }));
            }

            let transfer_effects = self.create_transfer_effects(bank, account_holder, amount, state);
            effects.extend(transfer_effects);
        }

        BankingResult { success: !effects.is_empty(), effects, errors: vec![] }
    }

    pub fn execute_transfer(&self, from: AgentId, to: AgentId, amount: f64, state: &SimState) -> BankingResult {
        let effects = self.create_transfer_effects(from, to, amount, state);
        BankingResult { success: !effects.is_empty(), effects, errors: vec![] }
    }

    pub fn execute_update_reserves(&self, _bank: AgentId, _amount_change: f64, _state: &SimState) -> BankingResult {
        BankingResult {
            success: false,
            effects: vec![],
            errors: vec!["Reserve update not yet implemented".to_string()],
        }
    }

    pub fn execute_inject_liquidity(&self, state: &SimState) -> BankingResult {
        let effects: Vec<StateEffect> = state.agents.consumers.iter().map(|consumer| {
            let cash = cash!(*consumer.0, 1000.0, state.financial_system.central_bank.id, state.current_date);
            StateEffect::Financial(FinancialEffect::CreateInstrument(cash))
        }).collect();

        BankingResult { success: true, effects, errors: vec![] }
    }

    fn create_transfer_effects(&self, from: AgentId, to: AgentId, amount: f64, state: &SimState) -> Vec<StateEffect> {
        let mut effects = vec![];
        let cb_id = state.financial_system.central_bank.id;

        if let Some((inst_id, inst)) = state.financial_system.get_bs_by_id(&from)
            .and_then(|bs| bs.assets.iter().find(|(_, i)| i.principal >= amount && (i.details.as_any().is::<CashDetails>() || i.details.as_any().is::<CentralBankReservesDetails>())))
        {
            effects.push(StateEffect::Financial(FinancialEffect::UpdateInstrument { id: *inst_id, new_principal: inst.principal - amount }));

            let new_inst = if inst.details.as_any().is::<CashDetails>() {
                cash!(to, amount, cb_id, state.current_date)
            } else {
                reserves!(to, cb_id, amount, state.current_date)
            };
            effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(new_inst)));
        }
        effects
    }
}