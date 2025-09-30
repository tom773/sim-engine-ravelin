use super::{StepResult, TickStep};
use ahash::AHashMap;
use domains::ResolutionPhase;
use sim_core::*;
use smallvec::SmallVec;
use std::collections::HashMap as StdHashMap;
use std::mem;

#[derive(Debug)]
pub struct StepContext {
    pub tick_number: u32,
    pub step_data: AHashMap<TickStep, StepResult>,
    categorized_intentions: Option<AHashMap<ResolutionPhase, Vec<SimIntention>>>,
    intentions: Option<SmallVec<[SimIntention; 8]>>,
    all_actions: SmallVec<[ActionRecord; 16]>,
    all_effects: SmallVec<[StateEffect; 32]>,
    action_to_effect_indices: AHashMap<usize, Vec<usize>>,
    trades: SmallVec<[Trade; 16]>,
    market_snapshots: StdHashMap<Symbol, MarketView>,
}

impl StepContext {
    pub fn new(tick_number: u32) -> Self {
        Self {
            tick_number,
            step_data: AHashMap::new(),
            categorized_intentions: None,
            intentions: None,
            all_actions: SmallVec::new(),
            all_effects: SmallVec::new(),
            action_to_effect_indices: AHashMap::new(),
            trades: SmallVec::new(),
            market_snapshots: StdHashMap::new(),
        }
    }

    pub fn set_categorized_intentions(&mut self, categorized: AHashMap<ResolutionPhase, Vec<SimIntention>>) {
        self.categorized_intentions = Some(categorized);
    }

    pub fn categorized_intentions(&self) -> Option<&AHashMap<ResolutionPhase, Vec<SimIntention>>> {
        self.categorized_intentions.as_ref()
    }

    pub fn get_categorized_intentions(&self) -> Result<AHashMap<ResolutionPhase, Vec<SimIntention>>, String> {
        self.categorized_intentions.clone().ok_or_else(|| "No categorized intentions stored".to_string())
    }

    pub fn set_intentions(&mut self, intentions: Vec<SimIntention>) {
        self.intentions = Some(SmallVec::from_vec(intentions));
    }

    pub fn intentions(&self) -> Option<&[SimIntention]> {
        self.intentions.as_ref().map(|s| s.as_slice())
    }

    pub fn intentions_mut(&mut self) -> &mut SmallVec<[SimIntention; 8]> {
        self.intentions.get_or_insert_with(SmallVec::new)
    }

    pub fn intentions_slice(&self) -> &[SimIntention] {
        self.intentions.as_ref().map(|s| s.as_slice()).unwrap_or(&[])
    }

    pub fn take_intentions(&mut self) -> Vec<SimIntention> {
        self.intentions.take().map(SmallVec::into_vec).unwrap_or_default()
    }

    pub fn get_intentions(&self) -> Result<Vec<SimIntention>, String> {
        self.intentions.as_ref().map(|s| s.clone().into_vec()).ok_or_else(|| "No intentions stored".to_string())
    }

    pub fn set_all_actions(&mut self, actions: Vec<ActionRecord>) {
        self.all_actions = SmallVec::from_vec(actions);
    }

    pub fn get_all_actions(&self) -> Result<Vec<ActionRecord>, String> {
        Ok(self.all_actions.clone().into_vec())
    }

    pub fn actions(&self) -> &[ActionRecord] {
        self.all_actions.as_slice()
    }

    pub fn actions_mut(&mut self) -> &mut SmallVec<[ActionRecord; 16]> {
        &mut self.all_actions
    }

    pub fn actions_len(&self) -> usize {
        self.all_actions.len()
    }

    pub fn take_actions(&mut self) -> Vec<ActionRecord> {
        mem::take(&mut self.all_actions).into_vec()
    }

    pub fn set_all_effects(&mut self, effects: Vec<StateEffect>) {
        self.all_effects = SmallVec::from_vec(effects);
    }

    pub fn get_all_effects(&self) -> Result<Vec<StateEffect>, String> {
        Ok(self.all_effects.clone().into_vec())
    }

    pub fn effects(&self) -> &[StateEffect] {
        self.all_effects.as_slice()
    }

    pub fn effects_mut(&mut self) -> &mut SmallVec<[StateEffect; 32]> {
        &mut self.all_effects
    }

    pub fn effects_len(&self) -> usize {
        self.all_effects.len()
    }

    pub fn take_effects(&mut self) -> Vec<StateEffect> {
        mem::take(&mut self.all_effects).into_vec()
    }

    pub fn set_action_to_effect_indices(&mut self, mapping: AHashMap<usize, Vec<usize>>) {
        self.action_to_effect_indices = mapping;
    }

    pub fn get_action_to_effect_indices(&self) -> Result<AHashMap<usize, Vec<usize>>, String> {
        Ok(self.action_to_effect_indices.clone())
    }

    pub fn action_to_effect_indices_ref(&self) -> &AHashMap<usize, Vec<usize>> {
        &self.action_to_effect_indices
    }

    pub fn action_to_effect_indices_mut(&mut self) -> &mut AHashMap<usize, Vec<usize>> {
        &mut self.action_to_effect_indices
    }

    pub fn take_action_to_effect_indices(&mut self) -> AHashMap<usize, Vec<usize>> {
        mem::take(&mut self.action_to_effect_indices)
    }

    pub fn set_trades(&mut self, trades: Vec<Trade>) {
        self.trades = SmallVec::from_vec(trades);
    }

    pub fn get_trades(&self) -> Result<Vec<Trade>, String> {
        Ok(self.trades.clone().into_vec())
    }

    pub fn trades(&self) -> &[Trade] {
        &self.trades
    }

    pub fn trades_mut(&mut self) -> &mut SmallVec<[Trade; 16]> {
        &mut self.trades
    }

    pub fn take_trades(&mut self) -> Vec<Trade> {
        mem::take(&mut self.trades).into_vec()
    }

    pub fn set_market_snapshots(&mut self, snapshots: StdHashMap<Symbol, MarketView>) {
        self.market_snapshots = snapshots;
    }

    pub fn get_market_snapshots(&self) -> Result<StdHashMap<Symbol, MarketView>, String> {
        Ok(self.market_snapshots.clone())
    }

    pub fn market_snapshots_ref(&self) -> &StdHashMap<Symbol, MarketView> {
        &self.market_snapshots
    }

    pub fn market_snapshots_mut(&mut self) -> &mut StdHashMap<Symbol, MarketView> {
        &mut self.market_snapshots
    }
}
