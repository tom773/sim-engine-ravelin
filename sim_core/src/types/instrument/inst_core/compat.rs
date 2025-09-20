//! Compatibility helpers for bridging the new `InstrumentCore` API with the
//! legacy `Instrument` struct. This module should be deleted once all call
//! sites operate directly on `InstrumentCore` and the legacy type is retired.

use super::{
    BondState, CashState, CreditState, InstrumentCore, InstrumentIdentifiers, InstrumentRuntime, Listability,
    LoanState, MarketProfile,
};
use crate::types::instrument::archetypes::{
    BondArchetype, CreditRating, FacilityType, LoanArchetype, LoanRepaymentSchedule, RateIndex, RateStructure,
};
use crate::types::instrument::credit::{
    ConsumerDebt, ConsumerLoanCategory, CreditLineDetails, DebtInstrument, LoanDetails, TradeCreditDetails,
};
use crate::types::instrument::instrument::{BondDetails, CashDetails, Instrument, InstrumentType};
use chrono::{Datelike, NaiveDate};

pub type LegacyInstrumentCore = InstrumentCore<InstrumentType>;
pub type RuntimeInstrumentCore = InstrumentCore<InstrumentRuntime>;

pub fn legacy_identifiers(instrument: &Instrument) -> InstrumentIdentifiers {
    InstrumentIdentifiers::from(instrument.id)
}

pub fn legacy_core_from_instrument_with_identifiers(
    instrument: &Instrument, mut identifiers: InstrumentIdentifiers,
) -> LegacyInstrumentCore {
    identifiers.instrument_id = instrument.id;
    InstrumentCore::new(
        identifiers,
        instrument.market_profile.clone(),
        instrument.listability.clone(),
        instrument.instrument_type.clone(),
    )
}

pub fn legacy_core_from_instrument(instrument: &Instrument) -> LegacyInstrumentCore {
    legacy_core_from_instrument_with_identifiers(instrument, legacy_identifiers(instrument))
}

pub fn legacy_core_from_owned_with_identifiers(
    instrument: Instrument, identifiers: InstrumentIdentifiers,
) -> LegacyInstrumentCore {
    let mut core = legacy_core_from_owned(instrument);
    let mut identifiers = identifiers;
    identifiers.instrument_id = core.identifiers.instrument_id;
    core.identifiers = identifiers;
    core
}

pub fn legacy_core_from_owned(instrument: Instrument) -> LegacyInstrumentCore {
    let Instrument { id, instrument_type, market_profile, listability } = instrument;
    let identifiers = InstrumentIdentifiers::new(id);
    InstrumentCore::new(identifiers, market_profile, listability, instrument_type)
}

pub fn update_instrument_from_legacy_core(instrument: &mut Instrument, core: &LegacyInstrumentCore) {
    instrument.id = core.identifiers.instrument_id;
    instrument.market_profile = core.market_profile.clone();
    instrument.listability = core.listability.clone();
    instrument.instrument_type = core.state.clone();
}

pub fn bond_state_from_legacy_details(details: &BondDetails, mut archetype: BondArchetype, units: f64) -> BondState {
    archetype.face_value = details.face_value;
    archetype.coupon_rate_bps = details.coupon_rate_bps;
    archetype.frequency_per_year = details.frequency;
    archetype.day_count = details.day_count;
    archetype.cash_flow_type = details.cash_flow;
    archetype.rating = Some(details.rating);

    BondState::new(
        details.issuer,
        details.bond_type,
        archetype,
        details.cash_flow,
        details.issue_date,
        details.maturity_date,
        units,
        details.last_accrual_date,
        Some(details.rating),
    )
}

pub fn bond_details_from_state(state: &BondState) -> BondDetails {
    BondDetails {
        bond_type: state.bond_type,
        issuer: state.issuer,
        cash_flow: state.cash_flow,
        coupon_rate_bps: state.archetype.coupon_rate_bps,
        face_value: state.archetype.face_value,
        issue_date: state.issue_date,
        maturity_date: state.maturity_date,
        frequency: state.archetype.frequency_per_year,
        day_count: state.archetype.day_count,
        rating: state.rating.or(state.archetype.rating).unwrap_or_else(CreditRating::corporate_bbb),
        last_accrual_date: state.last_accrual_date,
    }
}

pub fn cash_state_from_legacy_details(details: &CashDetails) -> CashState {
    CashState::new(details.issuer, details.cash_type, details.currency, details.interest_bps)
}

