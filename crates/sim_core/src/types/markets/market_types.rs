use std::collections::HashMap;
use crate::*;
use super::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use ordered_float::NotNan;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MarketTick {
    pub date: chrono::NaiveDate,
    pub last_price: Option<f64>,
    pub last_qty: Option<f64>,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub spread: Option<f64>,
    pub volume: f64,
    pub turnover: f64,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MarketView {
    pub last: Option<f64>,
    pub mid: Option<f64>,
    pub spread: Option<f64>,
    pub volume: f64,
    pub turnover: f64,
    pub vwap_5: Option<f64>,
    pub ma_20: Option<f64>,
    pub realized_vol_20: Option<f64>,
}

impl MarketView {
    pub fn last_or_mid(&self) -> Option<f64> { self.last.or(self.mid) }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarketSnapshot {
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub spread: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimedTrade {
    pub at: i64,
    pub trade: Trade,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketDepthSummary {
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub bid_size_at_best: f64,
    pub ask_size_at_best: f64,
    #[serde(serialize_with = "crate::notnan_map_as_f64")]
    pub bid_levels: HashMap<NotNan<f64>, f64>,
    #[serde(serialize_with = "crate::notnan_map_as_f64")]
    pub ask_levels: HashMap<NotNan<f64>, f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketSummary {
    pub depth: MarketDepthSummary,
    pub mid: Option<f64>,
    pub spread: Option<f64>,
    pub last_price: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TreasuryMarketSummary {
    pub market_id: FinancialMarketId,
    pub tenor: Tenor,
    pub price: f64,
    pub yield_to_maturity: f64,
    pub spread_bps: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candle {
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub vwap: Option<f64>,
    pub trades_count: u32,
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoodsMarket {
    pub good_id: GoodId,
    pub name: String,
    pub order_book: OrderBook,
}

impl GoodsMarket {
    pub fn new(good_id: GoodId, name: String) -> Self { Self { good_id, name, order_book: OrderBook::new() } }
    pub fn best_ask(&self) -> Option<&Ask> { self.order_book.best_ask() }
    pub fn current_price(&self) -> Option<f64> { self.order_book.representative_price() }
}

impl MarketSnapshotProvider for GoodsMarket {
    fn snapshot(&self) -> MarketSnapshot {
        MarketSnapshot {
            best_bid: self.order_book.best_bid().map(|b| b.price),
            best_ask: self.order_book.best_ask().map(|a| a.price),
            spread: self.order_book.spread(),
        }
    }
}

impl MarketSummaryProvider for GoodsMarket {
    fn summary(&self) -> MarketSummary {
        MarketSummary {
            depth: self.order_book.depth_summary(),
            mid: self.order_book.mid_price(),
            spread: self.order_book.spread(),
            last_price: self.current_price(),
        }
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialMarket {
    pub market_id: FinancialMarketId,
    pub name: String,
    pub order_book: OrderBook,
}

impl FinancialMarket {
    pub fn new(market_id: FinancialMarketId, name: String) -> Self { Self { market_id, name, order_book: OrderBook::new() } }
    pub fn current_price(&self) -> f64 { self.order_book.representative_price().unwrap_or(100.0) }
    pub fn last_or_mid(&self) -> Option<f64> {
        self.order_book.mid_price()
            .or_else(|| self.order_book.best_bid().map(|b| b.price))
            .or_else(|| self.order_book.best_ask().map(|a| a.price))
    }
    pub fn best_ask(&self) -> Option<f64> { self.order_book.best_ask().map(|a| a.price) }
    pub fn best_bid(&self) -> Option<f64> { self.order_book.best_bid().map(|b| b.price) }
    pub fn spread_bps(&self) -> f64 {
        if let Some(spread) = self.order_book.spread() {
            if let Some(mid) = self.order_book.mid_price() {
                if mid > 0.0 { return (spread / mid) * 10000.0; }
            }
            spread * 100.0
        } else {
            0.0
        }
    }
    pub fn calculate_ytm(&self, fs: &FinancialSystem) -> Option<f64> {
        if let FinancialMarketId::Treasury { tenor } = &self.market_id {
            if let Some(bond_details) = fs.exchange.get_treasury_bond_details(fs, tenor) {
                return Some(pricing::ytm_bond(
                    self.current_price(),
                    bond_details.face_value,
                    bond_details.coupon_rate_bps / 10000.0,
                    tenor.to_years(),
                    bond_details.frequency,
                ));
            }
        }
        None
    }
    pub fn calculate_ytm_with_price(&self, fs: &FinancialSystem, price: f64) -> Option<f64> {
        if let FinancialMarketId::Treasury { tenor } = &self.market_id {
            if let Some(bond_details) = fs.exchange.get_treasury_bond_details(fs, tenor) {
                return Some(pricing::ytm_bond(
                    price,
                    bond_details.face_value,
                    bond_details.coupon_rate_bps / 10000.0,
                    tenor.to_years(),
                    bond_details.frequency,
                ));
            }
        }
        None
    }
    pub fn default_yield(&self) -> f64 {
        if let FinancialMarketId::Treasury { tenor } = &self.market_id {
            match tenor {
                Tenor::T2Y => 0.025, Tenor::T5Y => 0.030,
                Tenor::T10Y => 0.035, Tenor::T30Y => 0.040,
            }
        } else {
            0.0
        }
    }
}

impl MarketSnapshotProvider for FinancialMarket {
    fn snapshot(&self) -> MarketSnapshot {
        MarketSnapshot {
            best_bid: self.order_book.best_bid().map(|b| b.price),
            best_ask: self.order_book.best_ask().map(|a| a.price),
            spread: self.order_book.spread(),
        }
    }
}

impl MarketSummaryProvider for FinancialMarket {
    fn summary(&self) -> MarketSummary {
        MarketSummary {
            depth: self.order_book.depth_summary(),
            mid: self.order_book.mid_price(),
            spread: self.order_book.spread(),
            last_price: self.order_book.representative_price(),
        }
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobOffer {
    pub offer_id: Uuid,
    pub firm_id: AgentId,
    pub wage_rate: f64,
    pub hours_required: f64,
    pub quantity: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobApplication {
    pub application_id: Uuid,
    pub consumer_id: AgentId,
    pub reservation_wage: f64,
    pub hours_desired: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LabourMarket {
    pub market_id: LabourMarketId,
    pub name: String,
    pub job_offers: Vec<JobOffer>,
    pub job_applications: Vec<JobApplication>,
}

impl LabourMarket {
    pub fn clear_and_match(&mut self, state: &SimState) -> Vec<StateEffect> {
        let mut effects = Vec::new();
        self.job_applications.sort_by(|a, b| a.reservation_wage.partial_cmp(&b.reservation_wage).unwrap_or(std::cmp::Ordering::Equal));
        self.job_offers.sort_by(|a, b| b.wage_rate.partial_cmp(&a.wage_rate).unwrap_or(std::cmp::Ordering::Equal));

        let mut filled_application_ids = Vec::new();
        let mut offers = self.job_offers.clone();

        for application in &self.job_applications {
            if state.agents.consumers.get(&application.consumer_id).map_or(true, |c| c.employed_by.is_some()) {
                continue;
            }
            for offer in offers.iter_mut() {
                if offer.quantity > 0 && offer.wage_rate >= application.reservation_wage {
                    let contract = EmploymentContract {
                        employee_id: application.consumer_id,
                        wage_rate: offer.wage_rate,
                        hours: application.hours_desired.min(offer.hours_required),
                        start_date: state.current_date,
                    };
                    effects.push(StateEffect::Agent(AgentEffect::EstablishEmployment {
                        firm_id: offer.firm_id,
                        consumer_id: application.consumer_id,
                        contract,
                    }));
                    offer.quantity -= 1;
                    filled_application_ids.push(application.application_id);
                    break;
                }
            }
        }
        effects.push(StateEffect::Market(MarketEffect::ClearLabourMarketOrders {
            market_id: self.market_id.clone(),
            filled_applications: filled_application_ids.clone(),
        }));
        self.job_applications.retain(|app| !filled_application_ids.contains(&app.application_id));
        self.job_offers = offers;
        effects
    }
}