use crate::types::*;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionType {
    CashWithdrawal { holder: AgentId, bank: AgentId, amount: f64 },
    CashDeposit { holder: AgentId, bank: AgentId, amount: f64 },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub transaction_type: TransactionType,
    pub amount: f64,
    pub payer: AgentId,
    pub receiver: AgentId,
    pub timestamp: DateTime<Utc>, 
}

impl Transaction {
    pub fn new(
        transaction_type: TransactionType,
        amount: f64,
        payer: AgentId,
        receiver: AgentId,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            transaction_type,
            amount,
            payer,
            receiver,
            timestamp,
        }
    }
}