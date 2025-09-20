use crate::prelude::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
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
    pub instrument_registry: InstrumentRegistry,
}

impl InstrumentCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, id: &InstrumentId) -> Option<&Instrument> {
        self.instruments.get(id)
    }

    pub fn get_mut(&mut self, id: &InstrumentId) -> Option<&mut Instrument> {
        self.instruments.get_mut(id)
    }

    pub fn insert(&mut self, id: InstrumentId, instrument: Instrument) -> Option<Instrument> {
        self.instruments.insert(id, instrument)
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

    pub fn len(&self) -> usize {
        self.instruments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instruments.is_empty()
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketClassification {
    pub primary_market: InstrumentMarket,
    pub default_venue_type: Option<VenueType>,
    pub is_exchange_tradeable: bool,
    pub requires_csd_custody: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleRules {
    pub requires_authorization: bool,
    pub supports_partial_redemption: bool,
    pub accrual_method: Option<AccrualMethod>,
    pub settlement_lag_days: u32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterestType {
    Fixed,
    Variable,
    Zero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BondClass {
    Government,
    Corporate,
    Municipal,
    Agency,
    Supranational,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetClass {
    FixedIncome,
    Equity,
    Commodity,
    Currency,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructuredProductType {
    CDO,
    CLO,
    MBS,
    ABS,
    CoveredBond,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DerivativeClass {
    Option,
    Future,
    Forward,
    Swap,
    CreditDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementType {
    Physical,
    Cash,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccrualMethod {
    Daily,
    Periodic,
    AtMaturity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateStructure {
    pub base_rate: RateIndex,
    pub spread_bps: BasisPoints,
    pub floor_bps: Option<BasisPoints>,
    pub cap_bps: Option<BasisPoints>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralRequirement {
    pub collateral_type: CollateralType,
    pub minimum_coverage_ratio: f64,
    pub haircut: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeScheduleItem {
    pub fee_type: String,
    pub amount: Money,
    pub frequency: PaymentFrequency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatoryApproval {
    pub authority: String,
    pub approval_type: String,
    pub approval_date: NaiveDate,
    pub reference_number: String,
}


#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstrumentRegistry {
    pub templates: HashMap<TemplateId, InstrumentTemplate>,
    pub templates_by_type: HashMap<ProductFamily, Vec<TemplateId>>,
    
    pub series: HashMap<SeriesId, InstrumentSeries>,
    pub series_by_template: HashMap<TemplateId, Vec<SeriesId>>,
    pub series_by_issuer: HashMap<AgentId, Vec<SeriesId>>,
    pub series_by_key: HashMap<SeriesKey, SeriesId>,
    
    pub lots: HashMap<InstrumentId, InstrumentLot>,
    pub lots_by_series: HashMap<SeriesId, Vec<InstrumentId>>,
    
    pub instrument_cache: HashMap<InstrumentId, Instrument>,
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
        
        self.templates_by_type
            .entry(template.product_family.clone())
            .or_default()
            .push(id);
            
        self.templates.insert(id, template);
        Ok(id)
    }
    
    pub fn open_series(
        &mut self,
        template_id: TemplateId,
        issuer: AgentId,
        key: SeriesKey,
        terms: SeriesTerms,
        issuance_data: IssuanceData,
    ) -> Result<SeriesId, String> {
        if !self.templates.contains_key(&template_id) {
            return Err(format!("Template {:?} not found", template_id));
        }
        
        if self.series_by_key.contains_key(&key) {
            return Err("Series with this key already exists".to_string());
        }
        
        let series_id = SeriesId(Uuid::new_v4());
        
        let series = InstrumentSeries {
            id: series_id,
            template_id,
            issuer,
            series_key: key.clone(),
            terms,
            issuance_data,
            outstanding: OutstandingData {
                total_issued: Money::ZERO,
                total_outstanding: Money::ZERO,
                total_retired: Money::ZERO,
                holder_count: 0,
                last_activity_date: chrono::Utc::now().date_naive(),
            },
        };
        
        self.series_by_template
            .entry(template_id)
            .or_default()
            .push(series_id);
            
        self.series_by_issuer
            .entry(issuer)
            .or_default()
            .push(series_id);
            
        self.series_by_key.insert(key, series_id);
        self.series.insert(series_id, series);
        
        Ok(series_id)
    }
    
    pub fn mint_lot(
        &mut self,
        series_id: SeriesId,
        lot_type: LotType,
        quantity: LotQuantity,
    ) -> Result<InstrumentId, String> {
        let series = self.series.get_mut(&series_id)
            .ok_or_else(|| format!("Series {:?} not found", series_id))?;
        
        let lot_id = InstrumentId(Uuid::new_v4());
        
        let lot = InstrumentLot {
            id: lot_id,
            series_id,
            lot_type,
            quantity: quantity.clone(),
            creation_date: chrono::Utc::now().date_naive(),
            status: LotStatus::Active,
        };
        
        match &quantity {
            LotQuantity::Notional(amount) => {
                series.outstanding.total_issued += *amount;
                series.outstanding.total_outstanding += *amount;
            }
            LotQuantity::Units(units) => {
                if let SeriesTerms::BondTerms { face_value, .. } = &series.terms {
                    let amount = *face_value * *units;
                    series.outstanding.total_issued += amount;
                    series.outstanding.total_outstanding += amount;
                }
            }
            _ => {}
        }
        
        series.outstanding.last_activity_date = chrono::Utc::now().date_naive();
        
        self.lots_by_series
            .entry(series_id)
            .or_default()
            .push(lot_id);
            
        self.lots.insert(lot_id, lot);
        
        Ok(lot_id)
    }
    
    pub fn find_or_create_series_for_bond(
        &mut self,
        template_id: TemplateId,
        issuer: AgentId,
        bond: &BondDetails,
    ) -> Result<SeriesId, String> {
        let key = SeriesKey::BondIssue {
            issue_date: bond.issue_date,
            maturity_date: bond.maturity_date,
            coupon_bps: bond.coupon_rate_bps,
            currency: Currency::USD, // You might want to get this from context
        };
        
        if let Some(&series_id) = self.series_by_key.get(&key) {
            return Ok(series_id);
        }
        
        let terms = SeriesTerms::BondTerms {
            face_value: bond.face_value,
            coupon_rate_bps: bond.coupon_rate_bps,
            frequency: bond.frequency,
            rating: bond.rating,
            covenants: Vec::new(),
        };
        
        let issuance_data = IssuanceData {
            authorization_date: bond.issue_date,
            issue_date: bond.issue_date,
            authorized_amount: bond.face_value * 1_000_000.0, // Placeholder
            minimum_denomination: bond.face_value,
            regulatory_approvals: Vec::new(),
        };
        
        self.open_series(template_id, issuer, key, terms, issuance_data)
    }
}