use std::collections::HashMap;
use crate::*;
use sim_core::prelude::*;

impl From<&ActionRecord> for ActionDto {
    fn from(record: &ActionRecord) -> Self {
        ActionDto {
            action_type: record.action.name(),
            agent_id: record.agent_id.to_string(),
            agent_type: record.agent_type.clone(),
            agent_name: record.agent_name.clone(),
            details: serde_json::to_value(&record.action).unwrap_or(serde_json::Value::Null),
        }
    }
}

impl From<&StateEffect> for EffectDto {
    fn from(effect: &StateEffect) -> Self {
        EffectDto {
            effect_type: effect.name(),
            details: serde_json::to_value(effect).unwrap_or(serde_json::Value::Null),
        }
    }
}

impl From<&Trade> for TradeDto {
    fn from(trade: &Trade) -> Self {
        TradeDto {
            market_id: trade.market_id.to_string(),
            buyer_id: trade.buyer.to_string(),
            seller_id: trade.seller.to_string(),
            quantity: trade.quantity,
            price: trade.price,
        }
    }
}

impl From<&TickRecord> for TickRecordDto {
    fn from(record: &TickRecord) -> Self {
        let actions: Vec<ActionDto> = record.actions.iter().map(ActionDto::from).collect();
        let effects: Vec<EffectDto> = record.effects.iter().map(EffectDto::from).collect();
        let trades: Vec<TradeDto> = record.trades.iter().map(TradeDto::from).collect();
        let action_to_effect_idx: HashMap<usize, Vec<usize>> = record
            .action_to_effect_indices
            .iter()
            .map(|(action_idx, effect_indices)| (*action_idx, effect_indices.clone()))
            .collect();
        

        let mut actions_by_type = std::collections::HashMap::new();
        for action in &actions {
            *actions_by_type.entry(action.action_type.clone()).or_insert(0) += 1;
        }
        
        let mut effects_by_type = std::collections::HashMap::new();
        for effect in &effects {
            *effects_by_type.entry(effect.effect_type.clone()).or_insert(0) += 1;
        }
        let mut action_to_effect_indices = HashMap::new();
        for (action_idx, effect_indices) in &action_to_effect_idx {
            action_to_effect_indices.insert(*action_idx, effect_indices.clone());
        }
        
        let agents_active = actions.iter()
            .map(|a| &a.agent_id)
            .collect::<std::collections::HashSet<_>>()
            .len();
        
        let summary = TickSummaryDto {
            total_actions: actions.len(),
            total_effects: effects.len(),
            total_trades: trades.len(),
            actions_by_type,
            effects_by_type,
            agents_active,
        };
        
        TickRecordDto {
            tick_number: record.tick_number,
            date: record.date.format("%Y-%m-%d").to_string(),
            actions,
            effects,
            action_to_effect_indices,
            trades,
            summary,
        }
    }
}