use crate::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobOffer {
    pub offer_id: Uuid,
    pub firm_id: AgentId,
    pub wage_rate: f64,
    pub value_per_hour: f64,
    pub hours_required: f64,
    pub quantity: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobApplication {
    pub application_id: Uuid,
    pub consumer_id: AgentId,
    pub reservation_wage: f64,
    pub hours_desired: f64,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WageRule {
    Posted,
    Nash { beta: f64 },
}
impl Default for WageRule {
    fn default() -> Self {
        WageRule::Nash { beta: 0.5 }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LabourMarket {
    pub job_offers: Vec<JobOffer>,
    pub job_applications: Vec<JobApplication>,
    pub wage_rule: WageRule,
}