use crate::*;
use chrono::NaiveDate;
use std::collections::HashSet;

pub trait BalanceSheetQuery {
    fn get_bs_by_id(&self, agent_id: &AgentId) -> Option<&BalanceSheet>;
    fn get_bs_mut_by_id(&mut self, agent_id: &AgentId) -> Option<&mut BalanceSheet>;
    fn get_total_assets(&self, agent_id: &AgentId) -> f64;
    fn get_cash_assets(&self, agent_id: &AgentId) -> f64;
    fn get_total_liabilities(&self, agent_id: &AgentId) -> f64;
    fn get_liquid_assets(&self, agent_id: &AgentId) -> f64;
    fn get_deposits_at_bank(&self, agent_id: &AgentId, bank_id: &AgentId) -> f64;
    fn liquidity(&self, agent_id: &AgentId) -> f64;
    fn get_total_deposits(&self, agent_id: &AgentId) -> f64;
    fn get_bank_reserves(&self, agent_id: &AgentId) -> Option<f64>;
}

pub trait InstrumentManager {
    fn create_instrument(&mut self, instrument: FinancialInstrument) -> Result<(), String>;
    fn transfer_instrument(&mut self, instrument_id: &InstrumentId, new_creditor: AgentId) -> Result<(), String>;
    fn find_consolidatable_instrument(&self, new_inst: &FinancialInstrument) -> Option<InstrumentId>;
    fn create_or_consolidate_instrument(&mut self, instrument: FinancialInstrument) -> Result<InstrumentId, String>;
    fn update_instrument(&mut self, id: &InstrumentId, new_principal: f64) -> Result<(), String>;
    fn remove_instrument(&mut self, id: &InstrumentId) -> Result<(), String>;
    fn swap_instrument(&mut self, id: &InstrumentId, new_debtor: &AgentId, new_creditor: &AgentId) -> Result<(), String>;
    fn split_and_transfer_instrument(&mut self, instrument_id: &InstrumentId, buyer: AgentId, quantity_to_transfer: u64) -> Result<InstrumentId, String>;
    fn pay_interest(&mut self, instrument_id: InstrumentId, payment_date: NaiveDate) -> Result<(), String>;
}

pub trait FinancialStatistics {
    fn m0(&self) -> f64;
    fn m1(&self, bank_ids: &HashSet<AgentId>) -> f64;
    fn m2(&self, bank_ids: &HashSet<AgentId>) -> f64;
    fn all_bank_assets(&self, bank_ids: &HashSet<AgentId>) -> f64;
    fn all_bank_reserves(&self, bank_ids: &HashSet<AgentId>) -> f64;
    fn all_bank_deposits(&self, bank_ids: &HashSet<AgentId>) -> f64;
    fn currency_in_circulation(&self, cb_id: AgentId) -> f64;
}

pub trait EconomicAnalytics {
    fn calculate_core_stats(&self) -> CoreEconomicStats;
}