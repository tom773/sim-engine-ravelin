use crate::prelude::*;
use crate::types::instrument::archetypes::{BondArchetype, BondType, CashFlow, CashType, CreditRating};
use crate::types::instrument::instrument::Currency;
use crate::types::money::Money;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstrumentRuntime {
    Cash(CashState),
    Bond(BondState),
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
