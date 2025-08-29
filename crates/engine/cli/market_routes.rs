use crate::{AppState, routes::*};
use engine::dto::*;
use std::str::FromStr;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize};
use serde_json::json;
use sim_core::*;
use std::sync::Arc;

pub async fn get_markets_overview(State(state): State<Arc<AppState>>) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);

    let query_service = match get_qs(&state).await {
        Ok(qs) => qs,
        Err(response) => return response,
    };
    match query_service.get_market_summaries().await {
        Ok(overview) => {
            (StatusCode::OK, headers, Json(serde_json::to_value(overview).unwrap()))
        }
        Err(e) => {
            let err = json!({ "error": format!("Database error: {}", e) });
            (StatusCode::INTERNAL_SERVER_ERROR, headers, Json(err))
        }
    }
}

pub async fn get_goods_catalogue(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    
    let query_service = match get_qs(&state).await {
        Ok(qs) => qs,
        Err(response) => return response,
    };

    match query_service.get_goods_catalogue().await {
        Ok(goods_page) => {
            (StatusCode::OK, headers, Json(serde_json::to_value(goods_page).unwrap()))
        }
        Err(e) => {
            let err = json!({ "error": format!("Database error: {}", e) });
            (StatusCode::INTERNAL_SERVER_ERROR, headers, Json(err))
        }
    }
}


pub async fn get_market_goods_overview(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let err = ApiError { code: "NOT_IMPLEMENTED_CQRS", message: "Goods market overview query is pending migration to QueryService." };
    (StatusCode::NOT_IMPLEMENTED, headers, Json(json!({ "error": err })))
}

pub async fn get_market_goods_orderbook(
    State(state): State<Arc<AppState>>, Path(good_id_str): Path<String>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    
    let good_id: GoodId = good_id_str.parse().unwrap_or_else(|_| GoodId::from_slug(&good_id_str));
    let market_id_str = MarketId::Goods(good_id).to_string();
    
    let query_service = match get_qs(&state).await {
        Ok(qs) => qs,
        Err(response) => return response,
    };

    match query_service.get_order_book(&market_id_str).await {
        Ok(Some(record)) => {
            let bids_res: Result<Vec<Bid>, _> = record.bids.into_iter().map(serde_json::from_value).collect();
            let asks_res: Result<Vec<Ask>, _> = record.asks.into_iter().map(serde_json::from_value).collect();

            match (bids_res, asks_res) {
                (Ok(bids_orders), Ok(asks_orders)) => {
                    let bids_dto: Vec<MarketBidDto> = bids_orders.iter().map(|b| MarketBidDto {
                        agent_id: b.agent_id.to_string(),
                        quantity: b.quantity,
                        price: b.price,
                    }).collect();

                    let asks_dto: Vec<MarketAskDto> = asks_orders.iter().map(|a| MarketAskDto {
                        agent_id: a.agent_id.to_string(),
                        quantity: a.quantity,
                        price: a.price,
                    }).collect();

                    let dto = OrderbookDto {
                        market_id: market_id_str.clone(),
                        market_name: market_id_str, // TODO: Enhance QS to join with good name
                        bids: bids_dto,
                        asks: asks_dto,
                    };
                    (StatusCode::OK, headers, Json(serde_json::to_value(dto).unwrap()))
                }
                _ => {
                     let err = json!({ "error": "Failed to deserialize order book data from database." });
                    (StatusCode::INTERNAL_SERVER_ERROR, headers, Json(err))
                }
            }
        }
        Ok(None) => {
            let err = ApiError { code: "NOT_FOUND", message: "Order book not found for this market at the latest tick." };
            (StatusCode::NOT_FOUND, headers, Json(json!({ "error": err })))
        }
        Err(e) => {
            let err = json!({ "error": format!("Database error: {}", e) });
            (StatusCode::INTERNAL_SERVER_ERROR, headers, Json(err))
        }
    }
}


#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<usize>,
    pub bucket_secs: Option<i64>,
}

