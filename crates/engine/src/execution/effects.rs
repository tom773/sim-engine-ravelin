use super::*;

#[derive(Clone, Debug)]
pub struct ExecutionResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum StateEffect {
    CreateInstrument(FinancialInstrument),
    UpdateInstrument { id: InstrumentId, new_principal: f64 },
    TransferInstrument { id: InstrumentId, new_creditor: AgentId },
    RemoveInstrument(InstrumentId),
    
    AddInventory { owner: AgentId, good_id: String, quantity: u32 },
    RemoveInventory { owner: AgentId, good_id: String, quantity: u32 },
    
    RecordTransaction(Transaction),
    
    UpdateConsumerIncome { id: AgentId, new_income: f64 },
    UpdateFirmRevenue { id: AgentId, revenue: f64 },
}

impl StateEffect {
    pub fn name(&self) -> String {
        match self {
            StateEffect::CreateInstrument(_) => "CreateInstrument".to_string(),
            StateEffect::UpdateInstrument { .. } => "UpdateInstrument".to_string(),
            StateEffect::TransferInstrument { .. } => "TransferInstrument".to_string(),
            StateEffect::RemoveInstrument(_) => "RemoveInstrument".to_string(),
            StateEffect::AddInventory { .. } => "AddInventory".to_string(),
            StateEffect::RemoveInventory { .. } => "RemoveInventory".to_string(),
            StateEffect::RecordTransaction(_) => "RecordTransaction".to_string(),
            StateEffect::UpdateConsumerIncome { .. } => "UpdateConsumerIncome".to_string(),
            StateEffect::UpdateFirmRevenue { .. } => "UpdateFirmRevenue".to_string(),
        }
    }
}