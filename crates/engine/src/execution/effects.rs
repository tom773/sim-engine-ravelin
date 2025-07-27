use super::*;
use axum::extract::State;
use serde::{Serialize, Deserialize};
#[derive(Clone, Debug)]
pub struct ExecutionResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StateEffect {
    CreateInstrument(FinancialInstrument),
    UpdateInstrument { id: InstrumentId, new_principal: f64 },
    TransferInstrument { id: InstrumentId, new_creditor: AgentId },
    RemoveInstrument(InstrumentId),
    SwapInstrument { 
        id: InstrumentId, 
        new_debtor: AgentId, 
        new_creditor: AgentId 
    },
    AddInventory { owner: AgentId, good_id: String, quantity: u32 },
    RemoveInventory { owner: AgentId, good_id: String, quantity: u32 },
    RecordTransaction(Transaction),
    UpdateConsumerIncome { id: AgentId, new_income: f64 },
    UpdateFirmRevenue { id: AgentId, revenue: f64 },
    Hire { firm: AgentId, count: u32 },
    Produce { firm: AgentId, good_id: GoodId, amount: f64 },
}

impl StateEffect {
    pub fn name(&self) -> String {
        match self {
            StateEffect::CreateInstrument(_) => "CreateInstrument".to_string(),
            StateEffect::UpdateInstrument { .. } => "UpdateInstrument".to_string(),
            StateEffect::TransferInstrument { .. } => "TransferInstrument".to_string(),
            StateEffect::SwapInstrument { .. } => "SwapInstrument".to_string(),
            StateEffect::RemoveInstrument(_) => "RemoveInstrument".to_string(),
            StateEffect::AddInventory { .. } => "AddInventory".to_string(),
            StateEffect::RemoveInventory { .. } => "RemoveInventory".to_string(),
            StateEffect::RecordTransaction(_) => "RecordTransaction".to_string(),
            StateEffect::UpdateConsumerIncome { .. } => "UpdateConsumerIncome".to_string(),
            StateEffect::UpdateFirmRevenue { .. } => "UpdateFirmRevenue".to_string(),
            StateEffect::Hire { .. } => "Hire".to_string(),
            StateEffect::Produce { .. } => "Produce".to_string(),
        }
    }
}