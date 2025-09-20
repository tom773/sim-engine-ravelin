use crate::prelude::*;
use crate::types::money::Money;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProductFamily {
    Cash,
    FixedIncome,
    Equity,
    Credit,
    Structured,
    Derivative,
    RealAsset,
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

pub use crate::types::instrument::inst_core::{
    CapitalMarketSegment, DerivativesMarketSegment, InstrumentMarket, MarketProfile, MoneyMarketSegment, VenueType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccrualMethod {
    Daily,
    Periodic,
    AtMaturity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleRules {
    pub requires_authorization: bool,
    pub supports_partial_redemption: bool,
    pub accrual_method: Option<AccrualMethod>,
    pub settlement_lag_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatoryApproval {
    pub authority: String,
    pub approval_type: String,
    pub approval_date: NaiveDate,
    pub reference_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuanceData {
    pub authorization_date: NaiveDate,
    pub issue_date: NaiveDate,
    pub authorized_amount: Money,
    pub minimum_denomination: Money,
    pub regulatory_approvals: Vec<RegulatoryApproval>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutstandingData {
    pub total_issued: Money,
    pub total_outstanding: Money,
    pub total_retired: Money,
    pub holder_count: u32,
    pub last_activity_date: NaiveDate,
}

impl OutstandingData {
    pub fn new(last_activity_date: NaiveDate) -> Self {
        Self {
            total_issued: Money::ZERO,
            total_outstanding: Money::ZERO,
            total_retired: Money::ZERO,
            holder_count: 0,
            last_activity_date,
        }
    }
}

impl Default for OutstandingData {
    fn default() -> Self {
        Self::new(NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid epoch"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterestType {
    Fixed,
    Variable,
    Zero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BondType {
    Government,
    Corporate,
    InterbankLoan,
    Municipal,
    Agency,
    Supranational,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CashFlow {
    Zero,
    Fixed,
    Floating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetClass {
    FixedIncome,
    Equity,
    Commodity,
    Currency,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructuredProductType {
    CDO,
    CLO,
    MBS,
    ABS,
    CoveredBond,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DerivativeClass {
    Option,
    Future,
    Forward,
    Swap,
    CreditDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SettlementType {
    Physical,
    Cash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FacilityType {
    Revolver,
    TermLoanFacility,
    Overdraft,
    LetterOfCredit,
    MultiCurrency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RateIndex {
    RBACashRate,
    BBSW1M,
    BBSW3M,
    BBSW6M,
    SOFR,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Compounding {
    Simple,
    Compounded,
    EffectiveInterestRate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaymentFrequency {
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
    InterestOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateStructure {
    pub base_rate: RateIndex,
    pub spread_bps: BasisPoints,
    pub floor_bps: Option<BasisPoints>,
    pub cap_bps: Option<BasisPoints>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Amortization {
    Bullet,
    Linear,
    Annuity,
    InterestOnly,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrepaymentPenalty {
    None,
    FixedAmount(i64),
    PercentOfPrincipal(u32),
    MakeWhole,
    YieldMaintenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepaymentTerms {
    pub allowed: bool,
    pub penalty_type: PrepaymentPenalty,
    pub lockout_period_months: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestFrequency {
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
    OnDemand,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CollateralType {
    Securities { instrument_id: InstrumentId, quantity: f64, haircut: f64 },
    RealEstate { property_id: Uuid, valuation: Money, ltv_limit: f64 },
    Receivables { debtor: AgentId, amount: Money },
    Cash { amount: Money },
    Other { description: String, value: Money },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollateralRequirement {
    pub collateral_type: CollateralType,
    pub minimum_coverage_ratio: f64,
    pub haircut: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CovenantType {
    DebtToEquity { max_ratio: f64 },
    InterestCoverage { min_ratio: f64 },
    CurrentRatio { min_ratio: f64 },
    MinimumNetWorth { amount: Money },
    MaximumCapex { amount: Money },
    RestrictedPayments,
    ChangeOfControl,
    CrossDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CovenantStatus {
    InCompliance,
    Breached { breach_date: NaiveDate },
    Waived { until_date: NaiveDate },
    Cured,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Covenant {
    pub covenant_type: CovenantType,
    pub status: CovenantStatus,
    pub test_frequency: TestFrequency,
    pub last_test_date: Option<NaiveDate>,
    pub cure_period_days: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpCreditRating {
    AAA,
    AA,
    A,
    BBB,
    BB,
    B,
    CCC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsumerCreditRating {
    Prime,
    NearPrime,
    Subprime,
    DeepSubprime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CreditRating {
    Government(SpCreditRating),
    Corporate(SpCreditRating),
    Consumer(ConsumerCreditRating),
}

impl CreditRating {
    pub fn government_aaa() -> Self {
        CreditRating::Government(SpCreditRating::AAA)
    }

    pub fn corporate_bbb() -> Self {
        CreditRating::Corporate(SpCreditRating::BBB)
    }

    pub fn consumer_prime() -> Self {
        CreditRating::Consumer(ConsumerCreditRating::Prime)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeeScheduleItem {
    pub fee_type: String,
    pub amount: Money,
    pub frequency: PaymentFrequency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondArchetype {
    pub bond_type: BondType,
    pub cash_flow_type: CashFlow,
    pub day_count: DayCount,
    pub face_value: Money,
    pub coupon_rate_bps: BasisPoints,
    pub frequency_per_year: u32,
    pub rating: Option<CreditRating>,
    pub covenants: Vec<Covenant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositArchetype {
    pub deposit_type: CashType,
    pub interest_type: InterestType,
    pub interest_rate_bps: BasisPoints,
    pub minimum_balance: Option<Money>,
    pub fee_schedule: Vec<FeeScheduleItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanRepaymentSchedule {
    pub payment_frequency: PaymentFrequency,
    pub term_months: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanArchetype {
    pub facility_type: FacilityType,
    pub amortization: Amortization,
    pub prepayment: PrepaymentTerms,
    pub principal: Money,
    pub rate_structure: RateStructure,
    pub repayment_schedule: LoanRepaymentSchedule,
    pub collateral_requirements: Vec<CollateralRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrancheProfile {
    pub subordination_level: u32,
    pub target_notional: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredProductArchetype {
    pub product_type: StructuredProductType,
    pub underlying_asset_class: AssetClass,
    pub tranche_definitions: HashMap<String, TrancheProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivativeArchetype {
    pub derivative_class: DerivativeClass,
    pub settlement_type: SettlementType,
    pub underlying: Option<AssetClass>,
    pub notional: Option<Money>,
    pub strike: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstrumentArchetype {
    Bond(BondArchetype),
    Loan(LoanArchetype),
    Deposit(DepositArchetype),
    StructuredProduct(StructuredProductArchetype),
    Derivative(DerivativeArchetype),
    Custom(HashMap<String, Value>),
}
