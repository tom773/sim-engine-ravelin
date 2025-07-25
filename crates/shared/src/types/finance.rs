use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentId(pub Uuid);

//
// Financial Instruments
//

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstrumentId(pub Uuid);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FinancialInstrument {
    pub id: InstrumentId,
    pub debtor: AgentId,
    pub creditor: AgentId,
    pub instrument_type: InstrumentType,
    pub principal: f64,
    pub interest_rate: f64,
    pub maturity: Option<u32>, // None for perpetual/demand deposits
    pub originated_date: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum InstrumentType {
    CentralBankReserves,
    // All Agents
    Cash,
    DemandDeposit,
    // Covers Consumer Mortgages, Corporate Loans, etc.
    Bond {
        bond_type: BondType,
        coupon_rate: f64,
        face_value: f64,
        rating: CreditRating,
    },
    Loan {
        loan_type: LoanType,
        collateral: Option<CollateralInfo>, // Optional collateral for secured loans
    },
    // Consumer
    SavingsDeposit { notice_period: u32 },
    
}
// 
// Credit 
//

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum LoanType {
    Mortgage,
    Personal,
    Auto,
    Student,
    CreditCard,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BondType {
    Corporate {
        spread: f64,
    },
    Government,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CollateralInfo {
    pub collateral_type: String,
    pub value: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CreditRating {
    AAA,
    AA,
    A,
    BBB,
    BB,
    B,
    CCC,
    CC,
    C,
    D, 
}
impl CreditRating {
    pub fn convert_fico(&self) -> u32 {
        match self {
            CreditRating::AAA => 800,
            CreditRating::AA => 750,
            CreditRating::A => 700,
            CreditRating::BBB => 650,
            CreditRating::BB => 600,
            CreditRating::B => 550,
            CreditRating::CCC => 500,
            CreditRating::CC => 450,
            CreditRating::C => 400,
            CreditRating::D => 300, // Defaulted
        }
    }
}
//
// Real assets (non-financial)
//
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RealAsset {
    pub id: AssetId,
    pub asset_type: RealAssetType,
    pub owner: AgentId,
    pub market_value: f64,
    pub acquired_date: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(pub Uuid);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RealAssetType {
    RealEstate { address: String, property_type: String },
    Inventory { goods: HashMap<String, InventoryItem> },
    Equipment { description: String, depreciation_rate: f64 },
    IntellectualProperty { description: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InventoryItem {
    pub quantity: f64,
    pub unit_cost: f64,
}
