use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type PaymentId = Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PaymentPriority {
    Urgent,
    Normal,
    Low,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentInstruction {
    pub id: PaymentId,
    pub from_bank: AgentId,
    pub to_bank: AgentId,
    pub payer: AgentId,
    pub payee: AgentId,
    pub amount: f64,
    pub context: TransactionContext,
    pub priority: PaymentPriority,
    pub earliest_release_tick: u32,
    pub deadline_tick: u32,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RtgsQueue {
    pub pending: Vec<PaymentInstruction>,
    pub settled: Vec<PaymentInstruction>,
    pub rejected: Vec<(PaymentInstruction, String)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DaylightCreditPolicy {
    pub per_bank_limit: f64,
    pub fee_bps: rust_decimal::Decimal,
    pub collateral_haircut: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RtgsPolicy {
    pub enabled: bool,
    pub mode: RtgsMode,
    pub daylight: Option<DaylightCreditPolicy>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum RtgsMode {
    PureRTGS,
    LsmNetting,
}

impl Default for RtgsPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: RtgsMode::PureRTGS,
            daylight: Some(DaylightCreditPolicy {
                per_bank_limit: 100_000.0,
                fee_bps: rust_decimal::Decimal::new(25, 4),
                collateral_haircut: 0.02,
            }),
        }
    }
}
