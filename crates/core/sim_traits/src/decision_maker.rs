//! Decision-making traits

use sim_types::*;

pub trait DecisionMaker {
    type Decision;
    
    fn decide(&self, state: &SimState) -> Vec<Self::Decision>;
}

