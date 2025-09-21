use crate::prelude::*;
use chrono::NaiveDate;
use thiserror::Error;

use super::*;

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

    let base_archetype = match &template.archetype {
        InstrumentArchetype::Bond(archetype) => archetype.clone(),
        _ => return Err(IssuanceError::UnsupportedTemplate),
    };

    let mut issuance_archetype = base_archetype.clone();
    issuance_archetype.face_value = spec.face_value;
    issuance_archetype.coupon_rate_bps = spec.coupon_rate_bps;
    issuance_archetype.frequency_per_year = spec.frequency_per_year;
    let series_id = registry
        .ensure_bond_series(spec.template_id, issuer, issuance_archetype.clone(), spec.issue_date, spec.maturity_date)
        .map_err(IssuanceError::Registry)?;

    let instrument_id = registry
        .mint_lot(series_id, LotType::Fungible { lot_size: spec.face_value.to_f64() }, LotQuantity::Units(spec.units))
        .map_err(IssuanceError::Registry)?;

    let lot_id = LotId(instrument_id.0);

    let instrument =
        Instrument::bond(instrument_id, issuer, spec.bond_type, spec.face_value, spec.issue_date, spec.maturity_date)
            .template(spec.template_id)
            .series(series_id)
            .lot(lot_id)
            .coupon_bps(spec.coupon_rate_bps)
            .frequency(spec.frequency_per_year)
            .rating(spec.rating)
            .outstanding_units(spec.units)
            .auto_market()
            .build()?;

    let instrument_id = instrument.instrument_id();

    let catalog_instrument = instrument.clone();
    catalog.insert(instrument_id, catalog_instrument.clone());
    catalog.insert_core(instrument_id, catalog_instrument.clone());

    registry.update_cache(instrument);
    registry.update_core_cache(catalog_instrument);

    Ok(IssuedBond { template_id: spec.template_id, series_id, instrument_id })
}
