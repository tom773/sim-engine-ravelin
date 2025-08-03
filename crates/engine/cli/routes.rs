use axum::{extract::State, response::Json};
use ravelin_traits::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use crate::Res;

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct TickInfo<A, E> {
    pub actions: Vec<A>,
    pub effects: Vec<E>,
}

pub async fn tick<C: Core>(State(state): State<Arc<RwLock<C::State>>>) -> Json<TickInfo<C::Action, C::Effect>> {
    let mut state_guard = state.write().await;
    let (_ss, actions, effects) = engine::tick::<C>(&mut state_guard);
    Json(TickInfo {
        actions: actions.clone(),
        effects: effects.clone(),
    })
}

pub async fn inject<C: Core>(State(state): State<Arc<RwLock<C::State>>>) -> Json<Res<C::State>> {
    let mut state_guard = state.write().await;
    
    let action = C::inject_liquidity_action();
    let registry = state_guard.get_domain_registry().clone();
    let result = registry.execute(&action, &state_guard);

    if result.success {
        if let Err(e) = state_guard.apply_effects(&result.effects) {
             println!("Error applying inject effects: {}", e);
        }
    }
    
    Json(Res {
        stats: Some(state_guard.get_stats_json()),
        messages: None,
        state: state_guard.clone(),
    })
}