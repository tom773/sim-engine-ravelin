use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use sim_core::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentCounts {
    pub banks: usize,
    pub firms: usize,
    pub consumers: usize,
    pub total: usize,
}

impl From<&SimState> for AgentCounts {
    fn from(state: &SimState) -> Self {
        let banks = state.agents.banks.len();
        let firms = state.agents.firms.len();
        let consumers = state.agents.consumers.len();
        Self { banks, firms, consumers, total: banks + firms + consumers }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MacroStatsDto {
    pub nominal_gdp_proxy: f64,
    pub consumer_spending_daily: f64,
    pub household_debt: f64,
    pub corporate_debt: f64,
    pub government_debt: f64,
    pub cpi: f64,
    pub inflation_rate: f64,
    pub unemployment_rate: f64,
    pub m0: f64,
    pub m1: f64,
    pub m2: f64,
    pub bank_reserves: f64,
}

impl From<&MacroStats> for MacroStatsDto {
    fn from(stats: &MacroStats) -> Self {
        Self {
            nominal_gdp_proxy: stats.nominal_gdp_proxy,
            consumer_spending_daily: stats.consumer_spending_daily,
            household_debt: stats.household_debt,
            corporate_debt: stats.corporate_debt,
            government_debt: stats.government_debt,
            cpi: stats.cpi,
            inflation_rate: stats.inflation_rate,
            unemployment_rate: stats.unemployment_rate,
            m0: stats.m0,
            m1: stats.m1,
            m2: stats.m2,
            bank_reserves: stats.bank_reserves,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatusDto {
    pub current_date: String,
    pub tick_number: u32,
    pub total_iterations: u32,
    pub agent_counts: AgentCounts,
    pub macro_stats: MacroStatsDto,
    pub monetary_stats: MonetaryStatsDto,
    pub labor_force_stats: LabourMarketStatsDto,
    pub maps: MapsDto,
}

#[derive(Serialize)]
pub struct AgentDto {
    pub id: Uuid,
    pub agent_type: String,
    pub name: String,
    pub balance_sheet: PopulatedBalanceSheetDto,
}

pub type AgentSummaryDto = AgentDto;
pub type AgentDetailDto = AgentDto;

#[derive(Serialize, Clone, Debug)]
pub struct OrderBookDepthDto {
    pub bid_levels: HashMap<String, f64>,
    pub ask_levels: HashMap<String, f64>,
    pub bid_size_at_best: f64,
    pub ask_size_at_best: f64,
}

#[derive(Serialize)]
pub struct MarketSummaryDto {
    pub market_id: String,
    pub market_type: String, // "financial", "goods", "labour"
    pub name: String,
    pub last_price: Option<Rate>,
    pub mid_price: Option<Rate>,
    pub best_bid: Option<Rate>,
    pub best_ask: Option<Rate>,
    pub spread: Option<Rate>,
    pub volume: f64,
    pub turnover: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<OrderBookDepthDto>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_bid: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_ask: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_mid: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_last: Option<Rate>,
}

#[derive(Serialize)]
pub struct CatalogDto {
    pub goods: Vec<Good>,
    pub recipes: Vec<ProductionRecipe>,
}

#[derive(Serialize)]
pub struct MarketDetailDto {
    pub market_id: String,
    pub name: String,
    pub order_book: OrderBook,
}

#[derive(Serialize)]
pub struct TickDetailDto {
    pub tick_number: u32,
    pub date: String,
    pub intentions: Vec<SimIntention>,
    pub actions: Vec<ActionRecord>,
    pub effects: Vec<StateEffect>,
    pub trades: Vec<Trade>,
    pub action_to_effect_indices: HashMap<usize, Vec<usize>>,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct MonetaryStatsDto {
    pub policy_rate: BasisPoints,
    pub reserve_requirement: Rate,
    pub overnight_rates: OvernightRatesDto,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct OvernightRatesDto {
    pub effr: Option<BasisPoints>,
    pub sofr: Option<BasisPoints>,
    pub iorb: Option<BasisPoints>,
    pub discount_rate: Option<BasisPoints>,
    pub overnight_RRP: Option<BasisPoints>,
}

#[derive(Serialize)]
pub struct PopulatedPositionDto {
    pub position: Position,
    pub instrument: Instrument,
    pub market_price: Option<Money>,
}

#[derive(Serialize, Clone, Debug)]
pub struct AggregatedBookEntryDto {
    pub label: String,
    pub position_count: usize,
    pub total_quantity: f64,
    pub total_book_value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_rate_bps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_original_term_days: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_remaining_term_days: Option<f64>,
}

#[derive(Serialize, Clone, Debug)]
pub struct BalanceSheetAggregatesDto {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub asset_books: Vec<AggregatedBookEntryDto>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub liability_books: Vec<AggregatedBookEntryDto>,
}

#[derive(Serialize)]
pub struct PopulatedBalanceSheetDto {
    pub assets: Vec<PopulatedPositionDto>,
    pub liabilities: Vec<PopulatedPositionDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equity: Option<PopulatedPositionDto>,
    pub income_statement: IncomeStatement,
    pub total_assets: f64,
    pub total_liabilities: f64,
    pub net_worth: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregates: Option<BalanceSheetAggregatesDto>,
}

#[derive(Serialize, Debug, Clone)]
pub struct MarketIndexDto {
    pub by_issuer: HashMap<AgentId, Vec<InstrumentId>>,
    pub by_rating_and_tenor: HashMap<String, Vec<InstrumentId>>,
    pub by_bond_type: HashMap<BondType, Vec<InstrumentId>>,
}

#[serde_as]
#[derive(Serialize, Clone, Debug)]
pub struct InstrumentRegistryDto {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub instruments: HashMap<InstrumentId, Instrument>,
}

#[serde_as]
#[derive(Serialize, Clone, Debug)]
pub struct ExchangeDto {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub markets: HashMap<Symbol, MarketTypeDto>,

    pub index: MarketIndexDto,

    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub tape: HashMap<Symbol, Vec<TimedTrade>>,
}

#[derive(Serialize, Clone, Debug)]
pub enum MarketTypeDto {
    Financial { book: OrderBook },
    Goods { book: OrderBook },
    Labour { market: LabourMarket },
}

#[derive(Serialize)]
pub struct DashboardBundleDto {
    pub status: StatusDto,
    pub market_summaries: Vec<MarketSummaryDto>,
    pub instruments: InstrumentRegistryDto,
    pub cb_actions: Vec<SimIntention>,
}

impl From<&Exchange> for ExchangeDto {
    fn from(exchange: &Exchange) -> Self {
        let markets_dto = exchange
            .markets
            .iter()
            .map(|(symbol, market_type)| {
                let dto = match market_type {
                    MarketType::Financial(m) => MarketTypeDto::Financial { book: m.book.clone() },
                    MarketType::Goods(m) => MarketTypeDto::Goods { book: m.book.clone() },
                    MarketType::Labour(m) => MarketTypeDto::Labour { market: m.clone() },
                };
                (symbol.clone(), dto)
            })
            .collect();

        Self { markets: markets_dto, index: MarketIndexDto::from(&exchange.index), tape: exchange.tape.clone() }
    }
}

impl From<&MarketIndex> for MarketIndexDto {
    fn from(market_index: &MarketIndex) -> Self {
        let by_rating_and_tenor_dto = market_index
            .by_rating_and_tenor
            .iter()
            .map(|((rating, tenor), instruments)| {
                let key = format!("{:?}-{:?}", rating, tenor);
                (key, instruments.clone())
            })
            .collect();

        Self {
            by_issuer: market_index.by_issuer.clone(),
            by_rating_and_tenor: by_rating_and_tenor_dto,
            by_bond_type: market_index.by_bond_type.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct MarketsPageDto {
    pub infrastructure: FinancialInfrastructureDto,
    pub omo_actions: Vec<SimIntention>,
    pub instruments: InstrumentRegistryDto,
    pub dashboard: StatusDto,
    pub goods: CatalogDto,
    pub tape: HashMap<String, Vec<TimedTrade>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvernightMarketsDto {
    pub fedfunds_on: Vec<ONQuote>,
    pub repo_gc1d: Vec<ONQuote>,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CsdStateDto {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub custody_accounts: HashMap<AgentId, CustodyAccount>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub pending_settlements: HashMap<TradeId, SettlementInstruction>,
    pub settlement_history: Vec<CompletedSettlement>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub registered_securities: HashMap<InstrumentId, SecurityInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RtgsStateDto {
    pub pending_payments: Vec<PaymentInstruction>,
    pub settled_payments: Vec<PaymentInstruction>,
    pub rejected_payments: Vec<(PaymentInstruction, String)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialInfrastructureDto {
    pub csd: CsdStateDto,
    pub rtgs: RtgsStateDto,
    pub cred_reg: CreditRegistryDto,
    pub overnight_markets: OvernightMarketsDto,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapsDto {
    pub agents_map: HashMap<AgentId, String>,
    pub instruments_map: HashMap<InstrumentId, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreditRegistryDto {
    pub applications: HashMap<Uuid, LoanApplication>,
    pub loans: HashMap<Uuid, Loan>,
    pub loans_by_borrower: HashMap<AgentId, Vec<Uuid>>,
    pub loans_by_lender: HashMap<AgentId, Vec<Uuid>>,
    pub applications_by_bank: HashMap<AgentId, Vec<Uuid>>,
    pub credit_histories: HashMap<AgentId, CreditHistory>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmploymentRecordDto {
    pub firm_id: AgentId,
    pub firm_name: String,
    pub contract: EmploymentContract,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LabourMarketStatsDto {
    pub employment: usize,
    pub unemployment: usize,
    pub labour_force: usize,
    pub unemployment_rate: f64,
    pub payroll_proxy: usize,
    pub avg_wage_rate: f64,
    pub labor_force_participation: f64,
    pub job_openings: usize,
    pub contracts: Vec<EmploymentRecordDto>,
}

impl LabourMarketStatsDto {
    pub fn from_macro(macro_stats: &MacroStats, contracts: Vec<EmploymentRecordDto>) -> Self {
        Self {
            employment: macro_stats.employment,
            unemployment: macro_stats.unemployment,
            labour_force: macro_stats.labour_force,
            unemployment_rate: macro_stats.unemployment_rate,
            payroll_proxy: macro_stats.payroll_proxy as usize,
            avg_wage_rate: macro_stats.avg_wage_rate,
            labor_force_participation: macro_stats.labor_force_participation,
            job_openings: macro_stats.job_openings as usize,
            contracts,
        }
    }
}

impl From<MarketDepthSummary> for OrderBookDepthDto {
    fn from(summary: MarketDepthSummary) -> Self {
        Self {
            bid_levels: summary.bid_levels.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            ask_levels: summary.ask_levels.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            bid_size_at_best: summary.bid_size_at_best,
            ask_size_at_best: summary.ask_size_at_best,
        }
    }
}
