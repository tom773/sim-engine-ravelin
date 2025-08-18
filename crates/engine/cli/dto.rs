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
pub struct MarketsPageDto {
    pub treasuries: Vec<TreasuryMarketDto>,
    pub yield_curve: Vec<YieldCurvePointDto>,
    pub sofr: f64,
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
pub struct ReferenceRatesDto {
    
    pub discount_window_rate: f64, // Fed will always lend dollars at this rate // UPPER BOUND -> Counterparty is Fed
    pub iorb: f64, // Interest on Reserve Balances - the rate paid on reserves held at the Fed -> Counterparty is Fed
    pub effr: f64, // Effective Federal Funds Rate - the average rate at which banks lend reserves to each other overnight -> Counterparty is another bank
    pub sofr: f64, // Secured Overnight Financing Rate - a market determined rate based on overnight repurchase agreements -> Counterparty is another bank
    pub reverse_repo_rate: f64, // Fed will always borrow dollars from banks // LOWER BOUND -> Counterparty is Fed

    pub cb_reserve_requirement: f64,
}