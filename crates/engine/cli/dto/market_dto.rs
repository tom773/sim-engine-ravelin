use serde::Serialize;
use crate::*;

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
pub struct MarketSentimentMetrics {
    pub fear_greed_index: Option<f64>,
    pub put_call_ratio: Option<f64>,
    pub insider_buying_selling_ratio: Option<f64>,
    pub short_interest_ratio: Option<f64>,
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
pub struct TradeDto {
    pub market_id: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub quantity: f64,
    pub price: f64,
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