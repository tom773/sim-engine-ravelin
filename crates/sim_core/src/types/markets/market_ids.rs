use crate::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;
use super::market_traits::{RatesMarket, Tradable};

impl Tradable for GoodId {
    fn check_holdings(
        &self,
        agent_id: &AgentId,
        quantity: f64,
        fs: &FinancialSystem,
    ) -> Result<(), String> {
        let bs = fs.get_bs_by_id(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
        let available = bs.get_inventory().and_then(|inv| inv.get(self)).map_or(0.0, |item| item.quantity);
        if available < quantity {
            Err(format!(
                "Insufficient inventory for GoodId({:?}): have {:.2}, need {:.2}",
                self.0, available, quantity
            ))
        } else {
            Ok(())
        }
    }
}

impl Tradable for FinancialMarketId {
    fn check_holdings(
        &self,
        agent_id: &AgentId,
        quantity: f64,
        fs: &FinancialSystem,
    ) -> Result<(), String> {
        match self {
            FinancialMarketId::FederalFundsOvernight | FinancialMarketId::TreasuryRepoOvernight => {
                let reserves = fs.get_bank_reserves(agent_id).unwrap_or(0.0);
                if reserves < quantity {
                    let market_name = match self {
                        FinancialMarketId::FederalFundsOvernight => "federal funds",
                        FinancialMarketId::TreasuryRepoOvernight => "Treasury repo",
                        _ => "overnight funding",
                    };
                    Err(format!(
                        "Insufficient reserves for {} ask (lending): need ${:.2}, has ${:.2}",
                        market_name, quantity, reserves
                    ))
                } else {
                    Ok(())
                }
            }
            FinancialMarketId::Treasury { tenor } => {
                let bs = fs.get_bs_by_id(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
                let held_quantity = bs
                    .assets
                    .values()
                    .map(|inst| {
                        if let Some(bond_details) = inst.details.as_any().downcast_ref::<BondDetails>() {
                            if bond_details.bond_type == BondType::Government && &bond_details.tenor == tenor {
                                bond_details.quantity as f64
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        }
                    })
                    .sum::<f64>();

                if held_quantity < quantity {
                    Err(format!(
                        "Insufficient Treasury holdings ({:?}): need {:.0}, has {:.0}",
                        tenor, quantity, held_quantity
                    ))
                } else {
                    Ok(())
                }
            }
            FinancialMarketId::CorporateBond { .. }
            | FinancialMarketId::DiscountWindow
            | FinancialMarketId::StandingRepoFacility
            | FinancialMarketId::OvernightReverseRepo => Ok(()),
        }
    }
}

impl Tradable for MarketId {
    fn check_holdings(&self, agent_id: &AgentId, quantity: f64, fs: &FinancialSystem) -> Result<(), String> {
        match self {
            MarketId::Goods(good_id) => good_id.check_holdings(agent_id, quantity, fs),
            MarketId::Financial(fin_id) => fin_id.check_holdings(agent_id, quantity, fs),
            MarketId::Labour(_) => Err("Labour market holdings check not implemented".to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LabourMarketId {
    GeneralLabour,
}

impl fmt::Display for LabourMarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LabourMarketId::GeneralLabour => write!(f, "GeneralLabour"),
        }
    }
}

impl FromStr for LabourMarketId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GeneralLabour" => Ok(LabourMarketId::GeneralLabour),
            _ => Err(format!("Unknown LabourMarketId: {}", s)),
        }
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MarketId {
    Goods(GoodId),
    Financial(FinancialMarketId),
    Labour(LabourMarketId),
}

impl std::hash::Hash for MarketId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            MarketId::Goods(id) => { 0.hash(state); id.hash(state); }
            MarketId::Financial(id) => { 1.hash(state); id.hash(state); }
            MarketId::Labour(id) => { 2.hash(state); id.hash(state); }
        }
    }
}

impl std::cmp::PartialEq for MarketId {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MarketId::Goods(id1), MarketId::Goods(id2)) => id1 == id2,
            (MarketId::Financial(id1), MarketId::Financial(id2)) => id1 == id2,
            (MarketId::Labour(id1), MarketId::Labour(id2)) => id1 == id2,
            _ => false,
        }
    }
}

impl std::cmp::Eq for MarketId {}

impl std::fmt::Display for MarketId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketId::Goods(id) => write!(f, "Goods({})", id),
            MarketId::Financial(id) => write!(f, "Financial({})", id),
            MarketId::Labour(id) => write!(f, "Labour({})", id),
        }
    }
}

#[derive(Debug, Error)]
pub enum ParseMarketIdError {
    #[error("Invalid MarketId format: {0}")]
    InvalidFormat(String),
    #[error("Failed to parse GoodId: {0}")]
    ParseGoodId(String),
    #[error("Failed to parse FinancialMarketId: {0}")]
    ParseFinancialMarketId(#[from] ParseFinancialMarketIdError),
    #[error("Failed to parse LabourMarketId: {0}")]
    ParseLabourMarketId(String),
}

impl std::str::FromStr for MarketId {
    type Err = ParseMarketIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(content) = s.strip_prefix("Goods(").and_then(|s| s.strip_suffix(')')) {
            let id = content.parse().map_err(|_| ParseMarketIdError::ParseGoodId(content.to_string()))?;
            return Ok(MarketId::Goods(id));
        }
        if let Some(content) = s.strip_prefix("Financial(").and_then(|s| s.strip_suffix(')')) {
            let id = content.parse()?;
            return Ok(MarketId::Financial(id));
        }
        if let Some(content) = s.strip_prefix("Labour(").and_then(|s| s.strip_suffix(')')) {
            let id = content.parse().map_err(|e| ParseMarketIdError::ParseLabourMarketId(e))?;
            return Ok(MarketId::Labour(id));
        }
        Err(ParseMarketIdError::InvalidFormat(s.to_string()))
    }
}

