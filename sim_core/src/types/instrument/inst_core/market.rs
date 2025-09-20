use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VenueType {
    CentralLimitOrderBook,
    PostedRates,
    OTC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoneyMarketSegment {
    Interbank,
    SovereignShortTerm,
    CorporateShortTerm,
    Repo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapitalMarketSegment {
    Equity,
    SovereignLongTerm,
    CorporateCredit,
    StructuredFinance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DerivativesMarketSegment {
    Options,
    Futures,
    Swaps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstrumentMarket {
    MoneyMarket(MoneyMarketSegment),
    CapitalMarket(CapitalMarketSegment),
    DerivativesMarket(DerivativesMarketSegment),
    Unlisted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketProfile {
    pub market: InstrumentMarket,
    pub default_venue_type: Option<VenueType>,
    pub is_exchange_tradeable: bool,
    pub requires_csd_custody: bool,
}

impl MarketProfile {
    pub fn unlisted() -> Self {
        Self {
            market: InstrumentMarket::Unlisted,
            default_venue_type: None,
            is_exchange_tradeable: false,
            requires_csd_custody: false,
        }
    }

    pub fn from_market(market: InstrumentMarket) -> Self {
        let (default_venue_type, is_exchange_tradeable, requires_csd_custody) = match market {
            InstrumentMarket::MoneyMarket(_) => (Some(VenueType::PostedRates), true, false),
            InstrumentMarket::CapitalMarket(_) => (Some(VenueType::CentralLimitOrderBook), true, true),
            InstrumentMarket::DerivativesMarket(_) => (Some(VenueType::CentralLimitOrderBook), true, true),
            InstrumentMarket::Unlisted => (None, false, false),
        };

        Self { market, default_venue_type, is_exchange_tradeable, requires_csd_custody }
    }
}
