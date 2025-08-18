use crate::{AppState, dto::*, routes::*};
use std::str::FromStr;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use serde_json::json;
use sim_core::*;
use std::sync::Arc;

pub async fn get_markets_dto(State(state): State<Arc<AppState>>) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;

    if let Some(engine) = engine_guard.as_ref() {
        let mut treasuries: Vec<TreasuryMarketDto> = Vec::new();

        for (market_key, market) in &engine.state.financial_system.exchange.financial_markets {
            if let FinancialMarketId::Treasury { tenor } = market_key {
                let price = market.current_price();

                let ytm =
                    market.calculate_ytm(&engine.state.financial_system).unwrap_or_else(|| market.default_yield())
                        * 100.0;

                let spread = market.spread_bps();

                treasuries.push(TreasuryMarketDto {
                    instrument_id: format!("Financial(Treasury_{})", tenor),
                    name: format!("US {}", tenor.to_string().replace("T", "").replace("Y", "Y")),
                    price: price / 10.0,
                    yield_to_maturity: ytm,
                    spread_bps: spread,
                    daily_change_pct: 0.001,
                });
            }
        }

        let yield_curve: Vec<YieldCurvePointDto> = treasuries
            .iter()
            .map(|treasury| YieldCurvePointDto {
                tenor: treasury.name.replace("US ", ""),
                yield_pct: treasury.yield_to_maturity,
            })
            .collect();

        let fed_funds_rate = engine
            .state
            .financial_system
            .exchange
            .financial_markets
            .get(&FinancialMarketId::FederalFundsOvernight)
            .and_then(|market| market.last_or_mid())
            .map(|price| {
                let daily_rate = FinancialMarketId::FederalFundsOvernight.price_to_daily_rate(price);
                daily_rate * 360.0 * 100.0
            })
            .unwrap_or(engine.state.financial_system.central_bank.policy_rate_bps / 100.0);

        let sofr = engine
            .state
            .financial_system
            .exchange
            .financial_markets
            .get(&FinancialMarketId::TreasuryRepoOvernight)
            .and_then(|market| market.last_or_mid())
            .map(|price| {
                let daily_rate = FinancialMarketId::TreasuryRepoOvernight.price_to_daily_rate(price);
                daily_rate * 360.0 * 100.0
            })
            .unwrap_or(fed_funds_rate - 0.10);

        let overnight_rates = OvernightRatesDto {
            effr: engine
                .state
                .financial_system
                .exchange
                .financial_markets
                .get(&FinancialMarketId::FederalFundsOvernight)
                .and_then(|market| market.last_or_mid())
                .map(|price| {
                    let daily_rate = FinancialMarketId::FederalFundsOvernight.price_to_daily_rate(price);
                    daily_rate * 360.0 * 100.0
                }),
            sofr: Some(sofr),
            iorb: Some((engine.state.financial_system.central_bank.policy_rate_bps + 15.0) / 100.0),
            discount_rate: Some((engine.state.financial_system.central_bank.policy_rate_bps + 25.0) / 100.0),
            overnight_RRP: Some((engine.state.financial_system.central_bank.policy_rate_bps).max(0.0) / 100.0),
        };

        let markets_dto = MarketsPageDto { treasuries, yield_curve, overnight_rates };

        (StatusCode::OK, headers, Json(serde_json::to_value(markets_dto).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(serde_json::json!({ "error": err })))
    }
}
pub async fn get_goods_markets_dto(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;

    if let Some(engine) = engine_guard.as_ref() {
        let goods = engine
            .state
            .financial_system
            .goods
            .goods
            .values()
            .map(|good| GoodsDto { id: good.id.to_string(), name: good.name.clone(), unit: good.unit.clone() })
            .collect::<Vec<_>>();
        let recipies = engine
            .state
            .financial_system
            .goods
            .recipes
            .values()
            .map(|recipe| RecipiesDto {
                id: recipe.id.to_string(),
                name: recipe.name.clone(),
                inputs: recipe
                    .inputs
                    .iter()
                    .map(|(good_id, _)| {
                        let good = engine.state.financial_system.goods.get_good_by_id(good_id).unwrap();
                        GoodsDto { id: good.id.to_string(), name: good.name.clone(), unit: good.unit.clone() }
                    })
                    .collect(),
                output: engine.state.financial_system.goods.get_good_by_id(&recipe.output.0).map_or_else(
                    || GoodsDto { id: String::new(), name: String::new(), unit: String::new() },
                    |good| GoodsDto { id: good.id.to_string(), name: good.name.clone(), unit: good.unit.clone() },
                ),
                efficiency: recipe.efficiency,
                labour_hours: recipe.labour_hours,
            })
            .collect::<Vec<_>>();
        let goods_page = GoodsPageDto { goods, recipies };
        (StatusCode::OK, headers, Json(serde_json::to_value(goods_page).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}

pub async fn get_goods_market_summaries(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;

    if let Some(engine) = engine_guard.as_ref() {
        let exchange = &engine.state.financial_system.exchange;
        let goods = &engine.state.financial_system.goods;

        let markets = exchange
            .goods_markets
            .iter()
            .map(|(good_id, market)| {
                let best_bid = market.order_book.best_bid().map(|b| b.price);
                let best_ask = market.best_ask().map(|a| a.price);
                let spread = match (best_bid, best_ask) {
                    (Some(b), Some(a)) => Some(a - b),
                    _ => None,
                };
                let mid = match (best_bid, best_ask) {
                    (Some(b), Some(a)) => Some((a + b) * 0.5),
                    _ => None,
                };
                let last = market.current_price();

                let bid_px = best_bid;
                let ask_px = best_ask;
                let bid_size_at_best = bid_px
                    .map(|px| market.order_book.bids.iter().filter(|b| b.price == px).map(|b| b.quantity).sum())
                    .unwrap_or(0.0);
                let ask_size_at_best = ask_px
                    .map(|px| market.order_book.asks.iter().filter(|a| a.price == px).map(|a| a.quantity).sum())
                    .unwrap_or(0.0);

                let g = goods.get_good_by_id(good_id);
                let name = g.map(|x| x.name.clone()).unwrap_or_else(|| format!("{good_id}"));
                let unit = g.map(|x| x.unit.clone()).unwrap_or_else(|| "".to_string());

                crate::dto::GoodsMarketSummaryDto {
                    market_id: format!("Goods({good_id})"),
                    good_id: good_id.to_string(),
                    name,
                    unit,
                    best_bid,
                    best_ask,
                    spread,
                    mid,
                    last,
                    depth: crate::dto::DepthDto {
                        bid_size_at_best,
                        ask_size_at_best,
                        bid_levels: market.order_book.bids.len(),
                        ask_levels: market.order_book.asks.len(),
                    },
                }
            })
            .collect::<Vec<_>>();

        let page = crate::dto::GoodsMarketsPageDto { markets };
        (StatusCode::OK, headers, Json(serde_json::to_value(page).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}

pub async fn get_goods_orderbook(
    State(state): State<Arc<AppState>>, Path(good_id_str): Path<String>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;

    if let Some(engine) = engine_guard.as_ref() {
        let good_id: GoodId = good_id_str.parse().map_err(|_| ()).unwrap_or_else(|_| GoodId::from_slug(&good_id_str));
        if let Some(market) = engine.state.financial_system.exchange.goods_market(&good_id) {
            let book = &market.order_book;
            let bids = book
                .bids
                .iter()
                .map(|b| crate::dto::MarketBidDto {
                    agent_id: b.agent_id.to_string(),
                    quantity: b.quantity,
                    price: b.price,
                })
                .collect::<Vec<_>>();

            let asks = book
                .asks
                .iter()
                .map(|a| crate::dto::MarketAskDto {
                    agent_id: a.agent_id.to_string(),
                    quantity: a.quantity,
                    price: a.price,
                })
                .collect::<Vec<_>>();

            let dto = crate::dto::OrderbookDto {
                market_id: format!("Goods({good_id})"),
                market_name: market.name.clone(),
                bids,
                asks,
            };
            (StatusCode::OK, headers, Json(serde_json::to_value(dto).unwrap()))
        } else {
            let err = ApiError { code: "NOT_FOUND", message: "Goods market not found." };
            (StatusCode::NOT_FOUND, headers, Json(json!({ "error": err })))
        }
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<usize>,
    pub bucket_secs: Option<i64>,
}

pub async fn get_goods_market_history(
    State(state): State<Arc<AppState>>, Path(good_id_str): Path<String>, Query(q): Query<HistoryQuery>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;

    if let Some(engine) = engine_guard.as_ref() {
        let good_id: GoodId = good_id_str.parse().map_err(|_| ()).unwrap_or_else(|_| GoodId::from_slug(&good_id_str));
        let goods = &engine.state.financial_system.goods;
        let name = goods.get_good_name(&good_id).unwrap_or("Unknown").to_string();

        let tape = engine
            .state
            .financial_system
            .exchange
            .trade_tape
            .get(&sim_core::markets::MarketId::Goods(good_id))
            .cloned()
            .unwrap_or_default();

        let limit = q.limit.unwrap_or(200);
        let trades = tape
            .iter()
            .rev()
            .take(limit)
            .rev()
            .map(|t| crate::dto::TradeDto {
                ts: t.at,
                price: t.trade.price,
                quantity: t.trade.quantity,
                buyer_id: t.trade.buyer.to_string(),
                seller_id: t.trade.seller.to_string(),
            })
            .collect::<Vec<_>>();

        let bucket = q.bucket_secs.unwrap_or(0);
        let candles = if bucket > 0 {
            use std::collections::BTreeMap;
            let mut buckets: BTreeMap<i64, Vec<&sim_core::markets::Trade>> = BTreeMap::new();
            for t in &tape {
                let k = (t.at / bucket) * bucket;
                buckets.entry(k).or_default().push(&t.trade);
            }
            buckets
                .into_iter()
                .rev()
                .take(limit)
                .rev()
                .map(|(bucket_ts, group)| {
                    let open = group.first().map(|x| x.price).unwrap_or(0.0);
                    let close = group.last().map(|x| x.price).unwrap_or(open);
                    let high = group.iter().map(|x| x.price).fold(f64::MIN, f64::max);
                    let low = group.iter().map(|x| x.price).fold(f64::MAX, f64::min);
                    let volume = group.iter().map(|x| x.quantity).sum::<f64>();
                    crate::dto::CandleDto { ts: bucket_ts, open, high, low, close, volume }
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let dto = crate::dto::MarketHistoryDto {
            market_id: format!("Goods({good_id})"),
            good_id: good_id.to_string(),
            name,
            trades,
            candles,
        };
        (StatusCode::OK, headers, Json(serde_json::to_value(dto).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}

pub async fn get_financial_market_summaries(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;

    if let Some(engine) = engine_guard.as_ref() {
        let exchange = &engine.state.financial_system.exchange;

        let markets = exchange.financial_markets.iter().map(|(instr_id, market)| {
            let best_bid = market.order_book.best_bid().map(|b| b.price);
            let best_ask = market.order_book.best_ask().map(|a| a.price);
            let spread = match (best_bid, best_ask) { (Some(b), Some(a)) => Some(a - b), _ => None };
            let mid    = match (best_bid, best_ask) { (Some(b), Some(a)) => Some((a + b) * 0.5), _ => None };
            let last   = market.current_price();

            // depth at best
            let bid_sz = if let Some(px) = best_bid {
                market.order_book.bids.iter().filter(|b| b.price == px).map(|b| b.quantity).sum()
            } else { 0.0 };
            let ask_sz = if let Some(px) = best_ask {
                market.order_book.asks.iter().filter(|a| a.price == px).map(|a| a.quantity).sum()
            } else { 0.0 };

            // Symbol/name fallbacks from market metadata if available
            let name   = market.name.clone();

            crate::dto::FinancialMarketSummaryDto {
                market_id: format!("Financial({})", instr_id),
                instrument_id: instr_id.to_string(),
                name,
                best_bid,
                best_ask,
                spread,
                mid,
                last: Some(last),
                depth: crate::dto::DepthDto {
                    bid_size_at_best: bid_sz,
                    ask_size_at_best: ask_sz,
                    bid_levels: market.order_book.bids.len(),
                    ask_levels: market.order_book.asks.len(),
                },
            }
        }).collect::<Vec<_>>();

        let page = crate::dto::FinancialMarketsPageDto { markets };
        (StatusCode::OK, headers, Json(serde_json::to_value(page).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(serde_json::to_value(json!({ "error": err })).unwrap()))
    }
}
pub async fn get_financial_orderbook(
    State(state): State<Arc<AppState>>,
    Path(instr_id_str): Path<String>, // note: binding, then type
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;

    if let Some(engine) = engine_guard.as_ref() {
        let fm_id = match FinancialMarketId::from_str(&instr_id_str) {
            Ok(id) => id,
            Err(_) => {
                let err = ApiError { code: "BAD_REQUEST", message: "Invalid financial market id" };
                return (StatusCode::BAD_REQUEST, headers, Json(json!({ "error": err })));
            }
        };

        if let Some(market) = engine.state.financial_system.exchange.financial_markets.get(&fm_id) {
            let bids = market.order_book.bids.iter().map(|b| crate::dto::MarketBidDto {
                agent_id: b.agent_id.to_string(),
                quantity: b.quantity,
                price: b.price,
            }).collect::<Vec<_>>();

            let asks = market.order_book.asks.iter().map(|a| crate::dto::MarketAskDto {
                agent_id: a.agent_id.to_string(),
                quantity: a.quantity,
                price: a.price,
            }).collect::<Vec<_>>();

            let dto = crate::dto::OrderbookDto {
                market_id: format!("Financial({fm_id})"),
                market_name: market.name.clone(),
                bids,
                asks,
            };
            (StatusCode::OK, headers, Json(serde_json::to_value(dto).unwrap()))
        } else {
            let err = ApiError { code: "NOT_FOUND", message: "Financial market not found." };
            (StatusCode::NOT_FOUND, headers, Json(json!({ "error": err })))
        }
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}
#[derive(Deserialize)]
pub struct FinHistoryQuery {
    pub limit: Option<usize>,
    pub bucket_secs: Option<i64>,
}
pub async fn get_financial_market_history(
    State(state): State<Arc<AppState>>,
    Path(instr_id_str): Path<String>,
    Query(q): Query<FinHistoryQuery>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;

    if let Some(engine) = engine_guard.as_ref() {
        let fm_id = match FinancialMarketId::from_str(&instr_id_str) {
            Ok(id) => id,
            Err(_) => {
                let err = ApiError { code: "BAD_REQUEST", message: "Invalid financial market id" };
                return (StatusCode::BAD_REQUEST, headers, Json(json!({ "error": err })));
            }
        };

        let exchange = &engine.state.financial_system.exchange;
        let tape = exchange.trade_tape
            .get(&sim_core::markets::MarketId::Financial(fm_id.clone()))
            .cloned()
            .unwrap_or_default();

        let limit = q.limit.unwrap_or(200);
        let trades = tape.iter().rev().take(limit).rev().map(|t| crate::dto::TradeDto {
            ts: t.at,
            price: t.trade.price,
            quantity: t.trade.quantity,
            buyer_id: t.trade.buyer.to_string(),
            seller_id: t.trade.seller.to_string(),
        }).collect::<Vec<_>>();

        let bucket = q.bucket_secs.unwrap_or(0);
        let candles = if bucket > 0 {
            use std::collections::BTreeMap;
            let mut buckets: BTreeMap<i64, Vec<&sim_core::markets::Trade>> = BTreeMap::new();
            for t in &tape {
                let k = (t.at / bucket) * bucket;
                buckets.entry(k).or_default().push(&t.trade);
            }
            buckets.into_iter().rev().take(limit).rev().map(|(ts, group)| {
                let open = group.first().map(|x| x.price).unwrap_or(0.0);
                let close = group.last().map(|x| x.price).unwrap_or(open);
                let high = group.iter().map(|x| x.price).fold(f64::MIN, f64::max);
                let low  = group.iter().map(|x| x.price).fold(f64::MAX, f64::min);
                let volume = group.iter().map(|x| x.quantity).sum::<f64>();
                crate::dto::CandleDto { ts, open, high, low, close, volume }
            }).collect::<Vec<_>>()
        } else { Vec::new() };
        let name = exchange.financial_markets.get(&fm_id)
            .and_then(|m| Some(m.name.clone()))
            .unwrap_or_else(|| fm_id.to_string());

        let dto = crate::dto::MarketHistoryDto {
            market_id: format!("Financial({fm_id})"),
            good_id: fm_id.to_string(), // keep field name, or rename in DTO if you prefer
            name,
            trades,
            candles,
        };

        (StatusCode::OK, headers, Json(serde_json::to_value(dto).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}
