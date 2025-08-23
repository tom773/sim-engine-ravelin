use std::collections::HashMap;

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
pub struct LaborMarketStats {
    pub unemployment_rate: f64,
    pub labor_force_participation: f64,
    pub employment_population_ratio: Option<f64>,
    pub job_openings: f64,
    pub job_openings_rate: Option<f64>,
    pub quits_rate: Option<f64>,
    pub hires_rate: Option<f64>,
    pub layoffs_rate: Option<f64>,
    pub average_hourly_earnings: Option<f64>,
    pub average_weekly_hours: Option<f64>,
    pub nonfarm_payrolls_change: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct ProductionMetrics {
    pub capacity_utilization: f64,
    pub industrial_production: f64,
    pub manufacturing_pmi: Option<f64>,
    pub services_pmi: Option<f64>,
    pub housing_starts: f64,
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
pub struct TradeMetrics {
    pub trade_balance: f64,
    pub exports: Option<f64>,
    pub imports: Option<f64>,
    pub current_account_balance: Option<f64>,
    pub goods_trade_balance: Option<f64>,
    pub services_trade_balance: Option<f64>,
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
    pub housing_starts: f64,

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
pub struct MarketSentimentMetrics {
    pub fear_greed_index: Option<f64>,
    pub put_call_ratio: Option<f64>,
    pub insider_buying_selling_ratio: Option<f64>,
    pub short_interest_ratio: Option<f64>,
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
pub struct BalanceSheetSummary {
    pub assets: f64,
    pub liabilities: f64,
    pub equity: f64,
}

#[derive(Serialize, Clone)]
pub struct AgentSummaryDto {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub balance_sheet: BalanceSheetSummary,
}

#[derive(Serialize, Clone)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total_items: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Serialize, Clone)]
pub struct TreasuryMarketDto {
    pub instrument_id: String,
    pub name: String,
    pub price: f64,
    pub yield_to_maturity: f64,
    pub spread_bps: f64,
    pub daily_change_pct: f64,
    pub duration: Option<f64>,
    pub convexity: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct YieldCurvePointDto {
    pub tenor: String,
    pub yield_pct: f64,
    pub price: Option<f64>,
    pub change_bps: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct GoodsDto {
    pub id: String,
    pub name: String,
    pub unit: String,
}

#[derive(Serialize, Clone)]
pub struct RecipiesDto {
    pub id: String,
    pub name: String,
    pub inputs: Vec<GoodsDto>,
    pub output: GoodsDto,
    pub efficiency: f64,
    pub labour_hours: f64,
}

#[derive(Serialize, Clone)]
pub struct GoodsPageDto {
    pub goods: Vec<GoodsDto>,
    pub recipies: Vec<RecipiesDto>,
}

#[derive(Serialize, Clone)]
pub struct MarketsPageDto {
    pub treasuries: Vec<TreasuryMarketDto>,
    pub yield_curve: Vec<YieldCurvePointDto>,
    pub overnight_rates: OvernightRatesDto,
    pub market_summary: Option<MarketSummaryStats>,
}

#[derive(Serialize, Clone)]
pub struct MarketSummaryStats {
    pub total_volume_24h: f64,
    pub total_trades_24h: u64,
    pub avg_spread_bps: f64,
    pub market_cap_total: Option<f64>,
    pub volatility_index: Option<f64>,
}

#[derive(Serialize, Clone)]
#[allow(non_snake_case)]
pub struct OvernightRatesDto {
    pub effr: Option<f64>,
    pub sofr: Option<f64>,
    pub iorb: Option<f64>,
    pub discount_rate: Option<f64>,
    pub overnight_RRP: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct MarketBidDto {
    pub agent_id: String,
    pub quantity: f64,
    pub price: f64,
}

#[derive(Serialize, Clone)]
pub struct MarketAskDto {
    pub agent_id: String,
    pub quantity: f64,
    pub price: f64,
}

#[derive(Serialize, Clone)]
pub struct OrderbookDto {
    pub market_id: String,
    pub market_name: String,
    pub bids: Vec<MarketBidDto>,
    pub asks: Vec<MarketAskDto>,
}

#[derive(Serialize, Clone)]
pub struct GoodsMarketSummaryDto {
    pub market_id: String,
    pub good_id: String,
    pub name: String,
    pub unit: String,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub spread: Option<f64>,
    pub mid: Option<f64>,
    pub last: Option<f64>,
    pub depth: DepthDto,
    pub volume_24h: Option<f64>,
    pub price_change_24h: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct LabourMarketSummaryDto {
   pub market_id: String,
   pub name: String,
   pub job_offers: Vec<JobOfferDto>,
   pub job_applications: Vec<JobApplicationDto>, 
}
#[derive(Clone, Serialize)]
pub struct LabourMarketPageDto {
    pub markets: Vec<LabourMarketSummaryDto>,
}

#[derive(Clone, Serialize)]
pub struct JobOfferDto {
    pub offer_id: String,
    pub firm_id: String,
    pub wage_rate: f64,
    pub hours_required: f64,
    pub quantity: u32,
}

#[derive(Clone, Serialize)]
pub struct JobApplicationDto {
    pub application_id: String,
    pub consumer_id: String,
    pub reservation_wage: f64,
    pub hours_desired: f64,
}

#[derive(Serialize, Clone, Default)]
pub struct DepthDto {
    pub bid_size_at_best: f64,
    pub ask_size_at_best: f64,
    pub bid_levels: usize,
    pub ask_levels: usize,
}
#[derive(Serialize, Clone)]
pub struct EmploymentContractDto {
    pub employee_id: String,
    pub firm_id: String,
    pub wage_rate: f64,
    pub hours: f64,
}

#[derive(Serialize, Clone)]
pub struct CandleDto {
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub vwap: Option<f64>,
    pub trades_count: Option<u32>,
}

#[derive(Serialize, Clone)]
pub struct MarketHistoryDto {
    pub market_id: String,
    pub good_id: String,
    pub name: String,
    pub trades: Vec<TradeDto>,
    pub candles: Vec<CandleDto>,
}

#[derive(Serialize, Clone)]
pub struct GoodsMarketsPageDto {
    pub markets: Vec<GoodsMarketSummaryDto>,
}

#[derive(Serialize, Clone)]
pub struct FinancialMarketSummaryDto {
    pub market_id: String,     
    pub instrument_id: String,
    pub name: String,          
    pub best_bid: Option<f64>,
    pub best_bid_yield: Option<f64>,
    pub best_ask_yield: Option<f64>,
    pub best_ask: Option<f64>,
    pub spread: Option<f64>,
    pub mid: Option<f64>,
    pub last: Option<f64>,
    pub depth: DepthDto,
    pub volume_24h: Option<f64>,
    pub price_change_24h: Option<f64>,
    pub yield_to_maturity: Option<f64>,
    pub duration: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct FinancialMarketsPageDto {
    pub markets: Vec<FinancialMarketSummaryDto>,
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

#[derive(Serialize, Clone)]
pub struct DetailedAnalyticsDto {
    pub economic_forecast: Option<EconomicForecastDto>,
    pub risk_metrics: Option<RiskMetricsDto>,
    pub network_analytics: Option<NetworkAnalyticsDto>,
    pub market_microstructure: Option<MarketMicrostructureDto>,
}

#[derive(Serialize, Clone)]
pub struct MarketMicrostructureDto {
    pub bid_ask_spreads_avg: Option<f64>,
    pub order_flow_imbalance: Option<f64>,
    pub price_impact: Option<f64>,
    pub effective_spread: Option<f64>,
    pub realized_spread: Option<f64>,
    pub adverse_selection_cost: Option<f64>,
}



#[derive(Serialize, Clone)]
pub struct ActionDto {
    pub action_type: String,
    pub agent_id: String,
    pub agent_type: String,
    pub agent_name: Option<String>,
    pub details: serde_json::Value,
}

#[derive(Serialize, Clone)]
pub struct EffectDto {
    pub effect_type: String,
    pub details: serde_json::Value,
}

#[derive(Serialize, Clone)]
pub struct TradeDto {
    pub market_id: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub quantity: f64,
    pub price: f64,
}

#[derive(Serialize, Clone)]
pub struct TickRecordDto {
    pub tick_number: u32,
    pub date: String,
    pub actions: Vec<ActionDto>,
    pub effects: Vec<EffectDto>,
    pub action_to_effect_indices: HashMap<usize, Vec<usize>>,
    pub trades: Vec<TradeDto>,
    pub summary: TickSummaryDto,
}

#[derive(Serialize, Clone)]
pub struct TickSummaryDto {
    pub total_actions: usize,
    pub total_effects: usize,
    pub total_trades: usize,
    pub actions_by_type: std::collections::HashMap<String, usize>,
    pub effects_by_type: std::collections::HashMap<String, usize>,
    pub agents_active: usize,
}

#[derive(Serialize, Clone)]
pub struct SimulationHistoryDto {
    pub ticks: Vec<TickRecordDto>,
    pub total_ticks: usize,
    pub page: u32,
    pub page_size: u32,
}


impl From<&ActionRecord> for ActionDto {
    fn from(record: &ActionRecord) -> Self {
        ActionDto {
            action_type: record.action.name(),
            agent_id: record.agent_id.to_string(),
            agent_type: record.agent_type.clone(),
            agent_name: record.agent_name.clone(),
            details: serde_json::to_value(&record.action).unwrap_or(serde_json::Value::Null),
        }
    }
}

impl From<&StateEffect> for EffectDto {
    fn from(effect: &StateEffect) -> Self {
        EffectDto {
            effect_type: effect.name(),
            details: serde_json::to_value(effect).unwrap_or(serde_json::Value::Null),
        }
    }
}

impl From<&Trade> for TradeDto {
    fn from(trade: &Trade) -> Self {
        TradeDto {
            market_id: trade.market_id.to_string(),
            buyer_id: trade.buyer.to_string(),
            seller_id: trade.seller.to_string(),
            quantity: trade.quantity,
            price: trade.price,
        }
    }
}

impl From<&TickRecord> for TickRecordDto {
    fn from(record: &TickRecord) -> Self {
        let actions: Vec<ActionDto> = record.actions.iter().map(ActionDto::from).collect();
        let effects: Vec<EffectDto> = record.effects.iter().map(EffectDto::from).collect();
        let trades: Vec<TradeDto> = record.trades.iter().map(TradeDto::from).collect();
        let action_to_effect_idx: HashMap<usize, Vec<usize>> = record
            .action_to_effect_indices
            .iter()
            .map(|(action_idx, effect_indices)| (*action_idx, effect_indices.clone()))
            .collect();
        

        let mut actions_by_type = std::collections::HashMap::new();
        for action in &actions {
            *actions_by_type.entry(action.action_type.clone()).or_insert(0) += 1;
        }
        
        let mut effects_by_type = std::collections::HashMap::new();
        for effect in &effects {
            *effects_by_type.entry(effect.effect_type.clone()).or_insert(0) += 1;
        }
        let mut action_to_effect_indices = HashMap::new();
        for (action_idx, effect_indices) in &action_to_effect_idx {
            action_to_effect_indices.insert(*action_idx, effect_indices.clone());
        }
        
        let agents_active = actions.iter()
            .map(|a| &a.agent_id)
            .collect::<std::collections::HashSet<_>>()
            .len();
        
        let summary = TickSummaryDto {
            total_actions: actions.len(),
            total_effects: effects.len(),
            total_trades: trades.len(),
            actions_by_type,
            effects_by_type,
            agents_active,
        };
        
        TickRecordDto {
            tick_number: record.tick_number,
            date: record.date.format("%Y-%m-%d").to_string(),
            actions,
            effects,
            action_to_effect_indices,
            trades,
            summary,
        }
    }
}