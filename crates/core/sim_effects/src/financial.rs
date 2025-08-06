use serde::{Deserialize, Serialize};
use sim_types::*;
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
        }
    }
}