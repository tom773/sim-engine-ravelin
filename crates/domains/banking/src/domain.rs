use sim_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankingDomain {
}

impl BankingDomain {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn can_handle(&self, action: &BankingAction) -> bool {
        match action {
            BankingAction::Deposit { .. } => true,
            BankingAction::Withdraw { .. } => true,
            BankingAction::Transfer { .. } => true,
            BankingAction::PayWages { .. } => true,
            BankingAction::UpdateReserves { .. } => true,
            BankingAction::InjectLiquidity => true,
        }
    }

    pub fn validate(&self, action: &BankingAction, state: &SimState) -> Result<(), String> {
        match action {
            BankingAction::Deposit { agent_id, bank, amount } => {
                self.validate_deposit(*agent_id, *bank, *amount, state)
            }
            BankingAction::Withdraw { agent_id, bank, amount } => {
                self.validate_withdraw(*agent_id, *bank, *amount, state)
            }
            BankingAction::Transfer { from, to, amount } => {
                self.validate_transfer(*from, *to, *amount, state)
            }
            BankingAction::PayWages { agent_id, employee, amount } => {
                self.validate_transfer(*agent_id, *employee, *amount, state)
            }
            BankingAction::UpdateReserves { bank, .. } => self.validate_bank_exists(*bank, state),
            BankingAction::InjectLiquidity => Ok(()),
        }
    }

    fn validate_deposit(&self, depositor: AgentId, bank: AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        Validator::positive_amount(amount)?;
        self.validate_agent_exists(depositor, state)?;
        self.validate_bank_exists(bank, state)?;
        self.validate_sufficient_cash(depositor, amount, state)
    }

    fn validate_withdraw(&self, account_holder: AgentId, bank: AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        Validator::positive_amount(amount)?;
        self.validate_agent_exists(account_holder, state)?;
        self.validate_bank_exists(bank, state)?;
        self.validate_sufficient_deposits(account_holder, bank, amount, state)?;
        self.validate_bank_liquidity(bank, amount, state)
    }

    fn validate_transfer(&self, from: AgentId, to: AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        Validator::positive_amount(amount)?;
        self.validate_agent_exists(from, state)?;
        self.validate_agent_exists(to, state)?;
        self.validate_sufficient_cash(from, amount, state)
    }

    fn validate_agent_exists(&self, agent_id: AgentId, state: &SimState) -> Result<(), String> {
        if state.financial_system.balance_sheets.contains_key(&agent_id) {
            Ok(())
        } else {
            Err(format!("Agent {} does not exist", agent_id.0))
        }
    }

    fn validate_bank_exists(&self, bank_id: AgentId, state: &SimState) -> Result<(), String> {
        if state.agents.banks.contains_key(&bank_id) {
            Ok(())
        } else {
            Err("Target is not a valid commercial bank".to_string())
        }
    }

    fn validate_sufficient_cash(&self, agent_id: AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        let cash = state.financial_system.get_cash_assets(&agent_id);
        if cash >= amount {
            Ok(())
        } else {
            Err(format!("Insufficient cash for agent {}: have ${:.2}, need ${:.2}", agent_id, cash, amount))
        }
    }

    fn validate_sufficient_deposits(&self, account_holder: AgentId, bank: AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        let deposits = state.financial_system.get_deposits_at_bank(&account_holder, &bank);
        if deposits >= amount {
            Ok(())
        } else {
            Err(format!("Insufficient deposits for agent {}: have ${:.2}, need ${:.2}", account_holder, deposits, amount))
        }
    }

    fn validate_bank_liquidity(&self, bank: AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        let liquidity = state.financial_system.get_liquid_assets(&bank);
        if liquidity >= amount {
            Ok(())
        } else {
            Err(format!("Insufficient bank liquidity for {}: have ${:.2}, need ${:.2}", bank, liquidity, amount))
        }
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
    pub fn execute(&self, action: &BankingAction, state: &SimState) -> BankingResult {
        if let Err(error) = self.validate(action, state) {
            return BankingResult {
                success: false,
                effects: vec![],
                errors: vec![error],
            };
        }

        match action {
            BankingAction::Deposit { agent_id, bank, amount } => {
                self.execute_deposit(*agent_id, *bank, *amount, state)
            }
            BankingAction::Withdraw { agent_id, bank, amount } => {
                self.execute_withdraw(*agent_id, *bank, *amount, state)
            }
            BankingAction::Transfer { from, to, amount } => {
                self.execute_transfer(*from, *to, *amount, state)
            }
            BankingAction::PayWages { agent_id, employee, amount } => {
                self.execute_transfer(*agent_id, *employee, *amount, state)
            }
            BankingAction::UpdateReserves { bank, amount_change } => {
                self.execute_update_reserves(*bank, *amount_change, state)
            }
            BankingAction::InjectLiquidity => {
                self.execute_inject_liquidity(state)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct BankingResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl Default for BankingDomain {
    fn default() -> Self {
        Self::new()
    }
}