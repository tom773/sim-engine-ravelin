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
use sim_core::traits::MarketSummaryProvider;

pub async fn get_markets_overview(State(state): State<Arc<AppState>>) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;

    if let Some(engine) = engine_guard.as_ref() {
        let fs = &engine.state.financial_system;

        let treasury_summaries = fs.exchange.get_treasury_market_summaries(fs);
        
        let treasuries: Vec<TreasuryMarketDto> = treasury_summaries.iter().map(|summary| {
            TreasuryMarketDto {
                instrument_id: format!("Financial(Treasury_{})", summary.tenor),
                name: format!("US {}", summary.tenor.to_string().replace("T", "").replace("Y", "Y")),
                price: summary.price / 10.0,
                yield_to_maturity: summary.yield_to_maturity * 100.0,
                spread_bps: summary.spread_bps,
                daily_change_pct: 0.001,
                duration: None,
                convexity: None,
            }
        }).collect();

        let yield_curve: Vec<YieldCurvePointDto> = treasuries
            .iter()
            .map(|treasury| YieldCurvePointDto {
                tenor: treasury.name.replace("US ", ""),
                yield_pct: treasury.yield_to_maturity,
                price: Some(treasury.price),
                change_bps: None,
            })
            .collect();

        let overnight_rates_data = fs.calculate_overnight_rates();
        let overnight_rates_dto = OvernightRatesDto {
            effr: overnight_rates_data.effr,
            sofr: overnight_rates_data.sofr,
            iorb: Some(overnight_rates_data.iorb),
            discount_rate: Some(overnight_rates_data.discount_rate),
            overnight_RRP: Some(overnight_rates_data.overnight_rrp),
        };

        let markets_dto = MarketsPageDto { 
            treasuries, 
            yield_curve, 
            overnight_rates: overnight_rates_dto,
            market_summary: None,
        };

        (StatusCode::OK, headers, Json(serde_json::to_value(markets_dto).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(serde_json::json!({ "error": err })))
    }
}

