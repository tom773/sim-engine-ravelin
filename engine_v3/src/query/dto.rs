use serde::{Deserialize, Serialize};
use sim_core::{ prelude::*};
use std::collections::HashMap;
use uuid::Uuid;
use serde_with::{serde_as, DisplayFromStr};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentCounts {
    pub banks: usize,
    pub firms: usize,
    pub consumers: usize,
    pub total: usize,
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

pub type DashboardDto = StatusDto;

#[derive(Serialize)]
pub struct AgentSummaryDto {
    pub id: Uuid,
    pub agent_type: String,
    pub name: String,
    pub balance_sheet: PopulatedBalanceSheetDto,
}

#[derive(Serialize)]
pub struct AgentDetailDto {
    pub id: Uuid,
    pub agent_type: String,
    pub name: String,
    pub balance_sheet: PopulatedBalanceSheetDto,
}

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
pub struct TickSummaryDto {
    pub tick_number: u32,
    pub date: String,
    pub intentions: usize,
    pub actions: usize,
    pub effects: usize,
    pub trades: usize,
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
    pub market_price: Option<Money>
}

#[derive(Serialize)]
pub struct PopulatedBalanceSheetDto {
    pub assets: Vec<PopulatedPositionDto>,
    pub liabilities: Vec<PopulatedPositionDto>,
    pub income_statement: IncomeStatement,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct MarketOverviewDto {
    pub instrument_registry: HashMap<InstrumentId, Instrument>,
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
    pub markets: HashMap<InstrumentId, Market>,
    
    pub index: MarketIndexDto,

    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub goods_markets: HashMap<GoodId, Market>,

    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub labour_markets: HashMap<LabourMarketId, LabourMarket>,
    
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub tape: HashMap<MarketId, Vec<TimedTrade>>,
    
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


impl From<&Exchange> for ExchangeDto {
    fn from(exchange: &Exchange) -> Self {
        let markets_dto = exchange
            .markets
            .iter()
            .map(|(id, market_generic)| (*id, market_generic.book.clone().into()))
            .collect();

        let goods_markets_dto = exchange
            .goods_markets
            .iter()
            .map(|(id, market_generic)| (*id, market_generic.book.clone().into()))
            .collect();

        Self {
            markets: markets_dto,
            index: MarketIndexDto::from(&exchange.index),
            goods_markets: goods_markets_dto,
            labour_markets: exchange.labour_markets.clone(),
            tape: exchange.tape.clone(),
        }
    }
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
    pub contracts: Vec<EmploymentRecordDto>
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