use crate::{AppState, dto::*, routes::*};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde_json::json;
use sim_core::*;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct TickHistoryQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub tick_from: Option<u32>,
    pub tick_to: Option<u32>,
}

#[derive(Deserialize)]
pub struct ActionQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub agent_type: Option<String>,
    pub action_type: Option<String>,
    pub agent_id: Option<String>,
    pub tick_from: Option<u32>,
    pub tick_to: Option<u32>,
}

#[derive(Deserialize)]
pub struct EffectQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub agent_type: Option<String>,
    pub effect_type: Option<String>,
    pub agent_id: Option<String>,
    pub tick_from: Option<u32>,
    pub tick_to: Option<u32>,
}

pub async fn get_simulation_history(
    Query(query): Query<TickHistoryQuery>, 
    State(state): State<Arc<AppState>>
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;
    
    if let Some(engine) = engine_guard.as_ref() {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).min(100).max(1);
        
        let all_ticks: Vec<&TickRecord> = engine.state.history.tick_records
            .iter()
            .filter(|tick| {
                let tick_in_range = match (query.tick_from, query.tick_to) {
                    (Some(from), Some(to)) => tick.tick_number >= from && tick.tick_number <= to,
                    (Some(from), None) => tick.tick_number >= from,
                    (None, Some(to)) => tick.tick_number <= to,
                    (None, None) => true,
                };
                tick_in_range
            })
            .collect();
        
        let total_ticks = all_ticks.len();
        let skip = ((page - 1) * page_size) as usize;
        
        let ticks: Vec<TickRecordDto> = all_ticks
            .into_iter()
            .rev() // Most recent first
            .skip(skip)
            .take(page_size as usize)
            .map(TickRecordDto::from)
            .collect();
        
        let history_dto = SimulationHistoryDto {
            ticks,
            total_ticks,
            page,
            page_size,
        };
        
        (StatusCode::OK, headers, Json(serde_json::to_value(history_dto).unwrap()))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}

pub async fn get_actions_history(
    Query(query): Query<ActionQuery>, 
    State(state): State<Arc<AppState>>
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;
    
    if let Some(engine) = engine_guard.as_ref() {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(50).min(500).max(1);
        
        let mut all_actions: Vec<(u32, &ActionRecord)> = Vec::new();
        
        for tick_record in &engine.state.history.tick_records {
            let tick_in_range = match (query.tick_from, query.tick_to) {
                (Some(from), Some(to)) => tick_record.tick_number >= from && tick_record.tick_number <= to,
                (Some(from), None) => tick_record.tick_number >= from,
                (None, Some(to)) => tick_record.tick_number <= to,
                (None, None) => true,
            };
            
            if tick_in_range {
                for action in &tick_record.actions {
                    let matches_filter = 
                        query.agent_type.as_ref().map_or(true, |t| &action.agent_type == t) &&
                        query.action_type.as_ref().map_or(true, |t| action.action.name().contains(t)) &&
                        query.agent_id.as_ref().map_or(true, |id| &action.agent_id.to_string() == id);
                    
                    if matches_filter {
                        all_actions.push((tick_record.tick_number, action));
                    }
                }
            }
        }
        
        let total_actions = all_actions.len();
        let skip = ((page - 1) * page_size) as usize;
        
        #[derive(Serialize)]
        struct ActionWithTick {
            tick_number: u32,
            #[serde(flatten)]
            action: ActionDto,
        }
        
        let actions: Vec<ActionWithTick> = all_actions
            .into_iter()
            .rev() // Most recent first
            .skip(skip)
            .take(page_size as usize)
            .map(|(tick, action)| ActionWithTick {
                tick_number: tick,
                action: ActionDto::from(action),
            })
            .collect();
        
        let response = json!({
            "actions": actions,
            "total_actions": total_actions,
            "page": page,
            "page_size": page_size,
        });
        
        (StatusCode::OK, headers, Json(response))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}

pub async fn get_tick_details(
    Path(tick_number): Path<u32>,
    State(state): State<Arc<AppState>>
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;
    
    if let Some(engine) = engine_guard.as_ref() {
        if let Some(tick_record) = engine.state.history.tick_records
            .iter()
            .find(|t| t.tick_number == tick_number) {
            
            let tick_dto = TickRecordDto::from(tick_record);
            (StatusCode::OK, headers, Json(serde_json::to_value(tick_dto).unwrap()))
        } else {
            let err = ApiError { code: "NOT_FOUND", message: "Tick not found." };
            (StatusCode::NOT_FOUND, headers, Json(json!({ "error": err })))
        }
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}

pub async fn get_effects_history(
    Query(query): Query<EffectQuery>, 
    State(state): State<Arc<AppState>>
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;
    
    if let Some(engine) = engine_guard.as_ref() {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(50).min(500).max(1);
        
        let mut all_effects: Vec<(u32, &StateEffect)> = Vec::new();
        
        for tick_record in &engine.state.history.tick_records {
            let tick_in_range = match (query.tick_from, query.tick_to) {
                (Some(from), Some(to)) => tick_record.tick_number >= from && tick_record.tick_number <= to,
                (Some(from), None) => tick_record.tick_number >= from,
                (None, Some(to)) => tick_record.tick_number <= to,
                (None, None) => true,
            };
            
            if tick_in_range {
                for effect in &tick_record.effects {
                    let matches_filter = 
                        query.effect_type.as_ref().map_or(true, |t| effect.name().contains(t)); 
                    
                    if matches_filter {
                        all_effects.push((tick_record.tick_number, effect));
                    }
                }
            }
        }
        
        let total_effects = all_effects.len();
        let skip = ((page - 1) * page_size) as usize;
        
        #[derive(Serialize)]
        struct EffectWithTick {
            tick_number: u32,
            #[serde(flatten)]
            effect: EffectDto,
        }
        
        let effects: Vec<EffectWithTick> = all_effects
            .into_iter()
            .rev() // Most recent first
            .skip(skip)
            .take(page_size as usize)
            .map(|(tick, effect)| EffectWithTick {
                tick_number: tick,
                effect: EffectDto::from(effect),
            })
            .collect();
        
        let response = json!({
            "effects": effects,
            "total_effects": total_effects,
            "page": page,
            "page_size": page_size,
        });
        (StatusCode::OK, headers, Json(response))
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is   not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err }
        )))
    }
}

pub async fn get_actions_to_effects(
    Path(tick_number): Path<u32>,
    State(state): State<Arc<AppState>>
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let headers = with_epoch_header(HeaderMap::new(), state.epoch);
    let engine_guard = state.sim_engine.read().await;
    if let Some(engine) = engine_guard.as_ref() {
        println!("[DEBUG] TICKRECORDS: {:#?}\n", engine.state.history.tick_records);
        if let Some(tick_record) = engine.state.history.tick_records
            .iter()
            .find(|t| t.tick_number == tick_number) {
            
            let action_to_effect_indices: HashMap<usize, Vec<usize>> = tick_record
                .action_to_effect_indices
                .iter()
                .map(|(action_idx, effect_indices)| (*action_idx, effect_indices.clone()))
                .collect();
            
            (StatusCode::OK, headers, Json(json!({ "a2e_indices": action_to_effect_indices })))
        } else {
            let err = ApiError { code: "NOT_FOUND", message: "Tick not found." };
            (StatusCode::NOT_FOUND, headers, Json(json!({ "error": err })))
        }
    } else {
        let err = ApiError { code: "NOT_INITIALIZED", message: "Simulation is not initialized." };
        (StatusCode::CONFLICT, headers, Json(json!({ "error": err })))
    }
}