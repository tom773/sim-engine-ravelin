use axum::{
    extract::{State},
    response::Json,
};
use engine::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use crate::{Stats, Res};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TickInfo {
    pub ticknum: u32,
    pub actions: Vec<SimAction>,
    pub effects: Vec<StateEffect>,
}

pub async fn tick(State(state): State<Arc<RwLock<SimState>>>) -> Json<TickInfo> {
    let mut state_guard = state.write().await;
    let (_ss, actions, effects) = engine::tick(&mut state_guard);
    let ti = TickInfo {
        ticknum: state_guard.ticknum,
        actions: actions.clone(),
        effects: effects.clone(),
    };
    Json(ti)
}
pub async fn inject(State(state): State<Arc<RwLock<SimState>>>) -> Json<Res> {
    let mut state_guard = state.write().await;
    let _ss = engine::inject_liquidity(&mut state_guard);
    
    Json(Res {
        stats: Some(Stats {
            m0: state_guard.financial_system.m0(),
            m1: state_guard.financial_system.m1(),
            m2: state_guard.financial_system.m2(),
        }),
        messages: None,
        state: state_guard.clone(),
    })
}
pub async fn tick_ret_date(State(state): State<Arc<RwLock<SimState>>>) -> Json<HashMap<InstrumentId, FinancialInstrument>> {
    let mut state_guard = state.write().await;
    let (ss, _actions, _effects) = engine::tick(&mut state_guard);
    let instruments = ss.financial_system.instruments.clone();
    Json(instruments)
}