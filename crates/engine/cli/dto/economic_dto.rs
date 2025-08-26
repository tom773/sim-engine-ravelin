use serde::Serialize;
use crate::*;

#[derive(Serialize, Clone)]
pub struct MonetaryStats {
    pub m0: f64,
    pub m1: f64,
    pub m2: f64,
    pub velocity_m1: Option<f64>,
    pub velocity_m2: Option<f64>,
    pub monetary_base: f64,
    pub currency_in_circulation: f64,
    pub bank_reserves: f64,
}

#[derive(Serialize, Clone)]
pub struct PolicyRates {
    pub policy_rate: f64,
    pub reserve_requirement: f64,
}

#[derive(Serialize, Clone)]
pub struct InflationMetrics {
    pub cpi: f64,
    pub cpi_yoy: Option<f64>,
    pub cpi_mom: Option<f64>,
    pub core_cpi: Option<f64>,
    pub ppi: f64,
    pub ppi_yoy: Option<f64>,
    pub pce: Option<f64>,
    pub breakeven_5y: Option<f64>,
    pub breakeven_10y: Option<f64>,
}




#[derive(Serialize, Clone)]
pub struct DebtMetrics {
    pub credit_growth: f64,
    pub household_debt: f64,
    pub household_debt_to_income: Option<f64>,
    pub corporate_debt: f64,
    pub corporate_debt_to_gdp: Option<f64>,
    pub government_debt: f64,
    pub debt_to_gdp: Option<f64>,
    pub financial_sector_debt: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct FinancialStabilityMetrics {
    pub vix: Option<f64>,
    pub term_spread_10y2y: Option<f64>,
    pub credit_spreads_high_yield: Option<f64>,
    pub credit_spreads_investment_grade: Option<f64>,
    pub bank_lending_standards: Option<f64>,
    pub margin_debt: Option<f64>,
    pub leverage_ratio_avg: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct EconomicStats {

    pub core_stats: CoreStats,
    pub monetary_policy: PolicyRates,
    pub monetary_stats: MonetaryStats,
    pub overnight_rates: OvernightRatesDto,
}

#[derive(Serialize, Clone)]
pub struct CoreStats {

    pub gdp: f64,

    pub cpi: f64,
    pub ppi: f64,

    pub unemployment_rate: f64,
    pub labor_force_participation: f64,
    pub job_openings: f64,

    pub capacity_utilization: f64,
    pub industrial_production: f64,

    pub retail_sales: f64,
    pub consumer_spending: f64,

    pub trade_balance: f64,

    pub credit_growth: f64,
    pub household_debt: f64,
    pub corporate_debt: f64,
    pub government_debt: f64,
    pub bank_liabilities: f64,
}


#[derive(Serialize, Clone)]
pub struct SectorPerformance {
    pub sector_name: String,
    pub performance_1d: Option<f64>,
    pub performance_1w: Option<f64>,
    pub performance_1m: Option<f64>,
    pub performance_ytd: Option<f64>,
    pub pe_ratio: Option<f64>,
    pub dividend_yield: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct DetailedDashboardDto {
    pub current_date: String,
    pub tick_number: u64,
    pub total_iterations: u64,
    pub agent_counts: AgentCounts,
    pub economic_stats: EconomicStats,
    pub market_sentiment: Option<MarketSentimentMetrics>,
    pub sector_performance: Option<Vec<SectorPerformance>>,
    pub simulation_health: SimulationHealthMetrics,
}

#[derive(Serialize, Clone)]
pub struct SimulationHealthMetrics {
    pub total_agents_active: usize,
    pub markets_operational: usize,
    pub total_transactions_last_tick: Option<u64>,
    pub avg_transaction_value: Option<f64>,
    pub system_liquidity: Option<f64>,
    pub price_stability_index: Option<f64>,
}


#[derive(Serialize, Clone)]
pub struct DashboardDto {
    pub current_date: String,
    pub tick_number: u64,
    pub total_iterations: u64,
    pub agent_counts: AgentCounts,
    pub economic_stats: EconomicStats,
}

#[derive(Serialize, Clone)]
pub struct EconomicForecastDto {
    pub forecast_horizon_days: u32,
    pub gdp_growth_forecast: Option<f64>,
    pub inflation_forecast: Option<f64>,
    pub unemployment_forecast: Option<f64>,
    pub interest_rate_forecast: Option<f64>,
    pub confidence_interval: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct RiskMetricsDto {
    pub var_95: Option<f64>,
    pub var_99: Option<f64>,
    pub expected_shortfall: Option<f64>,
    pub max_drawdown: Option<f64>,
    pub sharpe_ratio: Option<f64>,
    pub sortino_ratio: Option<f64>,
    pub correlation_matrix: Option<Vec<Vec<f64>>>,
}

#[derive(Serialize, Clone)]
pub struct NetworkAnalyticsDto {
    pub network_density: Option<f64>,
    pub clustering_coefficient: Option<f64>,
    pub average_path_length: Option<f64>,
    pub systemic_risk_score: Option<f64>,
    pub interconnectedness_index: Option<f64>,
}