impl Default for MarketId {
    fn default() -> Self {
        MarketId::Goods(GoodId::default())
    }
}


#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy)]
pub enum Tenor {
    T2Y, T5Y, T10Y, T30Y,
}

impl Tenor {
    pub fn to_days(&self) -> u32 {
        match self { Tenor::T2Y => 730, Tenor::T5Y => 1825, Tenor::T10Y => 3650, Tenor::T30Y => 10950 }
    }
    pub fn to_years(&self) -> f64 {
        match self { Tenor::T2Y => 2.0, Tenor::T5Y => 5.0, Tenor::T10Y => 10.0, Tenor::T30Y => 30.0 }
    }
    pub fn add_to_date(&self, date: chrono::NaiveDate) -> chrono::NaiveDate {
        date + chrono::Duration::days(self.to_days() as i64)
    }
    pub fn periods(&self, frequency: usize) -> usize {
        let years = match self { Tenor::T2Y => 2, Tenor::T5Y => 5, Tenor::T10Y => 10, Tenor::T30Y => 30 };
        years * frequency
    }
}

impl fmt::Display for Tenor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{:?}", self) }
}

#[derive(Debug, Error)]
#[error("Invalid Tenor string: {0}")]
pub struct ParseTenorError(String);

impl FromStr for Tenor {
    type Err = ParseTenorError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "T2Y" => Ok(Tenor::T2Y), "T5Y" => Ok(Tenor::T5Y),
            "T10Y" => Ok(Tenor::T10Y), "T30Y" => Ok(Tenor::T30Y),
            _ => Err(ParseTenorError(s.to_string())),
        }
    }
}


#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FinancialMarketId {
    FederalFundsOvernight,
    TreasuryRepoOvernight,
    Treasury { tenor: Tenor },
    CorporateBond { rating: CreditRating },
    DiscountWindow,
    StandingRepoFacility,
    OvernightReverseRepo,
}

impl fmt::Display for FinancialMarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FinancialMarketId::FederalFundsOvernight => write!(f, "FedFunds_ON"),
            FinancialMarketId::TreasuryRepoOvernight => write!(f, "TreasuryRepo_ON"),
            FinancialMarketId::Treasury { tenor } => write!(f, "Treasury_{}", tenor),
            FinancialMarketId::CorporateBond { rating } => write!(f, "CorpBond_{}", rating),
            FinancialMarketId::DiscountWindow => write!(f, "DiscountWindow"),
            FinancialMarketId::StandingRepoFacility => write!(f, "SRF"),
            FinancialMarketId::OvernightReverseRepo => write!(f, "ON_RRP"),
        }
    }
}

#[derive(Debug, Error)]
pub enum ParseFinancialMarketIdError {
    #[error("Invalid FinancialMarketId string format: {0}")]
    InvalidFormat(String),
    #[error("Failed to parse tenor: {0}")]
    ParseTenor(#[from] ParseTenorError),
    #[error("Failed to parse credit rating: {0}")]
    ParseRating(#[from] ParseCreditRatingError),
}

impl FromStr for FinancialMarketId {
    type Err = ParseFinancialMarketIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "FedFunds_ON" => Ok(FinancialMarketId::FederalFundsOvernight),
            "TreasuryRepo_ON" => Ok(FinancialMarketId::TreasuryRepoOvernight),
            "DiscountWindow" => Ok(FinancialMarketId::DiscountWindow),
            "SRF" => Ok(FinancialMarketId::StandingRepoFacility),
            "ON_RRP" => Ok(FinancialMarketId::OvernightReverseRepo),
            "SOFR" => Ok(FinancialMarketId::TreasuryRepoOvernight),
            _ => {
                if let Some(tenor_str) = s.strip_prefix("Treasury_") {
                    return Ok(FinancialMarketId::Treasury { tenor: tenor_str.parse()? });
                }
                if let Some(rating_str) = s.strip_prefix("CorpBond_") {
                    return Ok(FinancialMarketId::CorporateBond { rating: rating_str.parse()? });
                }
                Err(ParseFinancialMarketIdError::InvalidFormat(s.to_string()))
            }
        }
    }
}

impl RatesMarket for FinancialMarketId {
    fn price_to_daily_rate(&self, price: f64) -> f64 {
        match self {
            FinancialMarketId::FederalFundsOvernight | FinancialMarketId::TreasuryRepoOvernight => {
                if price <= 0.0 { return f64::INFINITY; }
                (1.0 / price) - 1.0
            }
            _ => 0.0,
        }
    }
    fn daily_rate_to_annual_bps(&self, daily_rate: f64) -> BasisPoints {
        match self {
            FinancialMarketId::FederalFundsOvernight | FinancialMarketId::TreasuryRepoOvernight => {
                decimal_to_bps(daily_rate * 360.0)
            }
            _ => 0.0,
        }
    }
    fn annual_bps_to_daily_rate(&self, annual_bps: BasisPoints) -> f64 {
        match self {
            FinancialMarketId::FederalFundsOvernight | FinancialMarketId::TreasuryRepoOvernight => {
                bps_to_decimal(annual_bps) / 360.0
            }
            _ => 0.0,
        }
    }
}