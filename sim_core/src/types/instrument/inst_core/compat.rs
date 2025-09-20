//! Compatibility helpers for bridging the new `InstrumentCore` API with the
//! legacy `Instrument` struct. This module should be deleted once all call
//! sites operate directly on `InstrumentCore` and the legacy type is retired.

use super::{
    BondState, CashState, InstrumentCore, InstrumentIdentifiers, InstrumentRuntime, Listability, MarketProfile,
};
use crate::types::instrument::archetypes::{BondArchetype, CreditRating};
use crate::types::instrument::credit::DebtInstrument;
use crate::types::instrument::instrument::{BondDetails, CashDetails, Instrument, InstrumentType};

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

pub fn legacy_type_from_runtime(runtime: &InstrumentRuntime) -> InstrumentType {
    match runtime {
        InstrumentRuntime::Cash(state) => InstrumentType::Cash(cash_details_from_state(state)),
        InstrumentRuntime::Bond(state) => InstrumentType::Debt(DebtInstrument::Bond(bond_details_from_state(state))),
    }
}

pub fn legacy_type_from_runtime_owned(runtime: InstrumentRuntime) -> InstrumentType {
    match runtime {
        InstrumentRuntime::Cash(state) => InstrumentType::Cash(cash_details_from_state(&state)),
        InstrumentRuntime::Bond(state) => InstrumentType::Debt(DebtInstrument::Bond(bond_details_from_state(&state))),
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