pub fn cash_details_from_state(state: &CashState) -> CashDetails {
    CashDetails {
        issuer: state.issuer,
        cash_type: state.cash_type,
        currency: state.currency,
        interest_bps: state.interest_bps,
    }
}

fn months_between(start: NaiveDate, end: NaiveDate) -> u32 {
    let mut months = (end.year() - start.year()) * 12 + (end.month() as i32 - start.month() as i32);
    if months < 0 {
        months = 0;
    }
    months as u32
}

fn loan_archetype_from_details(details: &LoanDetails) -> LoanArchetype {
    LoanArchetype {
        facility_type: FacilityType::TermLoanFacility,
        amortization: details.amortization,
        prepayment: details.prepayment_terms.clone(),
        principal: details.principal,
        day_count: details.day_count,
        compounding: details.compounding,
        rate_structure: RateStructure {
            base_rate: details.reference_rate.unwrap_or(RateIndex::Fixed),
            spread_bps: details.spread_bps,
            floor_bps: details.rate_floor_bps,
            cap_bps: details.rate_cap_bps,
        },
        repayment_schedule: LoanRepaymentSchedule {
            payment_frequency: details.payment_frequency,
            term_months: months_between(details.origination_date, details.maturity_date),
        },
        collateral_requirements: Vec::new(),
    }
}

pub fn loan_state_from_legacy_details(details: &LoanDetails) -> LoanState {
    let archetype = loan_archetype_from_details(details);
    LoanState::new(
        details.loan_id,
        details.lender,
        details.borrower,
        details.facility_id,
        details.loan_type,
        archetype,
        details.outstanding_principal,
        details.reference_rate,
        details.spread_bps,
        details.rate_floor_bps,
        details.rate_cap_bps,
        details.origination_date,
        details.maturity_date,
        details.next_payment_date,
        details.last_accrual_date,
        details.collateral.clone(),
        details.covenants.clone(),
        details.rating,
        details.impairment.clone(),
        details.accrued_interest,
        details.unamortized_fees,
    )
}

pub fn loan_details_from_state(state: &LoanState) -> LoanDetails {
    LoanDetails {
        loan_id: state.loan_id,
        lender: state.lender,
        borrower: state.borrower,
        loan_type: state.loan_type,
        facility_id: state.facility_id,
        principal: state.archetype.principal,
        outstanding_principal: state.outstanding_principal,
        reference_rate: state.reference_rate,
        spread_bps: state.spread_bps,
        rate_floor_bps: state.rate_floor_bps,
        rate_cap_bps: state.rate_cap_bps,
        day_count: state.archetype.day_count,
        compounding: state.archetype.compounding,
        payment_frequency: state.archetype.repayment_schedule.payment_frequency,
        origination_date: state.origination_date,
        maturity_date: state.maturity_date,
        next_payment_date: state.next_payment_date,
        last_accrual_date: state.last_accrual_date,
        amortization: state.archetype.amortization,
        prepayment_terms: state.archetype.prepayment.clone(),
        collateral: state.collateral.clone(),
        covenants: state.covenants.clone(),
        rating: state.rating,
        impairment: state.impairment.clone(),
        accrued_interest: state.accrued_interest,
        unamortized_fees: state.unamortized_fees,
    }
}

fn consumer_debt_from_state(category: ConsumerLoanCategory, state: &LoanState) -> ConsumerDebt {
    let details = loan_details_from_state(state);
    match category {
        ConsumerLoanCategory::ResidentialMortgage => ConsumerDebt::ResidentialMortgage(details),
        ConsumerLoanCategory::AutoLoan => ConsumerDebt::AutoLoan(details),
        ConsumerLoanCategory::PersonalLoan => ConsumerDebt::PersonalLoan(details),
        ConsumerLoanCategory::StudentLoan => ConsumerDebt::StudentLoan(details),
    }
}

pub fn legacy_type_from_runtime(runtime: &InstrumentRuntime) -> InstrumentType {
    match runtime {
        InstrumentRuntime::Cash(state) => InstrumentType::Cash(cash_details_from_state(state)),
        InstrumentRuntime::Bond(state) => InstrumentType::Debt(DebtInstrument::Bond(bond_details_from_state(state))),
        InstrumentRuntime::Credit(state) => InstrumentType::Debt(debt_from_credit_state(state)),
    }
}

pub fn legacy_type_from_runtime_owned(runtime: InstrumentRuntime) -> InstrumentType {
    match runtime {
        InstrumentRuntime::Cash(state) => InstrumentType::Cash(cash_details_from_state(&state)),
        InstrumentRuntime::Bond(state) => InstrumentType::Debt(DebtInstrument::Bond(bond_details_from_state(&state))),
        InstrumentRuntime::Credit(state) => InstrumentType::Debt(debt_from_credit_state(&state)),
    }
}

