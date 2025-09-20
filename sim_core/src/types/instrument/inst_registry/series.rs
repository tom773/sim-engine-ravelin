use crate::*;
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

use super::registry::{SeriesId, TemplateId};
use crate::types::instrument::archetypes::{
    InstrumentArchetype, IssuanceData, MarketProfile, OutstandingData, ProductFamily,
};

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
