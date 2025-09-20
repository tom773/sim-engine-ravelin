use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::registry::TemplateId;
use crate::types::instrument::archetypes::{InstrumentArchetype, LifecycleRules, MarketProfile, ProductFamily};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentTemplate {
    pub id: TemplateId,
    pub product_family: ProductFamily,
    pub archetype: InstrumentArchetype,
    pub market_profile: MarketProfile,
    pub lifecycle_rules: LifecycleRules,
    pub created_date: NaiveDate,
}
