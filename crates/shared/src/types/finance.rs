use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentId(pub Uuid);


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
    pub maturity: Option<u32>,
    pub originated_date: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum InstrumentType {
    CentralBankReserves,
    Cash,
    DemandDeposit,
    Bond {
        bond_type: BondType,
        coupon_rate: f64,
        face_value: f64,
        rating: CreditRating,
    },
    Loan {
        loan_type: LoanType,
        collateral: Option<CollateralInfo>,
    },
    SavingsDeposit { notice_period: u32 },
    
}

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
            CreditRating::D => 300,
        }
    }
}
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub date: u32,
    pub qty: f64,
    pub from: AgentId,
    pub to: AgentId,
    pub tx_type: TransactionType,
    pub instrument_id: Option<InstrumentId>,
}
impl Transaction {
    pub fn new(tx_type: TransactionType, inst: InstrumentId, from: AgentId, to: AgentId, amount: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            date: chrono::Utc::now().timestamp() as u32,
            from,
            to,
            qty: amount,
            tx_type,
            instrument_id: Some(inst),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionType {
    Deposit {
        holder: AgentId,
        bank: AgentId,
        amount: f64,
    },
    Withdrawal{
        holder: AgentId,
        bank: AgentId,
        amount: f64,
    },
    Transfer {
        from: AgentId,
        to: AgentId,
        amount: f64,
    },
    InterestPayment
}

#[macro_export]
macro_rules! cash {
    ($creditor:expr, $amount:expr, $cb_id:expr, $originated:expr) => {
        FinancialInstrument {
            id: InstrumentId(Uuid::new_v4()),
            creditor: $creditor,
            debtor: $cb_id,
            principal: $amount,
            interest_rate: 0.0,
            maturity: None,
            instrument_type: InstrumentType::Cash,
            originated_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! deposit {
    ($depositor:expr, $bank:expr, $amount:expr, $rate:expr, $originated:expr) => {
        FinancialInstrument {
            id: InstrumentId(Uuid::new_v4()),
            creditor: $depositor,
            debtor: $bank,
            principal: $amount,
            interest_rate: $rate,
            maturity: None,
            instrument_type: InstrumentType::DemandDeposit,
            originated_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! reserves {
    ($bank:expr, $cb_id:expr, $amount:expr, $rate:expr, $originated:expr) => {
        FinancialInstrument {
            id: InstrumentId(Uuid::new_v4()),
            creditor: $bank,
            debtor: $cb_id,
            principal: $amount,
            interest_rate: $rate,
            maturity: None,
            instrument_type: InstrumentType::CentralBankReserves,
            originated_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! loan {
    ($lender:expr, $borrower:expr, $amount:expr, $rate:expr, $maturity:expr, $loan_type:expr, $originated:expr) => {
        FinancialInstrument {
            id: InstrumentId(Uuid::new_v4()),
            creditor: $lender,
            debtor: $borrower,
            principal: $amount,
            interest_rate: $rate,
            maturity: Some($maturity),
            instrument_type: InstrumentType::Loan {
                loan_type: $loan_type,
                collateral: None,
            },
            originated_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! bond {
    ($investor:expr, $issuer:expr, $principal:expr, $coupon_rate:expr, $maturity:expr, $face_value:expr, $rating:expr, $originated:expr) => {
        FinancialInstrument {
            id: InstrumentId(Uuid::new_v4()),
            creditor: $investor,
            debtor: $issuer,
            principal: $principal,
            interest_rate: $coupon_rate,
            maturity: Some($maturity),
            instrument_type: InstrumentType::Bond {
                bond_type: BondType::Corporate { spread: 0.0 },
                coupon_rate: $coupon_rate,
                face_value: $face_value,
                rating: $rating,
            },
            originated_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! create_instrument {
    ($fs:expr, $macro_name:ident, $($args:expr),*) => {{
        let instrument = $macro_name!($($args),*);
        $fs.create_instrument(instrument)
    }};
}

pub trait Consolidatable {
    fn can_consolidate_with(&self, other: &FinancialInstrument) -> bool;
    fn consolidation_key(&self) -> Option<ConsolidationKey>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConsolidationKey {
    pub creditor: AgentId,
    pub debtor: AgentId,
    pub instrument_type: String,
    pub subtype: Option<String>,
}

impl Consolidatable for FinancialInstrument {
    fn can_consolidate_with(&self, other: &FinancialInstrument) -> bool {
        if self.creditor != other.creditor || self.debtor != other.debtor {
            return false;
        }
        
        match (&self.instrument_type, &other.instrument_type) {
            (InstrumentType::Cash, InstrumentType::Cash) => true,
            
            (InstrumentType::CentralBankReserves, InstrumentType::CentralBankReserves) => true,
            
            (InstrumentType::DemandDeposit, InstrumentType::DemandDeposit) => {
                (self.interest_rate - other.interest_rate).abs() < 0.001
            }
            
            (InstrumentType::SavingsDeposit { notice_period: p1 }, 
             InstrumentType::SavingsDeposit { notice_period: p2 }) => {
                p1 == p2 && (self.interest_rate - other.interest_rate).abs() < 0.001
            }
            
            (InstrumentType::Loan { .. }, InstrumentType::Loan { .. }) => false,
            
            (InstrumentType::Bond { .. }, InstrumentType::Bond { .. }) => false,
            
            _ => false,
        }
    }
    
    fn consolidation_key(&self) -> Option<ConsolidationKey> {
        let key = match &self.instrument_type {
            InstrumentType::Cash => Some(ConsolidationKey {
                creditor: self.creditor.clone(),
                debtor: self.debtor.clone(),
                instrument_type: "Cash".to_string(),
                subtype: None,
            }),
            
            InstrumentType::CentralBankReserves => Some(ConsolidationKey {
                creditor: self.creditor.clone(),
                debtor: self.debtor.clone(),
                instrument_type: "Reserves".to_string(),
                subtype: None,
            }),
            
            InstrumentType::DemandDeposit => Some(ConsolidationKey {
                creditor: self.creditor.clone(),
                debtor: self.debtor.clone(),
                instrument_type: "DemandDeposit".to_string(),
                subtype: Some(format!("rate_{}", (self.interest_rate * 1000.0) as i32)),
            }),
            
            InstrumentType::SavingsDeposit { notice_period } => Some(ConsolidationKey {
                creditor: self.creditor.clone(),
                debtor: self.debtor.clone(),
                instrument_type: "SavingsDeposit".to_string(),
                subtype: Some(format!("notice_{}_rate_{}", notice_period, (self.interest_rate * 1000.0) as i32)),
            }),
            
            _ => None,
        };
        
        key
    }
}