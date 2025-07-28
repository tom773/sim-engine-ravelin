use crate::*;
use rand::rngs::StdRng;

pub trait Agent {
    type DecisionType;

    fn decide(&self, fs: &FinancialSystem, rng: &mut StdRng) -> Vec<Self::DecisionType>;
    fn act(&self, decisions: &[Self::DecisionType]) -> Vec<SimAction>;
}
