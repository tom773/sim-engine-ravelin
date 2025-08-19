use crate::*;
use std::collections::HashSet;
use chrono::NaiveDate;
pub trait InstrumentManager {
    fn update_instrument(&mut self, id: &InstrumentId, new_principal: f64) -> Result<(), String>;
    fn create_instrument(&mut self, instrument: FinancialInstrument) -> Result<(), String>;
    fn create_or_consolidate_instrument(&mut self, instrument: FinancialInstrument) -> Result<InstrumentId, String>;
    fn find_consolidatable_instrument(&self, new_inst: &FinancialInstrument) -> Option<InstrumentId>;
    fn remove_instrument(&mut self, id: &InstrumentId) -> Result<(), String>;
    fn transfer_instrument(&mut self, id: &InstrumentId, new_creditor: AgentId) -> Result<(), String>;
    fn swap_instrument(
        &mut self, id: &InstrumentId, new_debtor: &AgentId, new_creditor: &AgentId,
    ) -> Result<(), String>;
    fn split_and_transfer_instrument(
        &mut self,
        instrument_id: &InstrumentId,
        buyer: AgentId,
        quantity_to_transfer: u64,
    ) -> Result<InstrumentId, String>;
    fn pay_interest(
        &mut self, instrument_id: InstrumentId, payment_date: NaiveDate,
    ) -> Result<(), String>;
}

pub trait FinancialStatistics {
    fn m0(&self) -> f64;
    fn m1(&self, bank_ids: &HashSet<AgentId>) -> f64;
    fn m2(&self, bank_ids: &HashSet<AgentId>) -> f64;
    fn all_bank_reserves(&self, bank_ids: &HashSet<AgentId>) -> f64;
    fn all_bank_deposits(&self, bank_ids: &HashSet<AgentId>) -> f64;
    fn all_bank_assets(&self, bank_ids: &HashSet<AgentId>) -> f64;
    fn currency_in_circulation(&self, cb_id: AgentId) -> f64;
}

pub trait Tradable {
    fn check_holdings(&self, agent_id: &AgentId, quantity: f64, fs: &FinancialSystem) -> Result<(), String>;
}

pub trait RatesMarket {
    fn price_to_daily_rate(&self, price: f64) -> f64;
    fn daily_rate_to_annual_bps(&self, daily_rate: f64) -> BasisPoints;
    fn annual_bps_to_daily_rate(&self, annual_bps: BasisPoints) -> f64;
}

pub trait MarketSummaryProvider {
    fn summary(&self) -> MarketSummary;
}

pub trait EconomicAnalytics {
    fn calculate_core_stats(&self) -> CoreEconomicStats;
}