pub async fn get_goods_catalogue(
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


pub async fn get_market_goods_overview(
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
                let summary = market.summary();
                let depth = summary.depth;
                let g = goods.get_good_by_id(good_id);

                crate::dto::GoodsMarketSummaryDto {
                    market_id: good_id.to_string(),
                    good_id: good_id.to_string(),
                    name: g.map_or_else(|| format!("{good_id}"), |x| x.name.clone()),
                    unit: g.map_or_else(|| "".to_string(), |x| x.unit.clone()),
                    best_bid: depth.best_bid,
                    best_ask: depth.best_ask,
                    spread: summary.spread,
                    mid: summary.mid,
                    last: summary.last_price,
                    depth: crate::dto::DepthDto {
                        bid_size_at_best: depth.bid_size_at_best,
                        ask_size_at_best: depth.ask_size_at_best,
                        bid_levels: depth.bid_levels,
                        ask_levels: depth.ask_levels,
                    },
                    volume_24h: None,
                    price_change_24h: None,
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

pub async fn get_market_goods_orderbook(
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

pub async fn get_market_goods_history(
    State(state): State<Arc<AppState>>, Path(good_id_str): Path<String>, Query(q): Query<HistoryQuery>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;

    if let Some(engine) = engine_guard.as_ref() {
        let good_id: GoodId = good_id_str.parse().unwrap_or_else(|_| GoodId::from_slug(&good_id_str));
        let exchange = &engine.state.financial_system.exchange;
        let market_id = MarketId::Goods(good_id);
        
        let limit = q.limit.unwrap_or(200);
        let trades = exchange.trade_tape.get(&market_id).cloned().unwrap_or_default()
            .iter().rev().take(limit).rev()
            .map(|t| crate::dto::TradeDto {
                price: t.trade.price,
                quantity: t.trade.quantity,
                market_id: market_id.to_string(),
                buyer_id: t.trade.buyer.to_string(),
                seller_id: t.trade.seller.to_string(),
            }).collect();

        let bucket = q.bucket_secs.unwrap_or(0);
        let core_candles = exchange.calculate_candles(&market_id, bucket, limit);
        let candles = core_candles.into_iter().map(|c| CandleDto {
            ts: c.ts, open: c.open, high: c.high, low: c.low, close: c.close,
            volume: c.volume, vwap: c.vwap, trades_count: Some(c.trades_count),
        }).collect();

        let dto = crate::dto::MarketHistoryDto {
            market_id: format!("Goods({good_id})"),
            good_id: good_id.to_string(),
            name: exchange.goods_markets.get(&good_id).unwrap().name.to_string(),
            trades,
            candles,
        };
        (StatusCode::OK, headers, Json(serde_json::to_value(dto).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}

pub async fn get_market_financial_overview(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;

    if let Some(engine) = engine_guard.as_ref() {
        let exchange = &engine.state.financial_system.exchange;
        let fs = &engine.state.financial_system;

        let markets = exchange.financial_markets.iter().map(|(instr_id, market)| {
            let summary = market.summary();
            let depth = summary.depth;

            let yield_to_maturity = if let FinancialMarketId::Treasury { .. } = instr_id {
                market.calculate_ytm(fs).or(Some(market.default_yield())).map(|y| y * 100.0)
            } else { None };
            let bid_yield = if let FinancialMarketId::Treasury { .. } = instr_id {
                market.calculate_ytm_with_price(fs, depth.best_bid.unwrap_or(0.0)).or(Some(market.default_yield())).map(|y| y * 100.0)
            } else { None };
            let ask_yield = if let FinancialMarketId::Treasury { .. } = instr_id {
                market.calculate_ytm_with_price(fs, depth.best_ask.unwrap_or(0.0)).or(Some(market.default_yield())).map(|y| y * 100.0)
            } else { None };

            crate::dto::FinancialMarketSummaryDto {
                market_id: format!("Financial({})", instr_id),
                instrument_id: instr_id.to_string(),
                name: market.name.clone(),
                best_bid: depth.best_bid,
                best_bid_yield: bid_yield,
                best_ask: depth.best_ask,
                best_ask_yield: ask_yield,
                spread: summary.spread,
                mid: summary.mid,
                last: summary.last_price,
                depth: crate::dto::DepthDto {
                    bid_size_at_best: depth.bid_size_at_best,
                    ask_size_at_best: depth.ask_size_at_best,
                    bid_levels: depth.bid_levels,
                    ask_levels: depth.ask_levels,
                },
                volume_24h: None, price_change_24h: None, yield_to_maturity, duration: None,
            }
        }).collect::<Vec<_>>();

        let page = crate::dto::FinancialMarketsPageDto { markets };
        (StatusCode::OK, headers, Json(serde_json::to_value(page).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(serde_json::to_value(json!({ "error": err })).unwrap()))
    }
}

pub async fn get_market_financial_orderbook(
    State(state): State<Arc<AppState>>,
    Path(instr_id_str): Path<String>,
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

pub async fn get_market_financial_history(
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
        let market_id = MarketId::Financial(fm_id.clone());

        let limit = q.limit.unwrap_or(200);
        let trades = exchange.trade_tape.get(&market_id).cloned().unwrap_or_default()
            .iter().rev().take(limit).rev()
            .map(|t| crate::dto::TradeDto {
                market_id: market_id.to_string(), price: t.trade.price, quantity: t.trade.quantity,
                buyer_id: t.trade.buyer.to_string(), seller_id: t.trade.seller.to_string(),
            }).collect();
        
        let bucket = q.bucket_secs.unwrap_or(0);
        let core_candles = exchange.calculate_candles(&market_id, bucket, limit);
        let candles = core_candles.into_iter().map(|c| CandleDto {
            ts: c.ts, open: c.open, high: c.high, low: c.low, close: c.close,
            volume: c.volume, vwap: c.vwap, trades_count: Some(c.trades_count),
        }).collect();
        
        let name = exchange.financial_markets.get(&fm_id)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| fm_id.to_string());

        let dto = crate::dto::MarketHistoryDto {
            market_id: format!("Financial({fm_id})"),
            good_id: fm_id.to_string(), name, trades, candles,
        };

        (StatusCode::OK, headers, Json(serde_json::to_value(dto).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}