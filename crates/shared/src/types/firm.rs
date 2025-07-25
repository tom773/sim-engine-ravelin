use crate::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Firm {
    pub id: AgentId,
    pub bank_id: AgentId,
    pub name: String,
}

impl Firm {
    pub fn new(id: AgentId, bank_id: AgentId, name: String) -> Self {
        Self {
            id,
            name,
            bank_id,
        }
    }
}