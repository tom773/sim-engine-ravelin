use crate::prelude::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Instrument = InstrumentCore<InstrumentRuntime>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    USD,
    AUD,
    EUR,
    JPY,
    GBP,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DividendPolicy {
    pub last_dividend: Option<Money>,
    pub next_dividend_date: Option<NaiveDate>,
    pub frequency: Option<PaymentFrequency>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquityProfile {
    pub issuer: AgentId,
    pub share_class: EquityClass,
    pub authorized_shares: Option<u64>,
    pub par_value: Option<Money>,
    pub dividend_policy: Option<DividendPolicy>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EquityClass {
    Common,
    Preferred,
    Restricted,
    Treasury,
    DepositaryReceipt,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquityState {
    pub profile: EquityProfile,
    pub outstanding_shares: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuredTrancheState {
    pub issuer: AgentId,
    pub tranche_label: Option<String>,
    pub tranche_type: StructuredTrancheType,
    pub attachment_point: Rate,
    pub detachment_point: Rate,
    pub face_value: Money,
    pub outstanding_notional: Money,
    pub coupon_rate_bps: BasisPoints,
    pub maturity_date: NaiveDate,
    pub rating: CreditRating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructuredTrancheType {
    Senior,
    Mezzanine,
    Equity,
    Custom,
}

impl StructuredTrancheType {
    pub fn label(&self) -> &'static str {
        match self {
            StructuredTrancheType::Senior => "Senior Tranche",
            StructuredTrancheType::Mezzanine => "Mezzanine Tranche",
            StructuredTrancheType::Equity => "Equity Tranche",
            StructuredTrancheType::Custom => "Structured Tranche",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoState {
    pub lender: AgentId,
    pub borrower: AgentId,
    pub collateral_id: InstrumentId,
    pub collateral_quantity: f64,
    pub cash_principal: Money,
    pub interest_bps: BasisPoints,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub haircut: Rate,
    pub open_term: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RealAssetState {
    Inventory { owner: AgentId, goods: HashMap<GoodId, InventoryItem> },
    Property { owner: AgentId, address: String, sq_ft: u32, market_value: Money },
    Custom { owner: AgentId, description: String, metadata: HashMap<String, String> },
}

impl RealAssetState {
    pub fn label(&self) -> &'static str {
        match self {
            RealAssetState::Inventory { .. } => "Inventory",
            RealAssetState::Property { .. } => "Property",
            RealAssetState::Custom { .. } => "Real Asset",
        }
    }

    pub fn face_value(&self) -> Option<Money> {
        match self {
            RealAssetState::Inventory { goods, .. } => {
                let total = goods.values().fold(Money::ZERO, |acc, item| acc + (item.unit_cost * item.quantity));
                Some(total)
            }
            RealAssetState::Property { market_value, .. } => Some(*market_value),
            RealAssetState::Custom { .. } => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct InventoryItem {
    pub quantity: f64,
    pub unit_cost: Money,
    pub date_acquired: NaiveDate,
}

impl InventoryItem {
    pub fn age_days(&self, as_of: NaiveDate) -> i64 {
        (as_of - self.date_acquired).num_days()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DerivativeContract {
    Option(OptionContract),
    Future(FutureContract),
    Custom { description: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionContract {
    pub style: OptionStyle,
    pub strike_price: Money,
    pub european: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FutureContract {
    pub contract_size: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivativeState {
    pub issuer: AgentId,
    pub counterparty: Option<AgentId>,
    pub contract: DerivativeContract,
    pub underlying: UnderlyingAsset,
    pub trade_date: Option<NaiveDate>,
    pub expiry_date: Option<NaiveDate>,
    pub settlement_date: Option<NaiveDate>,
    pub notional: Option<Money>,
    pub margin_requirement: Option<Money>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OptionStyle {
    Call,
    Put,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum UnderlyingAsset {
    Instrument(InstrumentId),
    Good(GoodId),
    Index(String),
}

impl DerivativeContract {
    pub fn describe(&self) -> &'static str {
        match self {
            DerivativeContract::Option(_) => "Option",
            DerivativeContract::Future(_) => "Future",
            DerivativeContract::Custom { .. } => "Custom Derivative",
        }
    }
}

impl DerivativeState {
    pub fn label(&self) -> &'static str {
        self.contract.describe()
    }
}
