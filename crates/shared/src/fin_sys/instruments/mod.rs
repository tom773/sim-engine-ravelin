use crate::*;
use chrono::NaiveDate;
use dyn_clone::{clone_trait_object, DynClone};
use serde::{Deserialize, Serialize};
use sscanf::RegexRepresentation;
use std::any::Any;
use std::fmt::Debug;
use std::{fmt, str::FromStr};
use thiserror::Error;
use uuid::Uuid;

pub mod bond;
pub use bond::*;
pub mod cash;
pub use cash::*;
pub mod derivative;
pub use derivative::*;
pub mod equity;
pub use equity::*;


#[typetag::serde(tag = "instrument_details_type")]
pub trait InstrumentDetails: DynClone + Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}
clone_trait_object!(InstrumentDetails);


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CashDetails;
#[typetag::serde]
impl InstrumentDetails for CashDetails {
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
pub struct DemandDepositDetails {
    pub interest_rate: f64,
}
#[typetag::serde]
impl InstrumentDetails for DemandDepositDetails {
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


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialInstrument {
    pub id: InstrumentId,
    pub debtor: AgentId,
    pub creditor: AgentId,
    pub principal: f64,
    pub originated_date: chrono::NaiveDate,
    pub details: Box<dyn InstrumentDetails>,
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
    Corporate { spread: f64 },
    Government,
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
#[error("'{0}' is not a valid CreditRating")]
pub struct ParseCreditRatingError(String);

impl FromStr for CreditRating {
    type Err = ParseCreditRatingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AAA" => Ok(CreditRating::AAA), "AA" => Ok(CreditRating::AA), "A" => Ok(CreditRating::A),
            "BBB" => Ok(CreditRating::BBB), "BB" => Ok(CreditRating::BB), "B" => Ok(CreditRating::B),
            "CCC" => Ok(CreditRating::CCC), "CC" => Ok(CreditRating::CC), "C" => Ok(CreditRating::C),
            "D" => Ok(CreditRating::D),
            _ => Err(ParseCreditRatingError(s.to_string())),
        }
    }
}

impl CreditRating {
    pub fn name(&self) -> &'static str {
        match self {
            CreditRating::AAA => "AAA", CreditRating::AA => "AA", CreditRating::A => "A",
            CreditRating::BBB => "BBB", CreditRating::BB => "BB", CreditRating::B => "B",
            CreditRating::CCC => "CCC", CreditRating::CC => "CC", CreditRating::C => "C",
            CreditRating::D => "D",
        }
    }
}

impl RegexRepresentation for CreditRating {
    const REGEX: &'static str = r"AAA|AA|A|BBB|BB|B|CCC|CC|C|D";
}


#[macro_export]
macro_rules! cash {
    ($creditor:expr, $amount:expr, $cb_id:expr, $originated:expr) => {
        $crate::fin_sys::instruments::FinancialInstrument {
            id: $crate::fin_sys::core::InstrumentId(Uuid::new_v4()),
            creditor: $creditor,
            debtor: $cb_id,
            principal: $amount,
            details: Box::new($crate::fin_sys::instruments::CashDetails),
            originated_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! deposit {
    ($depositor:expr, $bank:expr, $amount:expr, $rate:expr, $originated:expr) => {
        $crate::fin_sys::instruments::FinancialInstrument {
            id: $crate::fin_sys::core::InstrumentId(Uuid::new_v4()),
            creditor: $depositor,
            debtor: $bank,
            principal: $amount,
            details: Box::new($crate::fin_sys::instruments::DemandDepositDetails { interest_rate: $rate }),
            originated_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! reserves {
    ($bank:expr, $cb_id:expr, $amount:expr, $originated:expr) => {
        $crate::fin_sys::instruments::FinancialInstrument {
            id: $crate::fin_sys::core::InstrumentId(Uuid::new_v4()),
            creditor: $bank,
            debtor: $cb_id,
            principal: $amount,
            details: Box::new($crate::fin_sys::instruments::CentralBankReservesDetails),
            originated_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! bond {
    ($investor:expr, $issuer:expr, $principal:expr, $coupon_rate:expr, $maturity_date:expr, $face_value:expr, $bond_type:expr, $frequency:expr, $originated:expr) => {
        $crate::fin_sys::instruments::FinancialInstrument {
            id: $crate::fin_sys::core::InstrumentId(Uuid::new_v4()),
            creditor: $investor,
            debtor: $issuer,
            principal: $principal,
            details: Box::new($crate::fin_sys::instruments::BondDetails {
                bond_type: $bond_type,
                coupon_rate: $coupon_rate,
                face_value: $face_value,
                maturity_date: $maturity_date,
                frequency: $frequency,
            }),
            originated_date: $originated,
        }
    };
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

        if self.details.as_any().is::<CashDetails>() {
            return other.details.as_any().is::<CashDetails>();
        }
        if self.details.as_any().is::<CentralBankReservesDetails>() {
            return other.details.as_any().is::<CentralBankReservesDetails>();
        }
        if let Some(self_details) = self.details.as_any().downcast_ref::<DemandDepositDetails>() {
            if let Some(other_details) = other.details.as_any().downcast_ref::<DemandDepositDetails>() {
                return (self_details.interest_rate - other_details.interest_rate).abs() < 0.001;
            }
        }
        false
    }

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
                subtype: Some(format!("rate_{}", (details.interest_rate * 1000.0) as i32)),
            });
        }
        None
    }
}