use serde::Serialize;
use crate::*;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total_items: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Serialize, Clone)]
pub struct ActionDto {
    pub action_type: String,
    pub agent_id: String,
    pub agent_type: String,
    pub agent_name: Option<String>,
    pub details: serde_json::Value,
}

#[derive(Serialize, Clone)]
pub struct EffectDto {
    pub effect_type: String,
    pub details: serde_json::Value,
}

#[derive(Serialize, Clone)]
pub struct TickRecordDto {
    pub tick_number: u32,
    pub date: String,
    pub actions: Vec<ActionDto>,
    pub effects: Vec<EffectDto>,
    pub action_to_effect_indices: HashMap<usize, Vec<usize>>,
    pub trades: Vec<TradeDto>,
    pub summary: TickSummaryDto,
}

#[derive(Serialize, Clone)]
pub struct TickSummaryDto {
    pub total_actions: usize,
    pub total_effects: usize,
    pub total_trades: usize,
    pub actions_by_type: std::collections::HashMap<String, usize>,
    pub effects_by_type: std::collections::HashMap<String, usize>,
    pub agents_active: usize,
}

#[derive(Serialize, Clone)]
pub struct SimulationHistoryDto {
    pub ticks: Vec<TickRecordDto>,
    pub total_ticks: usize,
    pub page: u32,
    pub page_size: u32,
}