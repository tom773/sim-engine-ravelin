use serde::{Deserialize, Serialize};
use crate::dto::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryServiceDashboardData {
    pub tick_number: u32,
    pub current_date: String,
    pub agent_counts: QueryServiceAgentCounts,
    pub economic_stats: QueryServiceEconomicStats,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryServiceAgentCounts {
    pub banks: usize,
    pub firms: usize, 
    pub consumers: usize,
    pub total: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryServiceOvernightRates {
    pub effr: Option<f64>,
    pub sofr: Option<f64>,
    pub iorb: Option<f64>,
    pub discount_rate: Option<f64>,
    pub overnight_rrp: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryServiceEconomicStats {
    pub nominal_gdp_proxy: f64,
    pub consumer_spending_daily: f64,
    pub cpi: f64,
    pub ppi: f64,
    pub inflation_rate: f64,
    pub unemployment_rate: f64,
    pub labor_force_participation: f64,
    pub job_openings: f64,
    pub household_debt: f64,
    pub corporate_debt: f64,
    pub government_debt: f64,
    pub bank_reserves: f64,
    pub bank_credit: f64,
    pub bank_liabilities: f64,
    pub overnight_rates: QueryServiceOvernightRates,
    pub m0: f64,
    pub m1: f64,
    pub m2: f64,
}

pub fn map_query_data_to_dashboard_dto(
    query_data: QueryServiceDashboardData,
    total_iterations: u64,
) -> DashboardDto {
    let velocity_m1 = if query_data.economic_stats.m1 > 0.0 {
        Some(query_data.economic_stats.nominal_gdp_proxy / query_data.economic_stats.m1)
    } else {
        None
    };
    
    let velocity_m2 = if query_data.economic_stats.m2 > 0.0 {
        Some(query_data.economic_stats.nominal_gdp_proxy / query_data.economic_stats.m2)
    } else {
        None
    };

    let core_stats = CoreStats {
        gdp: query_data.economic_stats.nominal_gdp_proxy,
        cpi: query_data.economic_stats.cpi,
        ppi: query_data.economic_stats.ppi,
        unemployment_rate: query_data.economic_stats.unemployment_rate,
        labor_force_participation: query_data.economic_stats.labor_force_participation,
        job_openings: query_data.economic_stats.job_openings,
        capacity_utilization: 0.75,
        industrial_production: query_data.economic_stats.nominal_gdp_proxy * 0.2,
        retail_sales: query_data.economic_stats.consumer_spending_daily * 0.6,
        consumer_spending: query_data.economic_stats.consumer_spending_daily,
        credit_growth: 0.0,
        household_debt: query_data.economic_stats.household_debt,
        corporate_debt: query_data.economic_stats.corporate_debt,
        government_debt: query_data.economic_stats.government_debt,
        bank_liabilities: 0.0,
    };

    let monetary_stats = MonetaryStats {
        velocity_m1,
        velocity_m2,
        m0: query_data.economic_stats.m0,
        monetary_base: query_data.economic_stats.m0,
        m1: query_data.economic_stats.m1,
        m2: query_data.economic_stats.m2,
        bank_reserves: query_data.economic_stats.bank_reserves,
        bank_credit: query_data.economic_stats.bank_credit,
        bank_liabilities: query_data.economic_stats.bank_liabilities,
        currency_in_circulation: query_data.economic_stats.m0 * 0.8,
    };

    let policy_rates = PolicyRates {
        policy_rate: 500.0,
        reserve_requirement: 0.1,
    };

    // ** FIX: Map from the correct nested location **
    let overnight_rates = OvernightRatesDto {
        effr: query_data.economic_stats.overnight_rates.effr,
        sofr: query_data.economic_stats.overnight_rates.sofr,
        iorb: query_data.economic_stats.overnight_rates.iorb,
        discount_rate: query_data.economic_stats.overnight_rates.discount_rate,
        overnight_RRP: query_data.economic_stats.overnight_rates.overnight_rrp,
    };

    let agent_counts = AgentCounts {
        banks: query_data.agent_counts.banks,
        firms: query_data.agent_counts.firms,
        consumers: query_data.agent_counts.consumers,
        total: query_data.agent_counts.total,
    };

    let economic_stats = EconomicStats {
        core_stats,
        monetary_policy: policy_rates,
        monetary_stats,
        overnight_rates,
    };

    DashboardDto {
        current_date: query_data.current_date,
        tick_number: query_data.tick_number as u64,
        total_iterations,
        agent_counts,
        economic_stats,
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryServiceStatsData {
    pub tick_number: u32,
    pub economic_stats: QueryServiceEconomicStats,
}

pub fn map_query_stats_to_macro_stats(stats_data: QueryServiceStatsData) -> serde_json::Value {
    serde_json::json!({
        "nominal_gdp_proxy": stats_data.economic_stats.nominal_gdp_proxy,
        "consumer_spending_daily": stats_data.economic_stats.consumer_spending_daily,
        "cpi": stats_data.economic_stats.cpi,
        "ppi": stats_data.economic_stats.ppi,
        "inflation_rate": stats_data.economic_stats.inflation_rate,
        "unemployment_rate": stats_data.economic_stats.unemployment_rate,
        "labor_force_participation": stats_data.economic_stats.labor_force_participation,
        "job_openings": stats_data.economic_stats.job_openings,
        "household_debt": stats_data.economic_stats.household_debt,
        "corporate_debt": stats_data.economic_stats.corporate_debt,
        "government_debt": stats_data.economic_stats.government_debt,
        "overnight_rates": serde_json::to_value(&stats_data.economic_stats.overnight_rates).unwrap_or(serde_json::Value::Null),
        "bank_reserves": stats_data.economic_stats.bank_reserves,
        "bank_credit": stats_data.economic_stats.bank_credit,
        "bank_liabilities": stats_data.economic_stats.bank_liabilities,
        "m0": stats_data.economic_stats.m0,
        "m1": stats_data.economic_stats.m1,
        "m2": stats_data.economic_stats.m2,
        "velocity": if stats_data.economic_stats.m1 > 0.0 {
            stats_data.economic_stats.nominal_gdp_proxy / stats_data.economic_stats.m1
        } else {
            0.0
        },
        "cpi_inflation_rate": stats_data.economic_stats.inflation_rate,
        "employment": 0,
        "labour_force": 0,
    })
}