use crate::prelude::*;
use chrono::NaiveDate;
use thiserror::Error;
use uuid::Uuid;

use super::inst_core::{InstrumentIdentifiers, legacy_instrument_from_runtime_core, runtime_core_from_legacy_bond};
use super::inst_registry::{InstrumentCatalog, InstrumentRegistry, LotId, LotQuantity, LotType, SeriesId, TemplateId};
use super::{BondType, Instrument, InstrumentArchetype};

#[derive(Debug, Clone)]
pub struct BondIssuanceSpec {
    pub template_id: TemplateId,
    pub bond_type: BondType,
    pub face_value: Money,
    pub coupon_rate_bps: BasisPoints,
    pub issue_date: NaiveDate,
    pub maturity_date: NaiveDate,
    pub frequency_per_year: u32,
    pub rating: CreditRating,
    pub units: f64,
}

#[derive(Debug, Error)]
pub enum IssuanceError {
    #[error("template {0} not found")]
    MissingTemplate(TemplateId),
    #[error("builder error: {0}")]
    BuildError(#[from] crate::types::instrument::builder::BuildError),
    #[error("registry error: {0}")]
    Registry(String),
    #[error("unsupported template archetype for bond issuance")]
    UnsupportedTemplate,
}

#[derive(Debug, Clone)]
pub struct IssuedBond {
    pub template_id: TemplateId,
    pub series_id: SeriesId,
    pub instrument_id: InstrumentId,
}

pub fn issue_corporate_bond(
    registry: &mut InstrumentRegistry, catalog: &mut InstrumentCatalog, issuer: AgentId, spec: BondIssuanceSpec,
) -> Result<IssuedBond, IssuanceError> {
    let template = registry.get_template(&spec.template_id).ok_or(IssuanceError::MissingTemplate(spec.template_id))?;

    if !matches!(template.archetype, InstrumentArchetype::Bond(_)) {
        return Err(IssuanceError::UnsupportedTemplate);
    }

    let built_instrument = Instrument::bond(
        InstrumentId(Uuid::new_v4()),
        issuer,
        spec.bond_type,
        spec.face_value,
        spec.issue_date,
        spec.maturity_date,
    )
    .coupon_bps(spec.coupon_rate_bps)
    .frequency(spec.frequency_per_year)
    .rating(spec.rating)
    .auto_market()
    .build()?;

    let bond_details = built_instrument.instrument_type.as_bond().expect("builder produced bond").clone();
    let market_profile = built_instrument.market_profile.clone();
    let listability = built_instrument.listability.clone();

    let series_id = registry
        .find_or_create_series_for_bond(spec.template_id, issuer, &bond_details)
        .map_err(IssuanceError::Registry)?;

    let lot_id = registry
        .mint_lot(series_id, LotType::Fungible { lot_size: spec.face_value.to_f64() }, LotQuantity::Units(spec.units))
        .map_err(IssuanceError::Registry)?;

    let bond_archetype = registry
        .series
        .get(&series_id)
        .and_then(|series| match &series.archetype {
            InstrumentArchetype::Bond(archetype) => Some(archetype.clone()),
            _ => None,
        })
        .ok_or(IssuanceError::UnsupportedTemplate)?;

    let identifiers = InstrumentIdentifiers::new(lot_id)
        .with_template(spec.template_id)
        .with_series(series_id)
        .with_lot(LotId(lot_id.0));

    let runtime_core = runtime_core_from_legacy_bond(
        identifiers,
        market_profile,
        listability,
        &bond_details,
        bond_archetype,
        spec.units,
    );

    let instrument = legacy_instrument_from_runtime_core(&runtime_core);
    catalog.insert(lot_id, instrument.clone());
    catalog.insert_core(lot_id, runtime_core.clone());

    registry.update_cache(instrument);
    registry.update_core_cache(runtime_core);

    Ok(IssuedBond { template_id: spec.template_id, series_id, instrument_id: lot_id })
}
