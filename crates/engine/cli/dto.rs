use serde::Serialize;

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
}

#[derive(Serialize, Clone)]
pub struct PolicyRates {
    pub policy_rate: f64,
    pub reserve_requirement: f64,
}

#[derive(Serialize, Clone)]
pub struct DashboardDto {
    pub current_date: String,
    pub tick_number: u64,
    pub total_iterations: u64,
    pub agent_counts: AgentCounts,
    pub employment_rate: f64,
    pub monetary_stats: MonetaryStats,
    pub central_bank_policy: PolicyRates,
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
}

#[derive(Serialize, Clone)]
pub struct YieldCurvePointDto {
    pub tenor: String,
    pub yield_pct: f64,
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
    pub overnight_rates: OvernightRatesDto, // Add detailed overnight rates
}

#[derive(Serialize, Clone)]
#[allow(non_snake_case)]
pub struct OvernightRatesDto {
    pub effr: Option<f64>,          // EFFR - federal funds market
    pub sofr: Option<f64>,          // SOFR - Treasury repo market
    pub iorb: Option<f64>,          // Interest on reserve balances (floor)
    pub discount_rate: Option<f64>, // Primary credit rate (ceiling)
    pub overnight_RRP: Option<f64>, // Overnight reverse repo (floor for nonbanks)
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
    pub market_id: String,     // e.g., "Goods(…)"
    pub good_id: String,
    pub name: String,
    pub unit: String,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub spread: Option<f64>,
    pub mid: Option<f64>,
    pub last: Option<f64>,
    pub depth: DepthDto,
}

#[derive(Serialize, Clone, Default)]
pub struct DepthDto {
    pub bid_size_at_best: f64,
    pub ask_size_at_best: f64,
    pub bid_levels: usize,
    pub ask_levels: usize,
}

#[derive(Serialize, Clone)]
pub struct TradeDto {
    pub ts: i64,               // epoch seconds (or tick index)
    pub price: f64,
    pub quantity: f64,
    pub buyer_id: String,
    pub seller_id: String,
}

#[derive(Serialize, Clone)]
pub struct CandleDto {
    pub ts: i64,               // start of bucket
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
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
    pub best_ask: Option<f64>,
    pub spread: Option<f64>,
    pub mid: Option<f64>,
    pub last: Option<f64>,
    pub depth: DepthDto,
}

#[derive(Serialize, Clone)]
pub struct FinancialMarketsPageDto {
    pub markets: Vec<FinancialMarketSummaryDto>,
}