pub async fn get_market_goods_history(
    State(state): State<Arc<AppState>>, Path(_good_id_str): Path<String>, Query(_q): Query<HistoryQuery>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let err = ApiError { code: "NOT_IMPLEMENTED_CQRS", message: "Market history query is pending migration to QueryService." };
    (StatusCode::NOT_IMPLEMENTED, headers, Json(json!({ "error": err })))
}

pub async fn get_market_financial_overview(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let err = ApiError { code: "NOT_IMPLEMENTED_CQRS", message: "Financial market overview query is pending migration to QueryService." };
    (StatusCode::NOT_IMPLEMENTED, headers, Json(json!({ "error": err })))
}

pub async fn get_market_financial_orderbook(
    State(state): State<Arc<AppState>>,
    Path(instr_id_str): Path<String>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    
    let fm_id = match FinancialMarketId::from_str(&instr_id_str) {
        Ok(id) => id,
        Err(_) => {
            let err = ApiError { code: "BAD_REQUEST", message: "Invalid financial market id" };
            return (StatusCode::BAD_REQUEST, headers, Json(json!({ "error": err })));
        }
    };
    let market_id_str = MarketId::Financial(fm_id).to_string();

    let query_service = match get_qs(&state).await {
        Ok(qs) => qs,
        Err(response) => return response,
    };
    
    match query_service.get_order_book(&market_id_str).await {
        Ok(Some(record)) => {
            let bids_res: Result<Vec<Bid>, _> = record.bids.into_iter().map(serde_json::from_value).collect();
            let asks_res: Result<Vec<Ask>, _> = record.asks.into_iter().map(serde_json::from_value).collect();

            match (bids_res, asks_res) {
                (Ok(bids_orders), Ok(asks_orders)) => {
                    let bids_dto: Vec<MarketBidDto> = bids_orders.iter().map(|b| MarketBidDto {
                        agent_id: b.agent_id.to_string(),
                        quantity: b.quantity,
                        price: b.price,
                    }).collect();

                    let asks_dto: Vec<MarketAskDto> = asks_orders.iter().map(|a| MarketAskDto {
                        agent_id: a.agent_id.to_string(),
                        quantity: a.quantity,
                        price: a.price,
                    }).collect();

                    let dto = OrderbookDto {
                        market_id: market_id_str.clone(),
                        market_name: market_id_str,
                        bids: bids_dto,
                        asks: asks_dto,
                    };
                    (StatusCode::OK, headers, Json(serde_json::to_value(dto).unwrap()))
                }
                _ => {
                     let err = json!({ "error": "Failed to deserialize order book data from database." });
                    (StatusCode::INTERNAL_SERVER_ERROR, headers, Json(err))
                }
            }
        }
        Ok(None) => {
            let err = ApiError { code: "NOT_FOUND", message: "Order book not found for this market at the latest tick." };
            (StatusCode::NOT_FOUND, headers, Json(json!({ "error": err })))
        }
        Err(e) => {
            let err = json!({ "error": format!("Database error: {}", e) });
            (StatusCode::INTERNAL_SERVER_ERROR, headers, Json(err))
        }
    }
}


#[derive(Deserialize)]
pub struct FinHistoryQuery {
    pub limit: Option<usize>,
    pub bucket_secs: Option<i64>,
}

pub async fn get_market_financial_history(
    State(state): State<Arc<AppState>>,
    Path(_instr_id_str): Path<String>,
    Query(_q): Query<FinHistoryQuery>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let err = ApiError { code: "NOT_IMPLEMENTED_CQRS", message: "Market history query is pending migration to QueryService." };
    (StatusCode::NOT_IMPLEMENTED, headers, Json(json!({ "error": err })))
}

pub async fn get_market_labour_overview(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let err = ApiError { code: "NOT_IMPLEMENTED_CQRS", message: "Labour market overview query is pending migration to QueryService." };
    (StatusCode::NOT_IMPLEMENTED, headers, Json(json!({ "error": err })))
}