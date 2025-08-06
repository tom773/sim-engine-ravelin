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
}