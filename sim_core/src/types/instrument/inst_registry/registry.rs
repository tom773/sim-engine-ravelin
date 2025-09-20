use crate::prelude::*;
use crate::types::instrument::archetypes::{InstrumentArchetype, IssuanceData, ProductFamily};
use crate::types::instrument::inst_core::MarketProfile;
use crate::types::instrument::inst_core::RuntimeInstrumentCore;
use crate::types::instrument::instrument::Currency;
use crate::types::instrument::{BondDetails, Instrument};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

use super::lot::{InstrumentLot, LotQuantity, LotStatus, LotType};
use super::series::{InstrumentSeries, SeriesKey};
use super::template::InstrumentTemplate;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemplateId(pub Uuid);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SeriesId(pub Uuid);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LotId(pub Uuid);

impl fmt::Display for TemplateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for TemplateId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

impl fmt::Display for SeriesId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for SeriesId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

impl fmt::Display for LotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for LotId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstrumentRegistry {
    pub templates: HashMap<TemplateId, InstrumentTemplate>,
    pub templates_by_family: HashMap<ProductFamily, Vec<TemplateId>>,

    pub series: HashMap<SeriesId, InstrumentSeries>,
    pub series_by_template: HashMap<TemplateId, Vec<SeriesId>>,
    pub series_by_issuer: HashMap<AgentId, Vec<SeriesId>>,
    pub series_by_key: HashMap<SeriesKey, SeriesId>,

    pub lots: HashMap<InstrumentId, InstrumentLot>,
    pub lots_by_series: HashMap<SeriesId, Vec<InstrumentId>>,

    #[serde(skip)]
    pub instrument_cache: HashMap<InstrumentId, Instrument>,
    #[serde(skip)]
    pub instrument_core_cache: HashMap<InstrumentId, RuntimeInstrumentCore>,
}

impl InstrumentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_template(&mut self, template: InstrumentTemplate) -> Result<TemplateId, String> {
        let id = template.id;
        if self.templates.contains_key(&id) {
            return Err(format!("Template {:?} already exists", id));
        }

        self.templates_by_family.entry(template.product_family.clone()).or_default().push(id);

        self.templates.insert(id, template);
        Ok(id)
    }

    pub fn get_template(&self, id: &TemplateId) -> Option<&InstrumentTemplate> {
        self.templates.get(id)
    }

    pub fn open_series(
        &mut self, template_id: TemplateId, issuer: AgentId, key: SeriesKey,
        archetype_override: Option<InstrumentArchetype>, market_profile_override: Option<MarketProfile>,
        issuance_data: IssuanceData,
    ) -> Result<SeriesId, String> {
        let template =
            self.templates.get(&template_id).ok_or_else(|| format!("Template {:?} not found", template_id))?;

        if self.series_by_key.contains_key(&key) {
            return Err("Series with this key already exists".to_string());
        }

        let mut archetype = template.archetype.clone();
        if let Some(custom) = archetype_override {
            archetype = custom;
        }

        let market_profile = market_profile_override.unwrap_or_else(|| template.market_profile.clone());

        let series_id = SeriesId(Uuid::new_v4());
        let series = InstrumentSeries::new(
            series_id,
            template_id,
            issuer,
            template.product_family.clone(),
            key.clone(),
            archetype,
            market_profile,
            issuance_data,
        );

        self.series_by_template.entry(template_id).or_default().push(series_id);
        self.series_by_issuer.entry(issuer).or_default().push(series_id);
        self.series_by_key.insert(key, series_id);
        self.series.insert(series_id, series);

        Ok(series_id)
    }

    pub fn mint_lot(
        &mut self, series_id: SeriesId, lot_type: LotType, quantity: LotQuantity,
    ) -> Result<InstrumentId, String> {
        let series = self.series.get_mut(&series_id).ok_or_else(|| format!("Series {:?} not found", series_id))?;

        let lot_id = InstrumentId(Uuid::new_v4());
        let lot = InstrumentLot {
            id: lot_id,
            series_id,
            lot_type,
            quantity: quantity.clone(),
            creation_date: Utc::now().date_naive(),
            status: LotStatus::Active,
        };

        match (&quantity, &series.archetype) {
            (LotQuantity::Notional(amount), _) => {
                series.outstanding.total_issued += *amount;
                series.outstanding.total_outstanding += *amount;
            }
            (LotQuantity::Units(units), InstrumentArchetype::Bond(bond)) => {
                let amount = bond.face_value * *units;
                series.outstanding.total_issued += amount;
                series.outstanding.total_outstanding += amount;
            }
            _ => {}
        }

        series.outstanding.last_activity_date = Utc::now().date_naive();

        self.lots_by_series.entry(series_id).or_default().push(lot_id);
        self.lots.insert(lot_id, lot);

        Ok(lot_id)
    }

    pub fn update_cache(&mut self, instrument: Instrument) {
        self.instrument_cache.insert(instrument.id, instrument);
    }

    pub fn cache_get(&self, id: &InstrumentId) -> Option<&Instrument> {
        self.instrument_cache.get(id)
    }

    pub fn update_core_cache(&mut self, core: RuntimeInstrumentCore) {
        self.instrument_core_cache.insert(core.identifiers.instrument_id, core);
    }

    pub fn core_cache_get(&self, id: &InstrumentId) -> Option<&RuntimeInstrumentCore> {
        self.instrument_core_cache.get(id)
    }

    pub fn find_or_create_series_for_bond(
        &mut self, template_id: TemplateId, issuer: AgentId, bond: &BondDetails,
    ) -> Result<SeriesId, String> {
        let key = SeriesKey::BondIssue {
            issue_date: bond.issue_date,
            maturity_date: bond.maturity_date,
            coupon_bps: bond.coupon_rate_bps,
            currency: Currency::USD,
        };

        if let Some(series_id) = self.series_by_key.get(&key) {
            return Ok(*series_id);
        }

        let template =
            self.templates.get(&template_id).ok_or_else(|| format!("Template {:?} not found", template_id))?;

        let base_bond = match &template.archetype {
            InstrumentArchetype::Bond(base) => base.clone(),
            _ => return Err("Template archetype is not a bond".into()),
        };

        let mut bond_archetype = base_bond;
        bond_archetype.face_value = bond.face_value;
        bond_archetype.coupon_rate_bps = bond.coupon_rate_bps;
        bond_archetype.frequency_per_year = bond.frequency;
        bond_archetype.rating = Some(bond.rating);

        let issuance = IssuanceData {
            authorization_date: bond.issue_date,
            issue_date: bond.issue_date,
            authorized_amount: bond.face_value * 1_000_000.0,
            minimum_denomination: bond.face_value,
            regulatory_approvals: Vec::new(),
        };

        self.open_series(template_id, issuer, key, Some(InstrumentArchetype::Bond(bond_archetype)), None, issuance)
    }
}
