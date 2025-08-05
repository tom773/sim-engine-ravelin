use sim_types::*;
use serde::{Deserialize, Serialize};
use ndarray::Array1;
use dyn_clone::{clone_trait_object, DynClone};
use std::fmt::Debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsumerDecision {
    Spend { agent_id: AgentId, seller_id: AgentId, amount: f64, good_id: GoodId },
    Save { agent_id: AgentId, amount: f64 },
    Work { agent_id: AgentId, hours: f64 },
}

impl ConsumerDecision {
    pub fn name(&self) -> &'static str {
        match self {
            ConsumerDecision::Spend { .. } => "Spend",
            ConsumerDecision::Save { .. } => "Save",
            ConsumerDecision::Work { .. } => "Work",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            ConsumerDecision::Spend { agent_id, .. } => *agent_id,
            ConsumerDecision::Save { agent_id, .. } => *agent_id,
            ConsumerDecision::Work { agent_id, .. } => *agent_id,
        }
    }
}

pub trait FeatureSource {
    fn get_age(&self) -> u32;
    fn get_income(&self) -> f64;
    fn get_savings(&self) -> f64;
    fn get_debt(&self) -> f64;
    fn get_family_size(&self) -> u32 { 1 }
    fn get_has_children(&self) -> bool { false }
    fn get_education_level_numeric(&self) -> u32 { 2 }
    fn get_housing_status_numeric(&self) -> u32 { 0 }
    fn get_is_urban(&self) -> bool { true }
    fn get_region_numeric(&self) -> u32 { 1 }
}

pub trait SpendingPredictor: DynClone + Send + Sync {
    fn predict_spending(&self, features: &Array1<f64>) -> f64;
    fn get_feature_names(&self) -> &[String];
}

clone_trait_object!(SpendingPredictor);

impl Debug for dyn SpendingPredictor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SpendingPredictor")
    }
}