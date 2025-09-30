use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use sim_core::actions::monetary::{MonetaryAction, OMOType};
use sim_core::prelude::*;
use sim_core::types::markets::market::{FinancialProduct, MarketGeneric, MarketType, listing_key_from_instrument};
use sim_core::types::markets::orderbook::MarketDepthSummary;
use std::collections::HashMap;
use uuid::Uuid;

use super::agent_digest::instrument_label;
use super::state_digest::{InstrumentRegistryDigest, rate_to_f64};

pub(crate) const ORDERBOOK_DEPTH_LEVELS: usize = 10;
pub(crate) const OMO_HISTORY_LIMIT: usize = 25;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketsDigest {
    pub snapshots: Vec<MarketDigest>,
    pub most_active: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infrastructure: Option<MarketInfrastructureDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketInfrastructureDigest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub listings: Vec<MarketListingDigest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub omo_actions: Vec<OmoActionDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketListingDigest {
    pub symbol: String,
    pub kind: MarketKindDigest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instrument_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub good_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labour_market_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listing_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OmoActionDigest {
    pub action_id: Uuid,
    pub tick: u32,
    pub agent_id: Uuid,
    pub description: String,
    pub amount: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterparty: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_bps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketDigest {
    pub market_id: String,
    pub label: String,
    pub kind: MarketKindDigest,
    pub last_price: Option<f64>,
    pub mid_price: Option<f64>,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub spread: Option<f64>,
    pub volume: f64,
    pub turnover: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<DepthDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_mid_bps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_bid_bps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_ask_bps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_last_bps: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MarketKindDigest {
    #[default]
    Financial,
    Goods,
    Labour,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DepthDigest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bids: Vec<DepthLevel>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub asks: Vec<DepthLevel>,
    pub bid_size_at_best: f64,
    pub ask_size_at_best: f64,
    pub total_bid_levels: usize,
    pub total_ask_levels: usize,
    pub total_bid_volume: f64,
    pub total_ask_volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DepthLevel {
    pub price: f64,
    pub quantity: f64,
}

pub(crate) fn compute_markets(state: &SimState) -> MarketsDigest {
    let mut snapshots: Vec<MarketDigest> = Vec::new();

    for (symbol, market) in &state.financial_system.exchange.markets {
        match market {
            MarketType::Financial(fin_market) => {
                let inst_id = &fin_market.key;
                let view = state.market_view(symbol).unwrap_or_default();
                let (yield_mid, yield_last, yield_bid, yield_ask) = calculate_yields(inst_id, fin_market);
                let depth = depth_from_summary(fin_market.book.depth_summary(), ORDERBOOK_DEPTH_LEVELS);
                let label = state
                    .financial_system
                    .instruments
                    .instruments
                    .get(inst_id)
                    .map(|i| instrument_label(i, &state.financial_system))
                    .unwrap_or_else(|| "Financial Market".into());

                snapshots.push(MarketDigest {
                    market_id: symbol.to_string(),
                    label,
                    kind: MarketKindDigest::Financial,
                    last_price: view.last,
                    mid_price: view.mid,
                    best_bid: fin_market.book.best_bid().map(|m| m.to_f64()),
                    best_ask: fin_market.book.best_ask().map(|m| m.to_f64()),
                    spread: fin_market.book.spread().map(|m| m.to_f64()),
                    volume: view.volume,
                    turnover: view.turnover,
                    depth,
                    yield_mid_bps: yield_mid,
                    yield_last_bps: yield_last,
                    yield_bid_bps: yield_bid,
                    yield_ask_bps: yield_ask,
                });
            }
            MarketType::Goods(goods_market) => {
                let view = state.market_view(symbol).unwrap_or_default();
                let depth = depth_from_summary(goods_market.book.depth_summary(), ORDERBOOK_DEPTH_LEVELS);
                let label = state
                    .financial_system
                    .goods
                    .goods
                    .get(&goods_market.key)
                    .map(|g| g.name.clone())
                    .unwrap_or_else(|| "Goods Market".into());

                snapshots.push(MarketDigest {
                    market_id: symbol.to_string(),
                    label,
                    kind: MarketKindDigest::Goods,
                    last_price: view.last,
                    mid_price: view.mid,
                    best_bid: goods_market.book.best_bid().map(|m| m.to_f64()),
                    best_ask: goods_market.book.best_ask().map(|m| m.to_f64()),
                    spread: goods_market.book.spread().map(|m| m.to_f64()),
                    volume: view.volume,
                    turnover: view.turnover,
                    depth,
                    yield_mid_bps: None,
                    yield_last_bps: None,
                    yield_bid_bps: None,
                    yield_ask_bps: None,
                });
            }
            MarketType::Labour(_labour_market) => {
                snapshots.push(MarketDigest {
                    market_id: symbol.to_string(),
                    label: "Labour Market".into(),
                    kind: MarketKindDigest::Labour,
                    last_price: None,
                    mid_price: None,
                    best_bid: None,
                    best_ask: None,
                    spread: None,
                    volume: 0.0,
                    turnover: 0.0,
                    depth: None,
                    yield_mid_bps: None,
                    yield_last_bps: None,
                    yield_bid_bps: None,
                    yield_ask_bps: None,
                });
            }
        }
    }

    snapshots.sort_by(|a, b| b.volume.partial_cmp(&a.volume).unwrap_or(std::cmp::Ordering::Equal));
    let most_active: Vec<String> = snapshots.iter().take(5).map(|m| m.market_id.clone()).collect();

    MarketsDigest { snapshots, most_active, infrastructure: None }
}

fn calculate_yields(inst_id: &InstrumentId, market: &MarketGeneric<FinancialProduct>) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    let mid = market
        .book
        .mid_price()
        .and_then(|price| market.pricer.yield_from_price(inst_id, price))
        .and_then(|r| r.to_f64());
    let last = market
        .book
        .last_price
        .and_then(|price| market.pricer.yield_from_price(inst_id, price))
        .and_then(|r| r.to_f64());
    let bid = market
        .book
        .best_bid()
        .and_then(|price| market.pricer.yield_from_price(inst_id, price))
        .and_then(|r| r.to_f64());
    let ask = market
        .book
        .best_ask()
        .and_then(|price| market.pricer.yield_from_price(inst_id, price))
        .and_then(|r| r.to_f64());
    (mid, last, bid, ask)
}

fn depth_from_summary(summary: MarketDepthSummary, max_levels: usize) -> Option<DepthDigest> {
    if summary.bid_levels.is_empty() && summary.ask_levels.is_empty() {
        return None;
    }

    fn ordered_levels(levels: &HashMap<Decimal, f64>, descending: bool, limit: usize) -> Vec<DepthLevel> {
        let mut pairs: Vec<_> = levels.iter().map(|(price, qty)| (*price, *qty)).collect();
        if descending {
            pairs.sort_by(|a, b| b.0.cmp(&a.0));
        } else {
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
        }
        pairs
            .into_iter()
            .take(limit)
            .map(|(price, quantity)| DepthLevel { price: price.to_f64().unwrap_or_default(), quantity })
            .collect()
    }

    let MarketDepthSummary { bid_levels, ask_levels, bid_size_at_best, ask_size_at_best, .. } = summary;

    let bids = ordered_levels(&bid_levels, true, max_levels);
    let asks = ordered_levels(&ask_levels, false, max_levels);

    let total_bid_volume: f64 = bid_levels.values().copied().sum();
    let total_ask_volume: f64 = ask_levels.values().copied().sum();

    Some(DepthDigest {
        bids,
        asks,
        bid_size_at_best,
        ask_size_at_best,
        total_bid_levels: bid_levels.len(),
        total_ask_levels: ask_levels.len(),
        total_bid_volume,
        total_ask_volume,
    })
}

pub(crate) fn build_market_infrastructure(
    state: &SimState, markets: &MarketsDigest, registry: &InstrumentRegistryDigest,
) -> MarketInfrastructureDigest {
    let exchange = &state.financial_system.exchange;
    let mut listings: Vec<MarketListingDigest> = Vec::with_capacity(exchange.markets.len());

    for (symbol, market) in &exchange.markets {
        let symbol_str = symbol.to_string();
        let mut listing = MarketListingDigest {
            symbol: symbol_str.clone(),
            kind: match market {
                MarketType::Financial(_) => MarketKindDigest::Financial,
                MarketType::Goods(_) => MarketKindDigest::Goods,
                MarketType::Labour(_) => MarketKindDigest::Labour,
            },
            instrument_id: None,
            good_id: None,
            labour_market_id: None,
            label: None,
            listing_key: None,
        };

        match market {
            MarketType::Financial(fin_market) => {
                let inst_id = fin_market.key;
                listing.instrument_id = Some(inst_id.to_string());
                if let Some(inst) = state.financial_system.instruments.instruments.get(&inst_id) {
                    listing.label = Some(inst.label().to_string());
                    listing.listing_key = Some(format!("{:?}", listing_key_from_instrument(inst)));
                } else if let Some(label) = registry
                    .instruments
                    .iter()
                    .find(|meta| meta.instrument_id == inst_id.to_string())
                    .map(|meta| meta.label.clone())
                {
                    listing.label = Some(label);
                }
            }
            MarketType::Goods(goods_market) => {
                let good_id = goods_market.key;
                listing.good_id = Some(good_id.to_string());
                listing.label = registry
                    .goods
                    .iter()
                    .find(|good| good.good_id == good_id.to_string())
                    .map(|good| good.name.clone())
                    .or_else(|| state.financial_system.goods.goods.get(&good_id).map(|g| g.name.clone()));
            }
            MarketType::Labour(_labour_market) => {
                if let Some((labour_id, _)) =
                    exchange.labour_to_symbol.iter().find(|(_, registered_symbol)| *registered_symbol == symbol)
                {
                    listing.labour_market_id = Some(labour_id.to_string());
                }
                if listing.label.is_none() {
                    listing.label = Some("Labour Market".into());
                }
            }
        }

        if listing.label.is_none() {
            listing.label = markets
                .snapshots
                .iter()
                .find(|snapshot| snapshot.market_id == listing.symbol)
                .map(|snapshot| snapshot.label.clone());
        }

        listings.push(listing);
    }

    listings.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    let omo_actions = collect_recent_omo_actions(&state.history, OMO_HISTORY_LIMIT);

    MarketInfrastructureDigest { listings, omo_actions }
}

pub(crate) fn collect_recent_omo_actions(history: &SimHistory, limit: usize) -> Vec<OmoActionDigest> {
    let mut actions: Vec<OmoActionDigest> = Vec::new();

    for record in history.tick_records.iter().rev() {
        for action in &record.actions {
            if let SimAction::Monetary(MonetaryAction::OpenMarketOperation { cb_id, operation_type, amount }) =
                &action.action
            {
                let (rate_bps, term_days) = match operation_type {
                    OMOType::QuantitativeEasing | OMOType::QuantitativeTightening => (None, None),
                    OMOType::Repo { rate_bps, term_days } | OMOType::ReverseRepo { rate_bps, term_days } => {
                        (Some(rate_to_f64(*rate_bps)), Some(*term_days))
                    }
                };

                actions.push(OmoActionDigest {
                    action_id: action.id,
                    tick: record.tick_number,
                    agent_id: cb_id.0,
                    description: operation_type.description(),
                    amount: *amount,
                    counterparty: None,
                    rate_bps,
                    term_days,
                });
            }
        }

        if actions.len() >= limit {
            break;
        }
    }

    actions.sort_by(|a, b| a.tick.cmp(&b.tick));
    if actions.len() > limit {
        actions = actions.split_off(actions.len() - limit);
    }
    actions
}

