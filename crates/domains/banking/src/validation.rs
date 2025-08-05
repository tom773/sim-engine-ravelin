use serde::{Deserialize, Serialize};
use sim_prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankingValidator;

impl BankingValidator {
    pub fn new() -> Self {
        Self
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
}