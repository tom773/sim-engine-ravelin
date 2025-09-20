use crate::prelude::*;
use crate::types::instrument::inst_registry::{LotId, SeriesId, TemplateId};
use crate::types::instrument::instrument::InstrumentType;
use crate::types::money::Money;
use serde::{Deserialize, Serialize};

use super::{MarketProfile, VenueType};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstrumentIdentifiers {
    pub instrument_id: InstrumentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<TemplateId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_id: Option<SeriesId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lot_id: Option<LotId>,
}

impl InstrumentIdentifiers {
    pub fn new(instrument_id: InstrumentId) -> Self {
        Self { instrument_id, template_id: None, series_id: None, lot_id: None }
    }

    pub fn with_template(mut self, template_id: TemplateId) -> Self {
        self.template_id = Some(template_id);
        self
    }

    pub fn with_series(mut self, series_id: SeriesId) -> Self {
        self.series_id = Some(series_id);
        self
    }

    pub fn with_lot(mut self, lot_id: LotId) -> Self {
        self.lot_id = Some(lot_id);
        self
    }
}

impl From<InstrumentId> for InstrumentIdentifiers {
    fn from(value: InstrumentId) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Listability {
    Unlisted,
    Listed(VenueType),
}

impl Listability {
    pub fn listed(venue: VenueType) -> Self {
        Listability::Listed(venue)
    }

    pub fn is_listed(&self) -> bool {
        matches!(self, Listability::Listed(_))
    }

    pub fn should_create_order_book(&self) -> bool {
        matches!(self, Listability::Listed(VenueType::CentralLimitOrderBook))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstrumentCore<S> {
    pub identifiers: InstrumentIdentifiers,
    pub market_profile: MarketProfile,
    pub listability: Listability,
    pub state: S,
}

impl<S> InstrumentCore<S> {
    pub fn new(
        identifiers: InstrumentIdentifiers, market_profile: MarketProfile, listability: Listability, state: S,
    ) -> Self {
        Self { identifiers, market_profile, listability, state }
    }

    pub fn map_state<T>(self, map: impl FnOnce(S) -> T) -> InstrumentCore<T> {
        InstrumentCore {
            identifiers: self.identifiers,
            market_profile: self.market_profile,
            listability: self.listability,
            state: map(self.state),
        }
    }

    pub fn identifiers(&self) -> &InstrumentIdentifiers {
        &self.identifiers
    }

    pub fn identifiers_mut(&mut self) -> &mut InstrumentIdentifiers {
        &mut self.identifiers
    }

    pub fn state(&self) -> &S {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut S {
        &mut self.state
    }

    pub fn into_state(self) -> S {
        self.state
    }
}

impl InstrumentCore<InstrumentType> {
    pub fn face_value(&self) -> Option<Money> {
        self.state.face_value()
    }

    pub fn type_as_string(&self) -> &'static str {
        self.state.type_as_string()
    }
}
