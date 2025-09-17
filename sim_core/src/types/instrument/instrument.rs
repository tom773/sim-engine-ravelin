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
    pub fn new(id: InstrumentId, instrument_type: InstrumentType, instrument_market: InstrumentMarket) -> Self {
        let listability = match &instrument_type {
            InstrumentType::Cash(details) => match details.cash_type {
                CashType::DemandDeposit | CashType::SavingsDeposit => Listability::Unlisted,
                CashType::Currency | CashType::VaultCash => Listability::Unlisted,
                CashType::CentralBankReserves => Listability::Listed(VenueType::PostedRates),
                CashType::TimeDeposit => Listability::Listed(VenueType::PostedRates),
                CashType::TreasuryGeneralAccount => Listability::Unlisted,
            },
            InstrumentType::Debt(debt) => debt.listability(),
            InstrumentType::Equity(_) => Listability::Listed(VenueType::CentralLimitOrderBook),
            InstrumentType::RealAsset(_) => Listability::Unlisted,
            _ => Listability::Listed(VenueType::CentralLimitOrderBook),
        };

        Self { id, instrument_type, instrument_market, listability }
    }

    pub fn with_listability(mut self, listability: Listability) -> Self {
        self.listability = listability;
        self
    }

    pub fn should_create_order_book(&self) -> bool {
        matches!(self.listability, Listability::Listed(VenueType::CentralLimitOrderBook))
    }

    pub fn face_value(&self) -> Option<Money> {
        match &self.instrument_type {
            InstrumentType::Debt(debt) => match debt {
                DebtInstrument::Bond(d) => Some(d.face_value),
                DebtInstrument::Loan(d) => Some(d.principal),
                DebtInstrument::Consumer(c) => match c {
                    ConsumerDebt::ResidentialMortgage(l)
                    | ConsumerDebt::AutoLoan(l)
                    | ConsumerDebt::PersonalLoan(l) => Some(l.principal),
                    | ConsumerDebt::CreditCard(cl) => Some(cl.commitment_amount),
                    | ConsumerDebt::StudentLoan(l) => Some(l.principal),
                },
                _ => None,
            },
            InstrumentType::StructuredTranche(d) => Some(d.face_value),
            _ => None,
        }
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
            InstrumentType::Debt(debt) => match debt {
                DebtInstrument::Bond(details) => match details.bond_type {
                    BondType::Corporate => "Corporate Bond",
                    BondType::Government => "Government Bond",
                    BondType::InterbankLoan => "Interbank Loan",
                },
                DebtInstrument::Loan(_) => "Loan",
                DebtInstrument::Consumer(c) => match c {
                    ConsumerDebt::ResidentialMortgage(_) => "Residential Mortgage",
                    ConsumerDebt::AutoLoan(_) => "Auto Loan",
                    ConsumerDebt::PersonalLoan(_) => "Personal Loan",
                    ConsumerDebt::CreditCard(_) => "Credit Card",
                    ConsumerDebt::StudentLoan(_) => "Student Loan",
                },
                DebtInstrument::CreditLine(_) => "Credit Facility",
                DebtInstrument::TradeCredit(_) => "Trade Credit",
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
    Debt(DebtInstrument),
    RealAsset(RealAssetType),
    Equity(EquityDetails),
    Derivative(DerivativeDetails),
    StructuredTranche(StructuredTrancheDetails),
    Repo(RepoDetails),
}
impl InstrumentType {
    pub fn as_bond(&self) -> Option<&BondDetails> {
        match self {
            InstrumentType::Debt(DebtInstrument::Bond(b)) => Some(b),
            _ => None,
        }
    }

    pub fn as_loan(&self) -> Option<&LoanDetails> {
        match self {
            InstrumentType::Debt(DebtInstrument::Loan(l)) => Some(l),
            _ => None,
        }
    }

    pub fn as_debt(&self) -> Option<&DebtInstrument> {
        match self {
            InstrumentType::Debt(d) => Some(d),
            _ => None,
        }
    }
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
    Unlisted
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
    Inventory { owner: AgentId, goods: HashMap<GoodId, InventoryItem> },
    Property { owner: AgentId, address: String, sq_ft: u32, market_value: Money },
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct InventoryItem {
    pub quantity: f64,
    pub unit_cost: Money,
    pub date_acquired: NaiveDate,
}

impl InventoryItem {
    pub fn age(&self, as_of: NaiveDate) -> i64 {
        (as_of - self.date_acquired).num_days()
    }
}

impl BondDetails {
    pub fn original_tenor_years(&self) -> f64 {
        self.day_count.year_fraction(self.issue_date, self.maturity_date)
    }

    pub fn remaining_tenor_years(&self, as_of: NaiveDate) -> f64 {
        self.day_count.year_fraction(as_of, self.maturity_date).max(0.0)
    }

    pub fn tenor_bucket(&self) -> TenorBucket {
        TenorBucket::from_years(self.original_tenor_years())
    }
}
