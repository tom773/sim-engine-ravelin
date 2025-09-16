use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum OvernightVenue {
    FedFundsON,
    RepoGC1D,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ONQuoteSide {
    Lend,
    Borrow,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ONQuote {
    pub venue: OvernightVenue,
    pub agent: AgentId,
    pub side: ONQuoteSide,
    pub notional: f64,
    pub limit_rate_bps: BasisPoints,
    pub haircut: Option<Rate>,
    pub preferred_collateral: Option<Vec<InstrumentId>>,
    pub min_fill: f64,
    pub ts: u64,
}
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct OvernightFundingBooks {
    pub fedfunds_on: Vec<ONQuote>,
    pub repo_gc1d: Vec<ONQuote>,
}

impl OvernightFundingBooks {
    pub fn post_quote(&mut self, quote: ONQuote) {
        match quote.venue {
            OvernightVenue::FedFundsON => self.fedfunds_on.push(quote),
            OvernightVenue::RepoGC1D => self.repo_gc1d.push(quote),
        }
    }
    pub fn take_fedfunds(&mut self) -> Vec<ONQuote> {
        std::mem::take(&mut self.fedfunds_on)
    }
    pub fn take_repo_gc1d(&mut self) -> Vec<ONQuote> {
        std::mem::take(&mut self.repo_gc1d)
    }
}
