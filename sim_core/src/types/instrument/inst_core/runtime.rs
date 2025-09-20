use crate::prelude::*;
use crate::types::instrument::archetypes::{
    BondArchetype, BondType, CashFlow, CashType, Covenant, CreditRating, LoanArchetype,
};
use crate::types::instrument::credit::{
    ConsumerLoanCategory, CreditLineDetails, LoanType, TradeCreditDetails,
};
use crate::types::instrument::instrument::Currency;
use crate::types::money::Money;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstrumentRuntime {
    Cash(CashState),
    Bond(BondState),
    Credit(CreditState),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CreditState {
    Loan(LoanState),
    ConsumerLoan { category: ConsumerLoanCategory, loan: LoanState },
    ConsumerCreditCard(CreditLineDetails),
    TradeCredit(TradeCreditDetails),
    CreditLine(CreditLineDetails),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashState {
    pub issuer: AgentId,
    pub cash_type: CashType,
    pub currency: Currency,
    pub interest_bps: BasisPoints,
}

impl CashState {
    pub fn new(issuer: AgentId, cash_type: CashType, currency: Currency, interest_bps: BasisPoints) -> Self {
        Self { issuer, cash_type, currency, interest_bps }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondState {
    pub issuer: AgentId,
    pub bond_type: BondType,
    pub archetype: BondArchetype,
    pub cash_flow: CashFlow,
    pub issue_date: NaiveDate,
    pub maturity_date: NaiveDate,
    pub outstanding_units: f64,
    pub last_accrual_date: Option<NaiveDate>,
    pub rating: Option<CreditRating>,
}

impl BondState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        issuer: AgentId, bond_type: BondType, archetype: BondArchetype, cash_flow: CashFlow, issue_date: NaiveDate,
        maturity_date: NaiveDate, outstanding_units: f64, last_accrual_date: Option<NaiveDate>,
        rating: Option<CreditRating>,
    ) -> Self {
        Self {
            issuer,
            bond_type,
            archetype,
            cash_flow,
            issue_date,
            maturity_date,
            outstanding_units,
            last_accrual_date,
            rating,
        }
    }

    pub fn outstanding_notional(&self) -> Money {
        self.archetype.face_value * self.outstanding_units
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanState {
    pub loan_id: Uuid,
    pub lender: AgentId,
    pub borrower: AgentId,
    pub facility_id: Option<Uuid>,
    pub loan_type: LoanType,
    pub archetype: LoanArchetype,
    pub outstanding_principal: Money,
    pub reference_rate: Option<RateIndex>,
    pub spread_bps: BasisPoints,
    pub rate_floor_bps: Option<BasisPoints>,
    pub rate_cap_bps: Option<BasisPoints>,
    pub origination_date: NaiveDate,
    pub maturity_date: NaiveDate,
    pub next_payment_date: NaiveDate,
    pub last_accrual_date: NaiveDate,
    pub collateral: Vec<LienId>,
    pub covenants: Vec<Covenant>,
    pub rating: Option<CreditRating>,
    pub impairment: ImpairmentState,
    pub accrued_interest: Money,
    pub unamortized_fees: Money,
}

impl LoanState {
    pub fn new(
        loan_id: Uuid, lender: AgentId, borrower: AgentId, facility_id: Option<Uuid>, loan_type: LoanType,
        archetype: LoanArchetype, outstanding_principal: Money, reference_rate: Option<RateIndex>,
        spread_bps: BasisPoints, rate_floor_bps: Option<BasisPoints>, rate_cap_bps: Option<BasisPoints>,
        origination_date: NaiveDate, maturity_date: NaiveDate, next_payment_date: NaiveDate,
        last_accrual_date: NaiveDate, collateral: Vec<LienId>, covenants: Vec<Covenant>, rating: Option<CreditRating>,
        impairment: ImpairmentState, accrued_interest: Money, unamortized_fees: Money,
    ) -> Self {
        Self {
            loan_id,
            lender,
            borrower,
            facility_id,
            loan_type,
            archetype,
            outstanding_principal,
            reference_rate,
            spread_bps,
            rate_floor_bps,
            rate_cap_bps,
            origination_date,
            maturity_date,
            next_payment_date,
            last_accrual_date,
            collateral,
            covenants,
            rating,
            impairment,
            accrued_interest,
            unamortized_fees,
        }
    }

    pub fn principal(&self) -> Money {
        self.archetype.principal
    }
}
