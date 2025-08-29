use serde::Serialize;
use sim_core::prelude::*;
#[derive(Serialize, Clone)]
pub struct AgentCounts {
    pub banks: usize,
    pub firms: usize,
    pub consumers: usize,
    pub total: usize,
}

#[derive(Serialize, Clone)]
pub struct ProductionMetrics {
    pub capacity_utilization: f64,
    pub industrial_production: f64,
    pub manufacturing_pmi: Option<f64>,
    pub services_pmi: Option<f64>,
    pub building_permits: Option<f64>,
    pub existing_home_sales: Option<f64>,
    pub new_home_sales: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct ConsumerMetrics {
    pub retail_sales: f64,
    pub retail_sales_ex_auto: Option<f64>,
    pub consumer_spending: f64,
    pub consumer_confidence: Option<f64>,
    pub personal_income: Option<f64>,
    pub personal_saving_rate: Option<f64>,
    pub consumer_credit: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct AgentSummaryDto {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub decision_model: String,
    pub balance_sheet: BalanceSheet,
}

#[derive(Serialize, Clone)]
pub struct BalanceSheetSummary {
    pub assets: f64,
    pub liabilities: f64,
    pub equity: f64,
}