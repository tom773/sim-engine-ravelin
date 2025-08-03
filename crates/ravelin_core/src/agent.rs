use crate::*;
use rand::RngCore;

pub trait Agent {
    type DecisionType;

    fn decide(&self, fs: &FinancialSystem, rng: &mut dyn RngCore) -> Vec<Self::DecisionType>;
    fn act(&self, decisions: &[Self::DecisionType]) -> Vec<SimAction>;
}