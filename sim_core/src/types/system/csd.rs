use crate::prelude::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

pub type TradeId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClearingHouse {
    pub csd: CentralSecuritiesDepository,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CentralSecuritiesDepository {
    pub custody_accounts: HashMap<AgentId, CustodyAccount>,
    pub pending_settlements: HashMap<TradeId, SettlementInstruction>,
    pub settlement_history: Vec<CompletedSettlement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyAccount {
    pub owner: AgentId,
    pub holdings: HashMap<InstrumentId, SecurityHolding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityHolding {
    pub available: f64,
    pub reserved: f64, // Earmarked for settlement
    pub pledged: f64,
    pub lent: f64,
    pub borrowed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementInstruction {
    pub instruction_id: Uuid,
    pub trade_id: TradeId,
    pub seller: AgentId,
    pub buyer: AgentId,
    pub instrument_id: InstrumentId,
    pub quantity: f64,
    pub cash_amount: f64,
    pub settlement_date: NaiveDate,
    pub status: SettlementStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedSettlement {
    pub trade_id: TradeId,
    pub settlement_time: NaiveDate,
    pub instrument_id: InstrumentId,
    pub quantity: f64,
    pub seller: AgentId,
    pub buyer: AgentId,
    pub cash_leg_reference: Option<PaymentId>,
}

#[derive(Debug, Error)]
pub enum CSDError {
    #[error("Participant account not found for {0}")]
    ParticipantNotFound(AgentId),
    #[error("Security position not found for instrument {1} in account {0}")]
    SecurityNotFound(AgentId, InstrumentId),
    #[error("Insufficient available securities for trade {0}: available {1}, needed {2}")]
    InsufficientSecurities(TradeId, f64, f64),
    #[error("Settlement instruction not found for trade {0}")]
    InstructionNotFound(TradeId),
    #[error("Instruction mismatch during finalization for trade {0}")]
    InstructionMismatch(TradeId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettlementStatus {
    Pending,
    Matched,
    Settled,
    Failed,
    Cancelled,
}

impl CentralSecuritiesDepository {
    pub fn initialize_government_account(&mut self, gov_id: AgentId) {
        if !self.custody_accounts.contains_key(&gov_id) {
            self.custody_accounts.insert(gov_id, CustodyAccount { owner: gov_id, holdings: HashMap::new() });
        }
    }
    pub fn reserve_securities_for_dvp(&mut self, instruction: SettlementInstruction, fs: &FinancialSystem) -> Result<(), CSDError> {
        let seller_id = instruction.seller;
        let instrument_id = instruction.instrument_id;
        let quantity = instruction.quantity;

        if seller_id == fs.government.id {
            let gov_account = self
                .custody_accounts
                .entry(seller_id)
                .or_insert_with(|| CustodyAccount { owner: seller_id, holdings: HashMap::new() });

            let holding = gov_account.holdings.entry(instrument_id).or_insert_with(|| SecurityHolding::default());

            holding.available += quantity;
            holding.reserved += quantity;

            self.pending_settlements.insert(instruction.trade_id, instruction);
            return Ok(());
        }

        let seller_account =
            self.custody_accounts.get_mut(&seller_id).ok_or_else(|| CSDError::ParticipantNotFound(seller_id))?;

        let holding = seller_account
            .holdings
            .get_mut(&instrument_id)
            .ok_or_else(|| CSDError::SecurityNotFound(seller_id, instrument_id))?;

        if holding.available < quantity {
            return Err(CSDError::InsufficientSecurities(instruction.trade_id, holding.available, quantity));
        }

        holding.available -= quantity;
        holding.reserved += quantity;
        self.pending_settlements.insert(instruction.trade_id, instruction);
        Ok(())
    }

    pub fn finalize_book_entry_transfer(&mut self, trade_id: &TradeId) -> Result<(), CSDError> {
        let instruction = self.pending_settlements.remove(trade_id).ok_or(CSDError::InstructionNotFound(*trade_id))?;

        let seller_account = self
            .custody_accounts
            .get_mut(&instruction.seller)
            .ok_or(CSDError::ParticipantNotFound(instruction.seller))?;
        let seller_holding = seller_account.holdings.entry(instruction.instrument_id).or_default();

        if seller_holding.reserved < instruction.quantity {
            return Err(CSDError::InstructionMismatch(*trade_id));
        }
        seller_holding.reserved -= instruction.quantity;

        let buyer_account = self
            .custody_accounts
            .entry(instruction.buyer)
            .or_insert_with(|| CustodyAccount { owner: instruction.buyer, holdings: HashMap::new() });
        let buyer_holding = buyer_account.holdings.entry(instruction.instrument_id).or_default();
        buyer_holding.available += instruction.quantity;

        Ok(())
    }

    pub fn cancel_security_reservation(&mut self, trade_id: &TradeId) -> Result<(), CSDError> {
        let instruction = self.pending_settlements.remove(trade_id).ok_or(CSDError::InstructionNotFound(*trade_id))?;

        let seller_account = self
            .custody_accounts
            .get_mut(&instruction.seller)
            .ok_or(CSDError::ParticipantNotFound(instruction.seller))?;
        let holding = seller_account.holdings.entry(instruction.instrument_id).or_default();

        if holding.reserved < instruction.quantity {
            return Err(CSDError::InstructionMismatch(*trade_id));
        }

        holding.reserved -= instruction.quantity;
        holding.available += instruction.quantity;

        Ok(())
    }
}
