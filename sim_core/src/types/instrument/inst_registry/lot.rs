use crate::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentLot {
    pub id: InstrumentId,  // Uses existing InstrumentId
    pub series_id: SeriesId,
    pub lot_type: LotType,
    pub quantity: LotQuantity,
    pub creation_date: NaiveDate,
    pub status: LotStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LotType {
    Fungible {
        lot_size: f64,
    },
    
    LoanDrawdown {
        draw_id: Uuid,
        outstanding_principal: Money,
        next_payment_date: NaiveDate,
    },
    
    AccountInstance {
        account_number: String,
        current_balance: Money,
    },
    
    Tranche {
        tranche_name: String,
        subordination_level: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LotQuantity {
    Units(f64),           // For securities
    Notional(Money),      // For loans, deposits
    Shares(u64),          // For equity
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LotStatus {
    Active,
    Frozen,
    PendingRedemption,
    Redeemed,
    Cancelled,
}
