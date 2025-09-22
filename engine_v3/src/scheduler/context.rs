use super::{StepResult, TickStep};
use domains::ResolutionPhase;
use sim_core::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct StepContext {
    pub tick_number: u32,
    pub step_data: HashMap<TickStep, StepResult>,
    categorized_intentions: Option<HashMap<ResolutionPhase, Vec<SimIntention>>>,
    intentions: Option<Vec<SimIntention>>,
    all_actions: Vec<ActionRecord>,
    all_effects: Vec<StateEffect>,
    action_to_effect_indices: HashMap<usize, Vec<usize>>,
    trades: Vec<Trade>,
    market_snapshots: HashMap<Symbol, MarketView>,
}

impl StepContext {
    pub fn new(tick_number: u32) -> Self {
        Self {
            tick_number,
            step_data: HashMap::new(),
            categorized_intentions: None,
            intentions: None,
            all_actions: Vec::new(),
            all_effects: Vec::new(),
            action_to_effect_indices: HashMap::new(),
            trades: Vec::new(),
            market_snapshots: HashMap::new(),
        }
    }

    pub fn set_categorized_intentions(&mut self, categorized: HashMap<ResolutionPhase, Vec<SimIntention>>) {
        self.categorized_intentions = Some(categorized);
    }

    pub fn categorized_intentions(&self) -> Option<&HashMap<ResolutionPhase, Vec<SimIntention>>> {
        self.categorized_intentions.as_ref()
    }

    pub fn get_categorized_intentions(&self) -> Result<HashMap<ResolutionPhase, Vec<SimIntention>>, String> {
        self.categorized_intentions.clone().ok_or_else(|| "No categorized intentions stored".to_string())
    }

    pub fn set_intentions(&mut self, intentions: Vec<SimIntention>) {
        self.intentions = Some(intentions);
    }

    pub fn intentions(&self) -> Option<&[SimIntention]> {
        self.intentions.as_deref()
    }

    pub fn get_intentions(&self) -> Result<Vec<SimIntention>, String> {
        self.intentions.clone().ok_or_else(|| "No intentions stored".to_string())
    }

    pub fn set_all_actions(&mut self, actions: Vec<ActionRecord>) {
        self.all_actions = actions;
    }

    pub fn get_all_actions(&self) -> Result<Vec<ActionRecord>, String> {
        Ok(self.all_actions.clone())
    }

    pub fn set_all_effects(&mut self, effects: Vec<StateEffect>) {
        self.all_effects = effects;
    }

    pub fn get_all_effects(&self) -> Result<Vec<StateEffect>, String> {
        Ok(self.all_effects.clone())
    }

    pub fn set_action_to_effect_indices(&mut self, mapping: HashMap<usize, Vec<usize>>) {
        self.action_to_effect_indices = mapping;
    }

    pub fn get_action_to_effect_indices(&self) -> Result<HashMap<usize, Vec<usize>>, String> {
        Ok(self.action_to_effect_indices.clone())
    }

    pub fn set_trades(&mut self, trades: Vec<Trade>) {
        self.trades = trades;
    }

    pub fn get_trades(&self) -> Result<Vec<Trade>, String> {
        Ok(self.trades.clone())
    }

    pub fn set_market_snapshots(&mut self, snapshots: HashMap<Symbol, MarketView>) {
        self.market_snapshots = snapshots;
    }

    pub fn get_market_snapshots(&self) -> Result<HashMap<Symbol, MarketView>, String> {
        Ok(self.market_snapshots.clone())
    }
}
