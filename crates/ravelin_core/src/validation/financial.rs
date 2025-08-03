use crate::*;
use super::generic::Validator;

pub struct FinancialValidator<'a> {
    fs: &'a FinancialSystem,
}

impl<'a> FinancialValidator<'a> {
    pub fn new(fs: &'a FinancialSystem) -> Self {
        Self { fs }
    }
    
    pub fn validate_deposit(&self, depositor: &AgentId, bank: &AgentId, amount: f64) -> Result<(), String> {
        Validator::positive_amount(amount)?;
        
        self.ensure_has_balance_sheet(depositor)?;
        self.ensure_is_bank(bank)?;
        self.ensure_sufficient_cash(depositor, amount)?;
        
        Ok(())
    }
    
    pub fn validate_withdraw(&self, account_holder: &AgentId, bank: &AgentId, amount: f64) -> Result<(), String> {
        Validator::positive_amount(amount)?;
        
        self.ensure_has_balance_sheet(account_holder)?;
        self.ensure_is_bank(bank)?;
        self.ensure_sufficient_deposits(account_holder, bank, amount)?;
        self.ensure_bank_liquidity(bank, amount)?;
        
        Ok(())
    }
    
    pub fn validate_loan(&self, lender: &AgentId, borrower: &AgentId, amount: f64, interest_rate: f64) -> Result<(), String> {
        Validator::positive_amount(amount)?;
        Validator::percentage(interest_rate / 10000.0)?; // Convert basis points to percentage
        
        self.ensure_has_balance_sheet(lender)?;
        self.ensure_has_balance_sheet(borrower)?;
        self.ensure_sufficient_liquidity(lender, amount)?;
        
        self.check_debt_to_income_ratio(borrower, amount)?;
        
        Ok(())
    }
    
    pub fn ensure_has_balance_sheet(&self, agent_id: &AgentId) -> Result<(), String> {
        if self.fs.balance_sheets.contains_key(agent_id) {
            Ok(())
        } else {
            Err(format!("Agent {} does not have a balance sheet", &agent_id.0.to_string()[..8]))
        }
    }
    
    fn ensure_is_bank(&self, bank_id: &AgentId) -> Result<(), String> {
        if self.fs.commercial_banks.contains_key(bank_id) {
            Ok(())
        } else {
            Err("Target is not a valid commercial bank".to_string())
        }
    }
    
    pub fn ensure_sufficient_cash(&self, agent_id: &AgentId, amount: f64) -> Result<(), String> {
        let cash = self.fs.get_cash_assets(agent_id);
        if cash >= amount {
            Ok(())
        } else {
            Err(format!("Insufficient cash: ${:.2} < ${:.2}", cash, amount))
        }
    }
    
    fn ensure_sufficient_deposits(&self, account_holder: &AgentId, bank: &AgentId, amount: f64) -> Result<(), String> {
        let deposits = self.fs.get_deposits_at_bank(account_holder, bank);
        if deposits >= amount {
            Ok(())
        } else {
            Err(format!("Insufficient deposits: ${:.2} < ${:.2}", deposits, amount))
        }
    }
    
    fn ensure_bank_liquidity(&self, bank: &AgentId, amount: f64) -> Result<(), String> {
        let liquidity = self.fs.liquidity(bank);
        if liquidity >= amount {
            Ok(())
        } else {
            Err(format!("Bank has insufficient liquidity: ${:.2} < ${:.2}", liquidity, amount))
        }
    }
    
    fn ensure_sufficient_liquidity(&self, agent_id: &AgentId, amount: f64) -> Result<(), String> {
        let liquidity = self.fs.liquidity(agent_id);
        if liquidity >= amount {
            Ok(())
        } else {
            Err(format!("Insufficient liquidity: ${:.2} < ${:.2}", liquidity, amount))
        }
    }
    
    fn check_debt_to_income_ratio(&self, borrower: &AgentId, new_debt: f64) -> Result<(), String> {
        let current_debt = self.fs.get_total_liabilities(borrower);
        let total_debt = current_debt + new_debt;
        
        let assets = self.fs.get_total_assets(borrower);
        if total_debt > assets * 0.8 {  // 80% leverage ratio
            Err(format!("Would exceed maximum leverage ratio"))
        } else {
            Ok(())
        }
    }
}