fn debt_from_credit_state(state: &CreditState) -> DebtInstrument {
    match state {
        CreditState::Loan(loan) => DebtInstrument::Loan(loan_details_from_state(loan)),
        CreditState::ConsumerLoan { category, loan } => {
            DebtInstrument::Consumer(consumer_debt_from_state(*category, loan))
        }
        CreditState::ConsumerCreditCard(details) => DebtInstrument::Consumer(ConsumerDebt::CreditCard(details.clone())),
        CreditState::TradeCredit(details) => DebtInstrument::TradeCredit(details.clone()),
        CreditState::CreditLine(details) => DebtInstrument::CreditLine(details.clone()),
    }
}

pub fn legacy_instrument_from_runtime_core(core: &RuntimeInstrumentCore) -> Instrument {
    Instrument {
        id: core.identifiers.instrument_id,
        instrument_type: legacy_type_from_runtime(&core.state),
        market_profile: core.market_profile.clone(),
        listability: core.listability.clone(),
    }
}

pub fn legacy_core_from_runtime_core(core: RuntimeInstrumentCore) -> LegacyInstrumentCore {
    let InstrumentCore { identifiers, market_profile, listability, state } = core;
    InstrumentCore::new(identifiers, market_profile, listability, legacy_type_from_runtime_owned(state))
}

pub fn runtime_core_from_legacy_bond(
    identifiers: InstrumentIdentifiers, market_profile: MarketProfile, listability: Listability, details: &BondDetails,
    archetype: BondArchetype, units: f64,
) -> RuntimeInstrumentCore {
    let state = InstrumentRuntime::Bond(bond_state_from_legacy_details(details, archetype, units));
    InstrumentCore::new(identifiers, market_profile, listability, state)
}

pub fn runtime_core_from_legacy_cash(
    identifiers: InstrumentIdentifiers, market_profile: MarketProfile, listability: Listability, details: &CashDetails,
) -> RuntimeInstrumentCore {
    let state = InstrumentRuntime::Cash(cash_state_from_legacy_details(details));
    InstrumentCore::new(identifiers, market_profile, listability, state)
}

fn runtime_core_from_credit_state(
    identifiers: InstrumentIdentifiers, market_profile: MarketProfile, listability: Listability, state: CreditState,
) -> RuntimeInstrumentCore {
    InstrumentCore::new(identifiers, market_profile, listability, InstrumentRuntime::Credit(state))
}

pub fn runtime_core_from_legacy_loan(
    identifiers: InstrumentIdentifiers, market_profile: MarketProfile, listability: Listability, details: &LoanDetails,
) -> RuntimeInstrumentCore {
    let loan_state = loan_state_from_legacy_details(details);
    runtime_core_from_credit_state(identifiers, market_profile, listability, CreditState::Loan(loan_state))
}

pub fn runtime_core_from_legacy_consumer_loan(
    identifiers: InstrumentIdentifiers, market_profile: MarketProfile, listability: Listability,
    category: ConsumerLoanCategory, details: &LoanDetails,
) -> RuntimeInstrumentCore {
    let loan_state = loan_state_from_legacy_details(details);
    runtime_core_from_credit_state(
        identifiers,
        market_profile,
        listability,
        CreditState::ConsumerLoan { category, loan: loan_state },
    )
}

pub fn runtime_core_from_legacy_credit_card(
    identifiers: InstrumentIdentifiers, market_profile: MarketProfile, listability: Listability,
    details: &CreditLineDetails,
) -> RuntimeInstrumentCore {
    runtime_core_from_credit_state(
        identifiers,
        market_profile,
        listability,
        CreditState::ConsumerCreditCard(details.clone()),
    )
}

pub fn runtime_core_from_legacy_credit_line(
    identifiers: InstrumentIdentifiers, market_profile: MarketProfile, listability: Listability,
    details: &CreditLineDetails,
) -> RuntimeInstrumentCore {
    runtime_core_from_credit_state(identifiers, market_profile, listability, CreditState::CreditLine(details.clone()))
}

pub fn runtime_core_from_legacy_trade_credit(
    identifiers: InstrumentIdentifiers, market_profile: MarketProfile, listability: Listability,
    details: &TradeCreditDetails,
) -> RuntimeInstrumentCore {
    runtime_core_from_credit_state(identifiers, market_profile, listability, CreditState::TradeCredit(details.clone()))
}
