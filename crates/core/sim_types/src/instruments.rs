use crate::*;
use chrono::NaiveDate;
use dyn_clone::{DynClone, clone_trait_object};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::str::FromStr;
use thiserror::Error;
#[typetag::serde(tag = "instrument_details_type")]
pub trait InstrumentDetails: DynClone + Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
clone_trait_object!(InstrumentDetails);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialInstrument {
    pub id: InstrumentId,
    pub debtor: AgentId,
    pub creditor: AgentId,
    pub principal: f64,
    pub originated_date: NaiveDate,
    pub details: Box<dyn InstrumentDetails>,
    pub accrued_interest: f64,
    pub last_accrual_date: NaiveDate,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CashDetails;
#[typetag::serde]
impl InstrumentDetails for CashDetails {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemandDepositDetails {
    pub interest_rate: f64,
}
#[typetag::serde]
impl InstrumentDetails for DemandDepositDetails {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavingsDepositDetails {
    pub interest_rate: f64,
}
#[typetag::serde]
impl InstrumentDetails for SavingsDepositDetails {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CentralBankReservesDetails;
#[typetag::serde]
impl InstrumentDetails for CentralBankReservesDetails {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BondDetails {
    pub bond_type: BondType,
    pub coupon_rate: f64,
    pub face_value: f64,
    pub maturity_date: NaiveDate,
    pub frequency: usize,
    pub tenor: Tenor,
    pub quantity: u64,
}
#[typetag::serde]
impl InstrumentDetails for BondDetails {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoanDetails {
    pub loan_type: LoanType,
    pub interest_rate: f64,
    pub maturity_date: NaiveDate,
    pub collateral: Option<CollateralInfo>,
}
#[typetag::serde]
impl InstrumentDetails for LoanDetails {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BondType {
    Corporate { spread: f64 },
    Government,
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
pub struct CollateralInfo {
    pub collateral_type: String,
    pub value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

impl fmt::Display for CreditRating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Error)]
#[error("Invalid CreditRating string: {0}")]
pub struct ParseCreditRatingError(String);

impl FromStr for CreditRating {
    type Err = ParseCreditRatingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AAA" => Ok(CreditRating::AAA),
            "AA" => Ok(CreditRating::AA),
            "A" => Ok(CreditRating::A),
            "BBB" => Ok(CreditRating::BBB),
            "BB" => Ok(CreditRating::BB),
            "B" => Ok(CreditRating::B),
            "CCC" => Ok(CreditRating::CCC),
            "CC" => Ok(CreditRating::CC),
            "C" => Ok(CreditRating::C),
            "D" => Ok(CreditRating::D),
            _ => Err(ParseCreditRatingError(s.to_string())),
        }
    }
}

impl Default for FinancialInstrument {
    fn default() -> Self {
        Self {
            id: Default::default(),
            debtor: Default::default(),
            creditor: Default::default(),
            principal: 0.0,
            originated_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            details: Box::new(CashDetails),
            accrued_interest: 0.0,
            last_accrual_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConsolidationKey {
    pub creditor: AgentId,
    pub debtor: AgentId,
    pub instrument_type: String,
    pub subtype: Option<String>,
}

pub trait Consolidatable {
    fn consolidation_key(&self) -> Option<ConsolidationKey>;
}

impl Consolidatable for FinancialInstrument {
    fn consolidation_key(&self) -> Option<ConsolidationKey> {
        if self.details.as_any().is::<CashDetails>() {
            return Some(ConsolidationKey {
                creditor: self.creditor,
                debtor: self.debtor,
                instrument_type: "Cash".to_string(),
                subtype: None,
            });
        }
        if self.details.as_any().is::<CentralBankReservesDetails>() {
            return Some(ConsolidationKey {
                creditor: self.creditor,
                debtor: self.debtor,
                instrument_type: "Reserves".to_string(),
                subtype: None,
            });
        }
        if let Some(details) = self.details.as_any().downcast_ref::<DemandDepositDetails>() {
            return Some(ConsolidationKey {
                creditor: self.creditor,
                debtor: self.debtor,
                instrument_type: "DemandDeposit".to_string(),
                subtype: Some(format!("rate_{}", (details.interest_rate * 10000.0) as i32)),
            });
        }
        if let Some(details) = self.details.as_any().downcast_ref::<BondDetails>() {
            return Some(ConsolidationKey {
                creditor: self.creditor,
                debtor: self.debtor,
                instrument_type: "Bond".to_string(),
                subtype: Some(format!("{:?}_{}", details.tenor, (details.coupon_rate * 10000.0) as i32,)),
            });
        }
        None
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

#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RealAssetType {
    RealEstate {
        address: String,
        property_type: String,
    },
    Inventory {
        #[serde_as(as = "HashMap<DisplayFromStr, _>")]
        goods: HashMap<GoodId, InventoryItem>,
    },
    Equipment {
        description: String,
        depreciation_rate: f64,
    },
    IntellectualProperty {
        description: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub id: uuid::Uuid,
    pub date: u32,
    pub qty: f64,
    pub from: AgentId,
    pub to: AgentId,
    pub tx_type: TransactionType,
    pub instrument_id: Option<InstrumentId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionType {
    Deposit { holder: AgentId, bank: AgentId, amount: f64 },
    Withdrawal { holder: AgentId, bank: AgentId, amount: f64 },
    Transfer { from: AgentId, to: AgentId, amount: f64 },
    InterestPayment { payer: AgentId, receiver: AgentId, amount: f64 },
    DividendPayment { payer: AgentId, receiver: AgentId, amount: f64 },
    TaxPayment { payer: AgentId, tax_type: TaxType, period: NaiveDate },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EquityDetails {
    pub shares_outstanding: u64,
    pub par_value: f64,
    pub voting_rights: bool,
    pub dividend_yield: Option<f64>,
}

#[typetag::serde]
impl InstrumentDetails for EquityDetails {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
