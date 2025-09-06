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
    pub registered_securities: HashMap<InstrumentId, SecurityInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityInfo {
    pub instrument_type: SecurityType,
    pub issuer: AgentId,
    pub created_date: NaiveDate,
    pub maturity_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityType {
    Bond,
    Equity,
    StructuredProduct,
    Derivative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyAccount {
    pub owner: AgentId,
    pub holdings: HashMap<InstrumentId, SecurityHolding>,
    pub account_type: CustodyAccountType,
    pub opened_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CustodyAccountType {
    House,
    Client,
    Collateral,
    Omnibus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityHolding {
    pub available: f64,
    pub reserved: f64,
    pub pledged: f64,
    pub lent: f64,
    pub borrowed: f64,
}

impl SecurityHolding {
    pub fn total_position(&self) -> f64 {
        self.available + self.reserved + self.pledged + self.lent
    }

    pub fn net_position(&self) -> f64 {
        self.total_position() + self.borrowed
    }
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
    #[error("Security not registered with CSD: {0}")]
    UnregisteredSecurity(InstrumentId),
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
    pub fn new() -> Self {
        Self {
            custody_accounts: HashMap::new(),
            pending_settlements: HashMap::new(),
            settlement_history: Vec::new(),
            registered_securities: HashMap::new(),
        }
    }

    pub fn register_security(
        &mut self, instrument_id: InstrumentId, instrument: &Instrument, issue_date: NaiveDate,
    ) -> Result<(), CSDError> {
        let (security_type, issuer, maturity) = match &instrument.instrument_type {
            InstrumentType::Bond(details) => (SecurityType::Bond, details.issuer, Some(details.maturity_date)),
            InstrumentType::Equity(details) => (SecurityType::Equity, details.issuer, None),
            InstrumentType::StructuredTranche(details) => {
                (SecurityType::StructuredProduct, details.issuer, Some(details.maturity_date))
            }
            InstrumentType::Derivative(_) => (
                SecurityType::Derivative,
                AgentId::default(),
                None,
            ),
            _ => return Ok(()),
        };

        self.registered_securities.insert(
            instrument_id,
            SecurityInfo { instrument_type: security_type, issuer, created_date: issue_date, maturity_date: maturity },
        );

        Ok(())
    }

    pub fn is_security(&self, instrument_id: &InstrumentId) -> bool {
        self.registered_securities.contains_key(instrument_id)
    }

    pub fn open_custody_account(&mut self, agent_id: AgentId, account_type: CustodyAccountType, date: NaiveDate) {
        if !self.custody_accounts.contains_key(&agent_id) {
            self.custody_accounts.insert(
                agent_id,
                CustodyAccount { owner: agent_id, holdings: HashMap::new(), account_type, opened_date: date },
            );
        }
    }

    pub fn credit_securities(
        &mut self, agent_id: AgentId, instrument_id: InstrumentId, quantity: f64,
    ) -> Result<(), CSDError> {
        if !self.is_security(&instrument_id) {
            return Err(CSDError::UnregisteredSecurity(instrument_id));
        }

        let account = self.custody_accounts.entry(agent_id).or_insert_with(|| CustodyAccount {
            owner: agent_id,
            holdings: HashMap::new(),
            account_type: CustodyAccountType::House,
            opened_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        });

        let holding = account.holdings.entry(instrument_id).or_default();

        holding.available += quantity;

        Ok(())
    }

    pub fn debit_securities(
        &mut self, agent_id: AgentId, instrument_id: InstrumentId, quantity: f64,
    ) -> Result<(), CSDError> {
        let account = self.custody_accounts.get_mut(&agent_id).ok_or(CSDError::ParticipantNotFound(agent_id))?;

        let holding =
            account.holdings.get_mut(&instrument_id).ok_or(CSDError::SecurityNotFound(agent_id, instrument_id))?;

        if holding.available < quantity {
            return Err(CSDError::InsufficientSecurities(
                Uuid::new_v4(),
                holding.available,
                quantity,
            ));
        }

        holding.available -= quantity;

        if holding.total_position() == 0.0 && holding.borrowed == 0.0 {
            account.holdings.remove(&instrument_id);
        }

        Ok(())
    }

    pub fn get_position(&self, agent_id: &AgentId, instrument_id: &InstrumentId) -> Option<f64> {
        self.custody_accounts.get(agent_id)?.holdings.get(instrument_id).map(|h| h.total_position())
    }

    pub fn get_all_positions(&self, agent_id: &AgentId) -> HashMap<InstrumentId, f64> {
        self.custody_accounts
            .get(agent_id)
            .map(|account| {
                account
                    .holdings
                    .iter()
                    .map(|(id, holding)| (*id, holding.total_position()))
                    .filter(|(_, qty)| *qty > 1e-9)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn reserve_securities_for_dvp(
        &mut self,
        instruction: SettlementInstruction,
        government_id: AgentId,
        _instrument_name: &str,
        current_date: NaiveDate,
    ) -> Result<(), CSDError> {
        let seller_id = instruction.seller;
        let instrument_id = instruction.instrument_id;
        let quantity = instruction.quantity;

        if seller_id == government_id && !self.custody_accounts.contains_key(&seller_id) {
            self.open_custody_account(seller_id, CustodyAccountType::House, current_date);
        }

        let seller_account =
            self.custody_accounts.get_mut(&seller_id).ok_or_else(|| CSDError::ParticipantNotFound(seller_id))?;

        let holding = seller_account.holdings.entry(instrument_id).or_default();

        if seller_id == government_id && holding.total_position() < quantity {
            let shortfall = quantity - holding.total_position();
            holding.available += shortfall;
        }

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

        if !self.custody_accounts.contains_key(&instruction.buyer) {
            self.open_custody_account(instruction.buyer, CustodyAccountType::House, instruction.settlement_date);
        }

        let buyer_account = self
            .custody_accounts
            .get_mut(&instruction.buyer)
            .ok_or(CSDError::ParticipantNotFound(instruction.buyer))?;

        let buyer_holding = buyer_account.holdings.entry(instruction.instrument_id).or_default();

        buyer_holding.available += instruction.quantity;


        self.settlement_history.push(CompletedSettlement {
            trade_id: *trade_id,
            settlement_time: instruction.settlement_date,
            instrument_id: instruction.instrument_id,
            quantity: instruction.quantity,
            seller: instruction.seller,
            buyer: instruction.buyer,
            cash_leg_reference: None,
        });

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

    pub fn initialize_government_account(&mut self, gov_id: AgentId) {
        self.open_custody_account(gov_id, CustodyAccountType::House, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
    }
}
