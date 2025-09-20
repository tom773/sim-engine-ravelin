use crate::prelude::*;
use crate::types::instrument::inst_core::RuntimeInstrumentCore;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::HashMap;

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
