use crate::*;
use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use dyn_clone::{clone_trait_object, DynClone};
use std::any::Any;
use std::fmt::Debug;
use std::str::FromStr;
use std::fmt;
use thiserror::Error;

#[typetag::serde(tag = "instrument_details_type")]
pub trait InstrumentDetails: DynClone + Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CashDetails;
#[typetag::serde]
impl InstrumentDetails for CashDetails {
    fn as_any(&self) -> &dyn Any { self }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemandDepositDetails {
    pub interest_rate: f64,
}
#[typetag::serde]
impl InstrumentDetails for DemandDepositDetails {
    fn as_any(&self) -> &dyn Any { self }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavingsDepositDetails {
    pub interest_rate: f64,
}
#[typetag::serde]
impl InstrumentDetails for SavingsDepositDetails {
    fn as_any(&self) -> &dyn Any { self }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CentralBankReservesDetails;
#[typetag::serde]
impl InstrumentDetails for CentralBankReservesDetails {
    fn as_any(&self) -> &dyn Any { self }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BondDetails {
    pub bond_type: BondType,
    pub coupon_rate: f64,
    pub face_value: f64,
    pub maturity_date: NaiveDate,
    pub frequency: usize,
}
#[typetag::serde]
impl InstrumentDetails for BondDetails {
    fn as_any(&self) -> &dyn Any { self }
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
    fn as_any(&self) -> &dyn Any { self }
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
    AAA, AA, A, BBB, BB, B, CCC, CC, C, D,
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
            originated_date: chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            details: Box::new(CashDetails),
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
        None
    }
}