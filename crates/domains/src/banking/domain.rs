use serde::{Deserialize, Serialize};
use sim_core::*;
use sim_macros::SimDomain;

#[derive(Clone, Debug, Serialize, Deserialize, SimDomain)]
pub struct BankingDomain {}

#[derive(Debug, Clone)]
pub struct BankingResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl BankingDomain {
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&self, action: &BankingAction, state: &SimState) -> BankingResult {
        if let Err(error) = self.basic_validate(action, state) {
            return BankingResult { success: false, effects: vec![], errors: vec![error] };
        }

        match action {
            BankingAction::Deposit { agent_id, bank, amount } => self.execute_deposit(*agent_id, *bank, *amount),
            BankingAction::Withdraw { agent_id, bank, amount } => self.execute_withdraw(*agent_id, *bank, *amount),
            BankingAction::Transfer { from, to, amount } => self.execute_transfer(*from, *to, *amount),
            BankingAction::PayWages { agent_id, employee, amount } => {
                self.execute_pay_wages(*agent_id, *employee, *amount)
            }
            BankingAction::UpdateReserves { bank: _, amount_change: _ } => {
                BankingResult {
                    success: false,
                    effects: vec![],
                    errors: vec!["Reserve updates not yet implemented with semantic effects".to_string()],
                }
            }
            BankingAction::InjectLiquidity => self.execute_inject_liquidity(state),
        }
    }

    fn basic_validate(&self, action: &BankingAction, state: &SimState) -> Result<(), String> {
        match action {
            BankingAction::Deposit { agent_id, bank, amount }
            | BankingAction::Withdraw { agent_id , bank, amount } => {
                Validator::positive_amount(*amount)?;
                self.validate_agent_exists(*agent_id, state)?;
                self.validate_bank_exists(*bank, state)?;
                Ok(())
            }
            BankingAction::Transfer { from, to, amount }
            | BankingAction::PayWages { agent_id: from, employee: to, amount } => {
                Validator::positive_amount(*amount)?;
                self.validate_agent_exists(*from, state)?;
                self.validate_agent_exists(*to, state)?;
                Ok(())
            }
            BankingAction::UpdateReserves { bank, amount_change: _ } => self.validate_bank_exists(*bank, state),
            BankingAction::InjectLiquidity => Ok(()),
        }
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

    pub fn execute_deposit(&self, depositor: AgentId, bank: AgentId, amount: f64) -> BankingResult {
        let effect = StateEffect::Financial(FinancialEffect::DepositFunds { depositor, bank, amount });
        BankingResult { success: true, effects: vec![effect], errors: vec![] }
    }

    pub fn execute_withdraw(&self, account_holder: AgentId, bank: AgentId, amount: f64) -> BankingResult {
        let effect = StateEffect::Financial(FinancialEffect::WithdrawFunds { account_holder, bank, amount });
        BankingResult { success: true, effects: vec![effect], errors: vec![] }
    }

    pub fn execute_transfer(&self, from: AgentId, to: AgentId, amount: f64) -> BankingResult {
        let effect = StateEffect::Financial(FinancialEffect::TransferFunds { from, to, amount });
        BankingResult { success: true, effects: vec![effect], errors: vec![] }
    }

    pub fn execute_pay_wages(&self, employer: AgentId, employee: AgentId, amount: f64) -> BankingResult {
        let effect = StateEffect::Financial(FinancialEffect::PayWages { employer, employee, amount });
        BankingResult { success: true, effects: vec![effect], errors: vec![] }
    }

    pub fn execute_inject_liquidity(&self, state: &SimState) -> BankingResult {
        let recipients: Vec<AgentId> = state.agents.consumers.keys().cloned().collect();
        let amount_per_recipient = 1000.0;

        let effect = StateEffect::Financial(FinancialEffect::InjectLiquidity { recipients, amount_per_recipient });
        BankingResult { success: true, effects: vec![effect], errors: vec![] }
    }
}
