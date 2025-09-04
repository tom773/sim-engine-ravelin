use crate::prelude::*;
use crate::types::money::{Money, Rate};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Listability {
    Unlisted,
    Listed(VenueType),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VenueType {
    CentralLimitOrderBook,
    PostedRates,
    OTC,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instrument {
    pub id: InstrumentId,
    pub instrument_type: InstrumentType,
    pub instrument_market: InstrumentMarket,
    pub listability: Listability,
}

impl Instrument {
    pub fn new(
        id: InstrumentId,
        instrument_type: InstrumentType,
        instrument_market: InstrumentMarket,
    ) -> Self {
        let listability = match &instrument_type {
            InstrumentType::Cash(details) => match details.cash_type {
                CashType::DemandDeposit | CashType::SavingsDeposit => Listability::Unlisted,
                CashType::Currency | CashType::VaultCash => Listability::Unlisted,
                CashType::CentralBankReserves => Listability::Listed(VenueType::PostedRates),
                CashType::TimeDeposit => Listability::Listed(VenueType::PostedRates),
                CashType::TreasuryGeneralAccount => Listability::Unlisted,
            },
            InstrumentType::Bond(_) => Listability::Listed(VenueType::CentralLimitOrderBook),
            InstrumentType::Equity(_) => Listability::Listed(VenueType::CentralLimitOrderBook),
            InstrumentType::RealAsset(_) => Listability::Unlisted,
            _ => Listability::Listed(VenueType::CentralLimitOrderBook),
        };

        Self {
            id,
            instrument_type,
            instrument_market,
            listability,
        }
    }


    pub fn face_value(&self) -> Option<Money> {
        match &self.instrument_type {
            InstrumentType::Bond(d) => Some(d.face_value),
            InstrumentType::StructuredTranche(d) => Some(d.face_value),
            _ => None,
        }
    }

    pub fn with_listability(mut self, listability: Listability) -> Self {
        self.listability = listability;
        self
    }

    pub fn should_create_order_book(&self) -> bool {
        matches!(self.listability, Listability::Listed(VenueType::CentralLimitOrderBook))
    }
    pub fn type_as_string(&self) -> &'static str {
        match &self.instrument_type {
            InstrumentType::Cash(details) => match details.cash_type {
                CashType::DemandDeposit => "Demand Deposit",
                CashType::SavingsDeposit => "Savings Deposit",
                CashType::TimeDeposit => "Time Deposit",
                CashType::Currency => "Physical Currency",
                CashType::CentralBankReserves => "Central Bank Reserves",
                CashType::VaultCash => "Vault Cash",
                CashType::TreasuryGeneralAccount => "Treasury General Account",
            },
            InstrumentType::Bond(details) => match details.bond_type {
                BondType::Corporate => "Corporate Bond",
                BondType::Government => "Government Bond",
                BondType::InterbankLoan => "Interbank Loan",
            },
            InstrumentType::RealAsset(details) => match details {
                RealAssetType::Inventory { .. } => "Inventory",
                RealAssetType::Property { .. } => "Property",
            },
            InstrumentType::Equity(_) => "Equity",
            InstrumentType::Derivative(_) => "Derivative",
            InstrumentType::StructuredTranche(_) => "Structured Tranche",
            InstrumentType::Repo(_) => "Repurchase Agreement",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InstrumentType {
    Cash(CashDetails),
    Bond(BondDetails),
    RealAsset(RealAssetType),
    Equity(EquityDetails),
    Derivative(DerivativeDetails),
    StructuredTranche(StructuredTrancheDetails),
    Repo(RepoDetails),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CashDetails {
    pub issuer: AgentId,
    pub cash_type: CashType,
    pub currency: Currency,

    pub interest_bps: BasisPoints,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CashType {
    DemandDeposit,
    SavingsDeposit,
    TimeDeposit,
    Currency,
    CentralBankReserves,
    VaultCash,
    TreasuryGeneralAccount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    USD,
    AUD,
    EUR,
    JPY,
    GBP,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstrumentMarket {
    MoneyMarket(MoneyMarketSegment),
    CapitalMarket(CapitalMarketSegment),
    DerivativesMarket(DerivativesMarketSegment),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoneyMarketSegment {
    Interbank,
    SovereignShortTerm,
    CorporateShortTerm,
    Repo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapitalMarketSegment {
    Equity,
    SovereignLongTerm,
    CorporateCredit,
    StructuredFinance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DerivativesMarketSegment {
    Options,
    Futures,
    Swaps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BondType {
    Corporate,
    Government,
    InterbankLoan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CashFlow {
    Zero,
    Fixed,
    Floating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CreditRating {
    AAA,
    AA,
    A,
    BBB,
    BB,
    B,
    CCC,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BondDetails {
    pub bond_type: BondType,
    pub issuer: AgentId,
    pub cash_flow: CashFlow,

    pub coupon_rate_bps: BasisPoints,

    pub face_value: Money,
    pub issue_date: NaiveDate,
    pub maturity_date: NaiveDate,
    pub frequency: u32,
    pub day_count: DayCount,
    pub rating: CreditRating,
    pub last_accrual_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquityDetails {
    pub issuer: AgentId,
    pub outstanding_shares: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivativeDetails {
    pub derivative_type: DerivativeType,
    pub underlying: UnderlyingAsset,
    pub expiry_date: NaiveDate,
    pub contract_size: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum UnderlyingAsset {
    Instrument(InstrumentId),
    Good(GoodId),
    Index(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DerivativeType {
    Option(OptionDetails),
    Future(FutureDetails),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OptionStyle {
    Call,
    Put,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionDetails {
    pub style: OptionStyle,

    pub strike_price: Money,
    pub european: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FutureDetails {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuredTrancheDetails {
    pub issuer: AgentId,
    pub tranche_type: TrancheType,

    pub attachment_point: Rate,
    pub detachment_point: Rate,

    pub face_value: Money,

    pub coupon_rate_bps: BasisPoints,
    pub maturity_date: NaiveDate,
    pub rating: CreditRating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrancheType {
    Senior,
    Mezzanine,
    Equity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoDetails {
    pub lender: AgentId,
    pub borrower: AgentId,
    pub collateral_id: InstrumentId,
    pub collateral_quantity: f64,

    pub loan_amount: Money,

    pub interest_bps: BasisPoints,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,

    pub haircut: Rate,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RealAssetType {
    Inventory {
        owner: AgentId,
        goods: HashMap<GoodId, InventoryItem>,
    },
    Property {
        owner: AgentId,
        address: String,
        sq_ft: u32,

        market_value: Money,
    },
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct InventoryItem {
    pub quantity: f64,
    pub unit_cost: Money,
}

impl BondDetails {
    pub fn original_tenor_years(&self) -> f64 {
        let days = (self.maturity_date - self.issue_date).num_days();
        days as f64 / 365.25
    }

    pub fn remaining_tenor_years(&self, as_of: NaiveDate) -> f64 {
        let days = (self.maturity_date - as_of).num_days();
        (days as f64 / 365.25).max(0.0)
    }

    pub fn tenor_bucket(&self) -> TenorBucket {
        TenorBucket::from_years(self.original_tenor_years())
    }
}