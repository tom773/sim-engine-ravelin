use crate::prelude::*;
use crate::types::instrument::archetypes::{
    BondArchetype, InstrumentArchetype, IssuanceData, LifecycleRules, MarketProfile, OutstandingData, ProductFamily,
};
use crate::types::instrument::inst_core::RuntimeInstrumentCore;
use crate::types::instrument::instrument::Currency;
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{DisplayFromStr, serde_as};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct InstrumentCatalog {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub instruments: HashMap<InstrumentId, Instrument>,
    #[serde(skip)]
    pub cores: HashMap<InstrumentId, RuntimeInstrumentCore>,
}

impl InstrumentCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, id: InstrumentId, instrument: Instrument) -> Option<Instrument> {
        self.instruments.insert(id, instrument)
    }

    pub fn get(&self, id: &InstrumentId) -> Option<&Instrument> {
        self.instruments.get(id)
    }

    pub fn get_mut(&mut self, id: &InstrumentId) -> Option<&mut Instrument> {
        self.instruments.get_mut(id)
    }

    pub fn len(&self) -> usize {
        self.instruments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instruments.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&InstrumentId, &Instrument)> {
        self.instruments.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&InstrumentId, &mut Instrument)> {
        self.instruments.iter_mut()
    }

    pub fn values(&self) -> impl Iterator<Item = &Instrument> {
        self.instruments.values()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Instrument> {
        self.instruments.values_mut()
    }

    pub fn insert_core(&mut self, id: InstrumentId, core: RuntimeInstrumentCore) -> Option<RuntimeInstrumentCore> {
        self.cores.insert(id, core)
    }

    pub fn get_core(&self, id: &InstrumentId) -> Option<&RuntimeInstrumentCore> {
        self.cores.get(id)
    }

    pub fn get_core_mut(&mut self, id: &InstrumentId) -> Option<&mut RuntimeInstrumentCore> {
        self.cores.get_mut(id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentTemplate {
    pub id: TemplateId,
    pub product_family: ProductFamily,
    pub archetype: InstrumentArchetype,
    pub market_profile: MarketProfile,
    pub lifecycle_rules: LifecycleRules,
    pub created_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentSeries {
    pub id: SeriesId,
    pub template_id: TemplateId,
    pub issuer: AgentId,
    pub product_family: ProductFamily,
    pub series_key: SeriesKey,
    pub archetype: InstrumentArchetype,
    pub market_profile: MarketProfile,
    pub issuance_data: IssuanceData,
    pub outstanding: OutstandingData,
    pub metadata: HashMap<String, Value>,
}

impl InstrumentSeries {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: SeriesId, template_id: TemplateId, issuer: AgentId, product_family: ProductFamily, series_key: SeriesKey,
        archetype: InstrumentArchetype, market_profile: MarketProfile, issuance_data: IssuanceData,
    ) -> Self {
        Self {
            id,
            template_id,
            issuer,
            product_family,
            series_key,
            archetype,
            market_profile,
            issuance_data,
            outstanding: OutstandingData::new(Utc::now().date_naive()),
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SeriesKey {
    BondIssue { issue_date: NaiveDate, maturity_date: NaiveDate, coupon_bps: BasisPoints, currency: Currency },
    DepositProduct { product_name: String, rate_bps: BasisPoints, currency: Currency },
    LoanDraw { facility_id: Uuid, draw_number: u32, draw_date: NaiveDate },
    Generic { series_code: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentLot {
    pub id: InstrumentId,
    pub series_id: SeriesId,
    pub lot_type: LotType,
    pub quantity: LotQuantity,
    pub creation_date: NaiveDate,
    pub status: LotStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LotType {
    Fungible { lot_size: f64 },
    LoanDrawdown { draw_id: Uuid, outstanding_principal: Money, next_payment_date: NaiveDate },
    AccountInstance { account_number: String, current_balance: Money },
    Tranche { tranche_name: String, subordination_level: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LotQuantity {
    Units(f64),
    Notional(Money),
    Shares(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LotStatus {
    Active,
    Frozen,
    PendingRedemption,
    Redeemed,
    Cancelled,
}

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

    #[allow(clippy::too_many_arguments)]
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
        let id = instrument.instrument_id();
        self.instrument_cache.insert(id, instrument);
    }

    pub fn cache_get(&self, id: &InstrumentId) -> Option<&Instrument> {
        self.instrument_cache.get(id)
    }

    pub fn update_core_cache(&mut self, core: RuntimeInstrumentCore) {
        let id = core.instrument_id();
        self.instrument_core_cache.insert(id, core);
    }

    pub fn core_cache_get(&self, id: &InstrumentId) -> Option<&RuntimeInstrumentCore> {
        self.instrument_core_cache.get(id)
    }

    pub fn ensure_bond_series(
        &mut self, template_id: TemplateId, issuer: AgentId, archetype: BondArchetype, issue_date: NaiveDate,
        maturity_date: NaiveDate,
    ) -> Result<SeriesId, String> {
        let key = SeriesKey::BondIssue {
            issue_date,
            maturity_date,
            coupon_bps: archetype.coupon_rate_bps,
            currency: Currency::USD,
        };

        if let Some(series_id) = self.series_by_key.get(&key) {
            return Ok(*series_id);
        }

        let template =
            self.templates.get(&template_id).ok_or_else(|| format!("Template {:?} not found", template_id))?;

        if !matches!(template.archetype, InstrumentArchetype::Bond(_)) {
            return Err("Template archetype is not a bond".into());
        }

        let issuance = IssuanceData {
            authorization_date: issue_date,
            issue_date,
            authorized_amount: archetype.face_value * 1_000_000.0,
            minimum_denomination: archetype.face_value,
            regulatory_approvals: Vec::new(),
        };

        self.open_series(template_id, issuer, key, Some(InstrumentArchetype::Bond(archetype)), None, issuance)
    }
}
