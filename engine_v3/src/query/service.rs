use crate::*;
use axum::response::Json;
use chrono::NaiveDate;
use parking_lot::{RwLock, RwLockReadGuard};
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use sim_core::prelude::*;
use sim_core::types::instrument::inst_core::MarketProfile;
use std::sync::Arc;
use std::{cmp::Ordering, collections::HashMap};
use uuid::Uuid;

pub struct QueryService {
    engine: Arc<RwLock<SimulationEngine>>,
}

#[derive(Default)]
struct AggregatedAccumulator {
    total_quantity: f64,
    total_book_value: f64,
    weighted_rate_sum: f64,
    weighted_original_term_sum: f64,
    weighted_remaining_term_sum: f64,
    rate_weight: f64,
    original_term_weight: f64,
    remaining_term_weight: f64,
    position_count: usize,
}
impl AggregatedAccumulator {
    fn add(
        &mut self, quantity: f64, book_value: f64, rate_bps: Option<f64>, original_term_days: Option<f64>,
        remaining_term_days: Option<f64>,
    ) {
        self.total_quantity += quantity;
        self.total_book_value += book_value;
        self.position_count += 1;

        let weight = book_value.abs();
        if let (Some(rate), true) = (rate_bps, weight.is_finite() && weight > 0.0) {
            self.weighted_rate_sum += rate * weight;
            self.rate_weight += weight;
        }
        if let (Some(days), true) = (original_term_days, weight.is_finite() && weight > 0.0) {
            self.weighted_original_term_sum += days * weight;
            self.original_term_weight += weight;
        }
        if let (Some(days), true) = (remaining_term_days, weight.is_finite() && weight > 0.0) {
            self.weighted_remaining_term_sum += days * weight;
            self.remaining_term_weight += weight;
        }
    }

    fn into_entry(self, label: String) -> AggregatedBookEntryDto {
        let average_rate_bps =
            if self.rate_weight > 0.0 { Some(self.weighted_rate_sum / self.rate_weight) } else { None };
        let average_original_term_days = if self.original_term_weight > 0.0 {
            Some(self.weighted_original_term_sum / self.original_term_weight)
        } else {
            None
        };
        let average_remaining_term_days = if self.remaining_term_weight > 0.0 {
            Some(self.weighted_remaining_term_sum / self.remaining_term_weight)
        } else {
            None
        };

        AggregatedBookEntryDto {
            label,
            position_count: self.position_count,
            total_quantity: self.total_quantity,
            total_book_value: self.total_book_value,
            average_rate_bps,
            average_original_term_days,
            average_remaining_term_days,
        }
    }
}

fn safe_book_value(position: &Position) -> f64 {
    let per_unit = position.book_value_per_unit.to_f64();
    if per_unit.is_finite() { position.quantity * per_unit } else { position.quantity }
}

fn loan_type_label(loan_type: LoanType) -> &'static str {
    match loan_type {
        LoanType::TermLoan => "Term Loans",
        LoanType::WorkingCapital => "Working Capital Loans",
        LoanType::BridgeLoan => "Bridge Loans",
        LoanType::MortgageLoan => "Mortgage Loans",
        LoanType::ProjectFinance => "Project Finance Loans",
        LoanType::AssetFinance => "Asset Finance Loans",
        LoanType::PersonalLoan => "Personal Loans",
    }
}

fn consumer_debt_label(debt: &ConsumerDebt) -> &'static str {
    match debt {
        ConsumerDebt::ResidentialMortgage(_) => "Residential Mortgages",
        ConsumerDebt::AutoLoan(_) => "Auto Loans",
        ConsumerDebt::PersonalLoan(_) => "Consumer Personal Loans",
        ConsumerDebt::StudentLoan(_) => "Student Loans",
        ConsumerDebt::CreditCard(_) => "Consumer Credit Cards",
    }
}

fn facility_type_label(facility_type: FacilityType) -> &'static str {
    match facility_type {
        FacilityType::Revolver => "Revolving Credit Facilities",
        FacilityType::TermLoanFacility => "Term Loan Facilities",
        FacilityType::Overdraft => "Overdraft Facilities",
        FacilityType::LetterOfCredit => "Letters of Credit",
        FacilityType::MultiCurrency => "Multi-currency Facilities",
    }
}

fn bond_type_label(bond_type: BondType) -> &'static str {
    match bond_type {
        BondType::Corporate => "Corporate Bonds",
        BondType::Government => "Government Bonds",
        BondType::InterbankLoan => "Interbank Loans",
        BondType::Municipal => "Municipal Bonds",
        BondType::Agency => "Agency Bonds",
        BondType::Supranational => "Supranational Bonds",
    }
}

fn cash_bucket_label(cash_type: CashType) -> Option<&'static str> {
    match cash_type {
        CashType::DemandDeposit => Some("Demand Deposits"),
        CashType::SavingsDeposit => Some("Savings Deposits"),
        CashType::TimeDeposit => Some("Time Deposits"),
        _ => None,
    }
}

struct AggregationDescriptor {
    label: String,
    rate_bps: Option<f64>,
    original_term_days: Option<f64>,
    remaining_term_days: Option<f64>,
}

fn accumulate_entry(
    groups: &mut HashMap<String, AggregatedAccumulator>, descriptor: AggregationDescriptor, position: &Position,
) {
    let AggregationDescriptor { label, rate_bps, original_term_days, remaining_term_days } = descriptor;
    let book_value = safe_book_value(position);
    groups.entry(label).or_default().add(
        position.quantity,
        book_value,
        rate_bps,
        original_term_days,
        remaining_term_days,
    );
}

fn non_negative_days(diff: i64) -> f64 {
    if diff < 0 { 0.0 } else { diff as f64 }
}

fn instrument_snapshot(state: &SimState, id: &InstrumentId, position: &Position) -> Option<PopulatedPositionDto> {
    state.financial_system.instruments.instruments.get(id).map(|inst| {
        let market_price =
            state.financial_system.exchange.financial_market(id).and_then(|book| book.representative_price());

        PopulatedPositionDto { position: position.clone(), instrument: inst.clone(), market_price }
    })
}

fn populate_positions<'a, I>(state: &SimState, positions: I) -> Vec<PopulatedPositionDto>
where
    I: IntoIterator<Item = (&'a InstrumentId, &'a Position)>,
{
    positions.into_iter().filter_map(|(id, pos)| instrument_snapshot(state, id, pos)).collect()
}

fn total_book_value<'a, I>(positions: I) -> f64
where
    I: IntoIterator<Item = &'a Position>,
{
    positions.into_iter().map(|pos| pos.quantity * pos.book_value_per_unit.to_f64()).sum()
}

fn build_agent_name_map(engine: &SimulationEngine) -> HashMap<AgentId, String> {
    let state = &engine.state;
    let mut map: HashMap<AgentId, String> = state
        .agents
        .all_agent_ids()
        .into_iter()
        .map(|id| {
            let name = engine.get_agent_info(&id).1.unwrap_or_else(|| "N/A".to_string());
            (id, name)
        })
        .collect();
    map.insert(state.financial_system.government.id, "Government".to_string());
    map.insert(state.financial_system.central_bank.id, "Central Bank".to_string());
    map
}

fn collect_employment_contracts(state: &SimState, agents_map: &HashMap<AgentId, String>) -> Vec<EmploymentRecordDto> {
    state
        .agents
        .firms
        .iter()
        .flat_map(|(fid, firm)| {
            let firm_name = agents_map.get(fid).cloned().unwrap_or_else(|| "Firm".to_string());
            firm.employees.values().cloned().map(move |contract| EmploymentRecordDto {
                firm_id: *fid,
                firm_name: firm_name.clone(),
                contract,
            })
        })
        .collect()
}

fn agent_kind(state: &SimState, id: &AgentId) -> &'static str {
    if state.agents.banks.contains_key(id) {
        "bank"
    } else if state.agents.firms.contains_key(id) {
        "firm"
    } else if state.agents.consumers.contains_key(id) {
        "consumer"
    } else if *id == state.financial_system.government.id {
        "government"
    } else if *id == state.financial_system.central_bank.id {
        "centralbank"
    } else {
        "unknown"
    }
}

fn derive_overnight_rates(base: BasisPoints) -> OvernightRatesDto {
    OvernightRatesDto {
        effr: Some(base + dec!(13.0)),
        sofr: Some(base + dec!(17.0)),
        iorb: Some(base),
        discount_rate: Some(base + dec!(20.0)),
        overnight_RRP: Some(base + dec!(25.0)),
    }
}

fn classify_asset_position(position: &PopulatedPositionDto, current_date: NaiveDate) -> Option<AggregationDescriptor> {
    match &position.instrument.instrument_type {
        InstrumentType::Debt(debt) => match debt {
            DebtInstrument::Loan(details) => Some(AggregationDescriptor {
                label: format!("{} Loan Book", loan_type_label(details.loan_type)),
                rate_bps: details.spread_bps.to_f64(),
                original_term_days: Some(non_negative_days(
                    (details.maturity_date - details.origination_date).num_days(),
                )),
                remaining_term_days: Some(non_negative_days((details.maturity_date - current_date).num_days())),
            }),
            DebtInstrument::Consumer(consumer) => match consumer {
                ConsumerDebt::CreditCard(details) => Some(AggregationDescriptor {
                    label: format!("{} Portfolio", consumer_debt_label(consumer)),
                    rate_bps: details.spread_bps.to_f64(),
                    original_term_days: Some(non_negative_days(
                        (details.expiry_date - details.commitment_date).num_days(),
                    )),
                    remaining_term_days: Some(non_negative_days((details.expiry_date - current_date).num_days())),
                }),
                ConsumerDebt::ResidentialMortgage(details)
                | ConsumerDebt::AutoLoan(details)
                | ConsumerDebt::PersonalLoan(details)
                | ConsumerDebt::StudentLoan(details) => Some(AggregationDescriptor {
                    label: format!("{} Portfolio", consumer_debt_label(consumer)),
                    rate_bps: details.spread_bps.to_f64(),
                    original_term_days: Some(non_negative_days(
                        (details.maturity_date - details.origination_date).num_days(),
                    )),
                    remaining_term_days: Some(non_negative_days((details.maturity_date - current_date).num_days())),
                }),
            },
            DebtInstrument::CreditLine(details) => Some(AggregationDescriptor {
                label: format!("{} Portfolio", facility_type_label(details.facility_type)),
                rate_bps: details.spread_bps.to_f64(),
                original_term_days: Some(non_negative_days((details.expiry_date - details.commitment_date).num_days())),
                remaining_term_days: Some(non_negative_days((details.expiry_date - current_date).num_days())),
            }),
            DebtInstrument::Bond(details) => Some(AggregationDescriptor {
                label: format!("{}", bond_type_label(details.bond_type)),
                rate_bps: details.coupon_rate_bps.to_f64(),
                original_term_days: Some(non_negative_days((details.maturity_date - details.issue_date).num_days())),
                remaining_term_days: Some(non_negative_days((details.maturity_date - current_date).num_days())),
            }),
            DebtInstrument::TradeCredit(details) => Some(AggregationDescriptor {
                label: "Trade Credit Assets".to_string(),
                rate_bps: None,
                original_term_days: Some(non_negative_days((details.due_date - details.invoice_date).num_days())),
                remaining_term_days: Some(non_negative_days((details.due_date - current_date).num_days())),
            }),
        },
        InstrumentType::StructuredTranche(details) => Some(AggregationDescriptor {
            label: "Structured Finance".to_string(),
            rate_bps: details.coupon_rate_bps.to_f64(),
            original_term_days: Some(non_negative_days((details.maturity_date - current_date).num_days())),
            remaining_term_days: Some(non_negative_days((details.maturity_date - current_date).num_days())),
        }),
        InstrumentType::Equity(_) => Some(AggregationDescriptor {
            label: "Equity Holdings".to_string(),
            rate_bps: None,
            original_term_days: None,
            remaining_term_days: None,
        }),
        InstrumentType::RealAsset(real) => match real {
            RealAssetType::Inventory { .. } => Some(AggregationDescriptor {
                label: "Inventory".to_string(),
                rate_bps: None,
                original_term_days: None,
                remaining_term_days: None,
            }),
            RealAssetType::Property { .. } => Some(AggregationDescriptor {
                label: "Property Holdings".to_string(),
                rate_bps: None,
                original_term_days: None,
                remaining_term_days: None,
            }),
        },
        InstrumentType::Derivative(details) => Some(AggregationDescriptor {
            label: "Derivatives".to_string(),
            rate_bps: None,
            original_term_days: Some(non_negative_days((details.expiry_date - current_date).num_days())),
            remaining_term_days: Some(non_negative_days((details.expiry_date - current_date).num_days())),
        }),
        InstrumentType::Repo(details) => Some(AggregationDescriptor {
            label: "Repo Financing".to_string(),
            rate_bps: details.interest_bps.to_f64(),
            original_term_days: Some(non_negative_days((details.end_date - details.start_date).num_days())),
            remaining_term_days: Some(non_negative_days((details.end_date - current_date).num_days())),
        }),
        _ => None,
    }
}

fn classify_liability_position(
    position: &PopulatedPositionDto, current_date: NaiveDate,
) -> Option<AggregationDescriptor> {
    match &position.instrument.instrument_type {
        InstrumentType::Cash(details) => {
            if details.cash_type == CashType::TreasuryGeneralAccount {
                return None;
            } else {
                cash_bucket_label(details.cash_type).map(|base| AggregationDescriptor {
                    label: format!("{}", base),
                    rate_bps: details.interest_bps.to_f64(),
                    original_term_days: None,
                    remaining_term_days: None,
                })
            }
        }
        InstrumentType::Debt(debt) => match debt {
            DebtInstrument::Bond(details) => Some(AggregationDescriptor {
                label: format!("{}", bond_type_label(details.bond_type)),
                rate_bps: details.coupon_rate_bps.to_f64(),
                original_term_days: Some(non_negative_days((details.maturity_date - details.issue_date).num_days())),
                remaining_term_days: Some(non_negative_days((details.maturity_date - current_date).num_days())),
            }),
            DebtInstrument::Loan(details) => Some(AggregationDescriptor {
                label: format!("{}", loan_type_label(details.loan_type)),
                rate_bps: details.spread_bps.to_f64(),
                original_term_days: Some(non_negative_days(
                    (details.maturity_date - details.origination_date).num_days(),
                )),
                remaining_term_days: Some(non_negative_days((details.maturity_date - current_date).num_days())),
            }),
            DebtInstrument::CreditLine(details) => Some(AggregationDescriptor {
                label: format!("{}", facility_type_label(details.facility_type)),
                rate_bps: details.spread_bps.to_f64(),
                original_term_days: Some(non_negative_days((details.expiry_date - details.commitment_date).num_days())),
                remaining_term_days: Some(non_negative_days((details.expiry_date - current_date).num_days())),
            }),
            DebtInstrument::TradeCredit(details) => Some(AggregationDescriptor {
                label: "Trade Payables".to_string(),
                rate_bps: None,
                original_term_days: Some(non_negative_days((details.due_date - details.invoice_date).num_days())),
                remaining_term_days: Some(non_negative_days((details.due_date - current_date).num_days())),
            }),
            DebtInstrument::Consumer(_) => None,
        },
        InstrumentType::Repo(details) => Some(AggregationDescriptor {
            label: "Repo Obligations".to_string(),
            rate_bps: details.interest_bps.to_f64(),
            original_term_days: Some(non_negative_days((details.end_date - details.start_date).num_days())),
            remaining_term_days: Some(non_negative_days((details.end_date - current_date).num_days())),
        }),
        _ => None,
    }
}

type QueryResult<T> = Result<T, (axum::http::StatusCode, String)>;

impl QueryService {
    pub fn new(engine: Arc<RwLock<SimulationEngine>>) -> Self {
        Self { engine }
    }

    fn get_engine_lock(&self) -> Result<RwLockReadGuard<'_, SimulationEngine>, (axum::http::StatusCode, String)> {
        Ok(self.engine.read())
    }

    fn build_balance_sheet_aggregates(
        &self, agent_id: &AgentId, state: &SimState, assets: &[PopulatedPositionDto],
        liabilities: &[PopulatedPositionDto],
    ) -> Option<BalanceSheetAggregatesDto> {
        if state.agents.consumers.contains_key(agent_id) {
            return None;
        }

        let current_date = state.current_date;

        let mut asset_groups: HashMap<String, AggregatedAccumulator> = HashMap::new();
        for asset in assets {
            if let Some(descriptor) = classify_asset_position(asset, current_date) {
                accumulate_entry(&mut asset_groups, descriptor, &asset.position);
            }
        }

        let mut liability_groups: HashMap<String, AggregatedAccumulator> = HashMap::new();
        for liability in liabilities {
            if let Some(descriptor) = classify_liability_position(liability, current_date) {
                accumulate_entry(&mut liability_groups, descriptor, &liability.position);
            }
        }

        let mut asset_books: Vec<_> = asset_groups.into_iter().map(|(label, acc)| acc.into_entry(label)).collect();
        asset_books.sort_by(|a, b| b.total_book_value.partial_cmp(&a.total_book_value).unwrap_or(Ordering::Equal));

        let mut liability_books: Vec<_> =
            liability_groups.into_iter().map(|(label, acc)| acc.into_entry(label)).collect();
        liability_books.sort_by(|a, b| b.total_book_value.partial_cmp(&a.total_book_value).unwrap_or(Ordering::Equal));

        if asset_books.is_empty() && liability_books.is_empty() {
            None
        } else {
            Some(BalanceSheetAggregatesDto { asset_books, liability_books })
        }
    }

    fn populate_balance_sheet(&self, agent_id: &AgentId, state: &SimState) -> PopulatedBalanceSheetDto {
        let all_assets_map = state.financial_system.get_agent_total_positions(agent_id);
        let assets = populate_positions(state, all_assets_map.iter());
        let bs = state.financial_system.balance_sheets.get(agent_id).unwrap();
        let liabilities = populate_positions(state, bs.liabilities.iter());

        let total_assets = total_book_value(all_assets_map.values());
        let total_liabilities = total_book_value(bs.liabilities.values());

        let net_worth = total_assets - total_liabilities;

        let equity = if net_worth.abs() > 1e-2 {
            let unit_money = Money::from(1_i64);
            let position =
                Position { quantity: net_worth, book_value_per_unit: unit_money, cost_basis_per_unit: unit_money };
            let instrument = Instrument::new(
                InstrumentId(Uuid::new_v4()),
                InstrumentType::Equity(EquityDetails { issuer: *agent_id, outstanding_shares: 1 }),
                MarketProfile::from_market(InstrumentMarket::CapitalMarket(CapitalMarketSegment::Equity)),
            );

            Some(PopulatedPositionDto { position, instrument, market_price: None })
        } else {
            None
        };

        let aggregates = self.build_balance_sheet_aggregates(agent_id, state, &assets, &liabilities);

        PopulatedBalanceSheetDto {
            assets,
            liabilities,
            equity,
            income_statement: bs.income_statement.clone(),
            total_assets,
            total_liabilities,
            net_worth,
            aggregates,
        }
    }

    pub fn get_dashboard_bundle(&self) -> QueryResult<DashboardBundleDto> {
        Ok(DashboardBundleDto {
            status: self.get_status_data()?,
            market_summaries: self.get_markets_summary()?,
            instruments: self.get_instrument_registry()?,
            cb_actions: self.get_cb_actions()?,
        })
    }

    pub fn get_markets_page_data(&self) -> QueryResult<MarketsPageDto> {
        Ok(MarketsPageDto {
            infrastructure: self.get_financial_infrastructure_state()?,
            omo_actions: self.get_cb_actions()?,
            instruments: self.get_instrument_registry()?,
            dashboard: self.get_status_data()?,
            goods: self.get_catalog()?,
            tape: self.get_exchange()?.tape.into_iter().map(|(k, v)| (k.0, v)).collect(),
        })
    }

    pub fn get_status_data(&self) -> QueryResult<StatusDto> {
        let engine_lock = self.get_engine_lock()?;
        let state = &engine_lock.state;

        let macro_stats = state.macro_stats();
        let agent_counts = AgentCounts::from(state);
        let agents_map = build_agent_name_map(&engine_lock);
        let instruments_map: HashMap<InstrumentId, String> = state
            .financial_system
            .instruments
            .instruments
            .iter()
            .map(|(id, inst)| (id.clone(), inst.type_as_string().to_string()))
            .collect();

        let labour_stats =
            LabourMarketStatsDto::from_macro(&macro_stats, collect_employment_contracts(state, &agents_map));
        let policy_rate = state.financial_system.central_bank.policy_rate_bps;
        let monetary_stats_dto = MonetaryStatsDto {
            policy_rate,
            reserve_requirement: Rate::from_f64(state.financial_system.central_bank.reserve_requirement)
                .unwrap_or_default(),
            overnight_rates: derive_overnight_rates(policy_rate),
        };

        Ok(StatusDto {
            current_date: state.current_date.format("%Y-%m-%d").to_string(),
            tick_number: state.ticknum,
            total_iterations: state.config.iterations,
            agent_counts,
            macro_stats: MacroStatsDto::from(&macro_stats),
            monetary_stats: monetary_stats_dto,
            labor_force_stats: labour_stats,
            maps: MapsDto { agents_map: agents_map.clone(), instruments_map },
        })
    }

    pub fn get_agents_summary(&self, agent_type_filter: Option<String>) -> QueryResult<Vec<AgentDto>> {
        let engine = self.get_engine_lock()?;
        let state = &engine.state;

        let filter = agent_type_filter.as_ref().map(|s| s.to_lowercase());
        let mut ids = Vec::new();
        ids.extend(state.agents.banks.keys().cloned());
        ids.extend(state.agents.firms.keys().cloned());
        ids.extend(state.agents.consumers.keys().cloned());
        ids.push(state.financial_system.government.id);
        ids.push(state.financial_system.central_bank.id);

        let mut summaries = Vec::new();
        for id in ids {
            if let Some(ref expected) = filter {
                if expected != agent_kind(state, &id) {
                    continue;
                }
            }

            let (agent_type_str, name_opt) = engine.get_agent_info(&id);
            let populated_bs = self.populate_balance_sheet(&id, state);
            summaries.push(AgentDto {
                id: id.0,
                agent_type: agent_type_str,
                name: name_opt.unwrap_or_else(|| "N/A".to_string()),
                balance_sheet: populated_bs,
            });
        }

        Ok(summaries)
    }

    pub fn get_agent_detail(&self, agent_id: Uuid) -> QueryResult<AgentDto> {
        let engine = self.get_engine_lock()?;
        let state = &engine.state;
        let agent_id = AgentId(agent_id);

        let (agent_type, name) = engine.get_agent_info(&agent_id);
        if agent_type == "Unknown" {
            return Err((axum::http::StatusCode::NOT_FOUND, format!("Agent with ID {} not found", agent_id)));
        }

        let populated_bs = self.populate_balance_sheet(&agent_id, state);

        Ok(AgentDto {
            id: agent_id.0,
            agent_type,
            name: name.unwrap_or_else(|| "N/A".to_string()),
            balance_sheet: populated_bs,
        })
    }

    fn calculate_yields(
        &self, inst_id: &InstrumentId, book: &OrderBook, pricer: &dyn Pricer<FinancialProduct>,
    ) -> (Option<Rate>, Option<Rate>, Option<Rate>, Option<Rate>) {
        let yield_bid = book.best_bid().and_then(|px| pricer.yield_from_price(inst_id, px)).and_then(Rate::from_f64);
        let yield_ask = book.best_ask().and_then(|px| pricer.yield_from_price(inst_id, px)).and_then(Rate::from_f64);
        let yield_mid = book.mid_price().and_then(|px| pricer.yield_from_price(inst_id, px)).and_then(Rate::from_f64);
        let yield_last =
            book.last_price.and_then(|price| pricer.yield_from_price(inst_id, price)).and_then(Rate::from_f64);

        (yield_bid, yield_ask, yield_mid, yield_last)
    }

    pub fn get_markets_summary(&self) -> QueryResult<Vec<MarketSummaryDto>> {
        let engine = self.get_engine_lock()?;
        let state = &engine.state;
        let mut summaries = vec![];

        for (symbol, market) in &state.financial_system.exchange.markets {
            match market {
                MarketType::Financial(fin_market) => {
                    let inst_id = &fin_market.key;
                    let view = state.market_view(symbol).unwrap_or_default();

                    let (yield_bid, yield_ask, yield_mid, yield_last) =
                        self.calculate_yields(inst_id, &fin_market.book, &*fin_market.pricer);

                    summaries.push(MarketSummaryDto {
                        market_id: symbol.to_string(),
                        market_type: "financial".to_string(),
                        name: state
                            .financial_system
                            .instruments
                            .instruments
                            .get(inst_id)
                            .map_or("Unknown".to_string(), |i| i.type_as_string().to_string()),
                        last_price: view.last.and_then(Rate::from_f64),
                        mid_price: fin_market.book.mid_price().map(|m| m.0),
                        best_bid: fin_market.book.best_bid().map(|m| m.0),
                        best_ask: fin_market.book.best_ask().map(|m| m.0),
                        spread: fin_market.book.spread().map(|m| m.0),
                        volume: view.volume,
                        turnover: view.turnover,
                        depth: Some(fin_market.book.depth_summary().into()),
                        yield_bid,
                        yield_ask,
                        yield_mid,
                        yield_last,
                    });
                }
                MarketType::Goods(goods_market) => {
                    let good_id = &goods_market.key;
                    let view = state.market_view(symbol).unwrap_or_default();
                    let depth_dto = goods_market.book.depth_summary().into();

                    summaries.push(MarketSummaryDto {
                        market_id: symbol.to_string(),
                        market_type: "goods".to_string(),
                        name: state
                            .financial_system
                            .goods
                            .goods
                            .get(good_id)
                            .map_or("Unknown".to_string(), |g| g.name.clone()),
                        last_price: view.last.and_then(Rate::from_f64),
                        mid_price: goods_market.book.mid_price().map(|m| m.0),
                        best_bid: goods_market.book.best_bid().map(|m| m.0),
                        best_ask: goods_market.book.best_ask().map(|m| m.0),
                        spread: goods_market.book.spread().map(|m| m.0),
                        volume: view.volume,
                        turnover: view.turnover,
                        depth: Some(depth_dto),
                        yield_bid: None,
                        yield_ask: None,
                        yield_mid: None,
                        yield_last: None,
                    });
                }
                MarketType::Labour(_labour_market) => {
                    summaries.push(MarketSummaryDto {
                        market_id: symbol.to_string(),
                        market_type: "labour".to_string(),
                        name: "General Labour Market".to_string(),
                        last_price: None,
                        mid_price: None,
                        best_bid: None,
                        best_ask: None,
                        spread: None,
                        volume: 0.0,
                        turnover: 0.0,
                        depth: None,
                        yield_bid: None,
                        yield_ask: None,
                        yield_mid: None,
                        yield_last: None,
                    });
                }
            }
        }
        Ok(summaries)
    }

    pub fn get_catalog(&self) -> QueryResult<CatalogDto> {
        let engine = self.get_engine_lock()?;
        Ok(CatalogDto {
            goods: engine.state.financial_system.goods.goods.values().cloned().collect(),
            recipes: engine.state.financial_system.goods.recipes.values().cloned().collect(),
        })
    }

    pub fn get_instrument_registry(&self) -> QueryResult<InstrumentRegistryDto> {
        let engine = self.get_engine_lock()?;
        let registry = &engine.state.financial_system.instrument_registry;
        Ok(InstrumentRegistryDto {
            instruments: engine.state.financial_system.instruments.instruments.clone(),
            templates: registry.templates.values().cloned().collect(),
            series: registry.series.values().cloned().collect(),
            lots: registry.lots.values().cloned().collect(),
        })
    }

    pub fn get_market_detail(&self, market_id_str: &str) -> QueryResult<MarketDetailDto> {
        let engine = self.get_engine_lock()?;
        let state = &engine.state;

        let symbol = Symbol(market_id_str.to_string());

        let market = state
            .financial_system
            .exchange
            .markets
            .get(&symbol)
            .ok_or((axum::http::StatusCode::NOT_FOUND, "Market not found".to_string()))?;

        match market {
            MarketType::Financial(fin_market) => {
                let name = state
                    .financial_system
                    .instruments
                    .instruments
                    .get(&fin_market.key)
                    .map_or("N/A".to_string(), |i| i.type_as_string().to_string());
                Ok(MarketDetailDto { market_id: market_id_str.to_string(), name, order_book: fin_market.book.clone() })
            }
            MarketType::Goods(goods_market) => {
                let name = state
                    .financial_system
                    .goods
                    .goods
                    .get(&goods_market.key)
                    .map_or("N/A".to_string(), |g| g.name.clone());
                Ok(MarketDetailDto {
                    market_id: market_id_str.to_string(),
                    name,
                    order_book: goods_market.book.clone(),
                })
            }
            MarketType::Labour(_) => Ok(MarketDetailDto {
                market_id: market_id_str.to_string(),
                name: "Labour Market".to_string(),
                order_book: OrderBook::default(),
            }),
        }
    }

    pub fn get_market_history(&self, market_id_str: &str) -> QueryResult<Vec<MarketTick>> {
        let engine = self.get_engine_lock()?;
        let state = &engine.state;

        let symbol = Symbol(market_id_str.to_string());

        let history = state
            .history
            .market_ticks
            .get(&symbol)
            .map(|deque| deque.iter().cloned().collect())
            .unwrap_or_else(Vec::new);

        Ok(history)
    }

    pub fn get_tick_detail(&self, tick_number: u32) -> QueryResult<TickDetailDto> {
        let engine = self.get_engine_lock()?;
        let record = engine
            .state
            .history
            .tick_records
            .iter()
            .find(|r| r.tick_number == tick_number)
            .ok_or((axum::http::StatusCode::NOT_FOUND, format!("Tick {} not found in history.", tick_number)))?;

        Ok(TickDetailDto {
            tick_number: record.tick_number,
            date: record.date.format("%Y-%m-%d").to_string(),
            intentions: record.intentions.clone(),
            actions: record.actions.clone(),
            effects: record.effects.clone(),
            trades: record.trades.clone(),
            action_to_effect_indices: record.action_to_effect_indices.clone(),
        })
    }

    pub fn get_exchange(&self) -> QueryResult<ExchangeDto> {
        let engine = self.get_engine_lock()?;
        let exchange = &engine.state.financial_system.exchange;
        let exchange_data = Json(ExchangeDto::from(exchange));
        Ok(exchange_data.0)
    }

    pub fn get_financial_infrastructure_state(&self) -> QueryResult<FinancialInfrastructureDto> {
        let engine_lock = self.get_engine_lock()?;
        let state = &engine_lock.state;

        let csd_dto = CsdStateDto {
            custody_accounts: state.financial_system.clearing_house.csd.custody_accounts.clone(),
            pending_settlements: state.financial_system.clearing_house.csd.pending_settlements.clone(),
            settlement_history: state.financial_system.clearing_house.csd.settlement_history.clone(),
            registered_securities: state.financial_system.clearing_house.csd.registered_securities.clone(),
        };

        let rtgs_dto = RtgsStateDto {
            pending_payments: state.financial_system.rtgs.pending.clone(),
            settled_payments: state.financial_system.rtgs.settled.clone(),
            rejected_payments: state.financial_system.rtgs.rejected.clone(),
        };

        let cred_dto = CreditRegistryDto {
            applications: state.financial_system.credit_registry.applications.clone(),
            loans: state.financial_system.credit_registry.loans.clone(),
            loans_by_borrower: state.financial_system.credit_registry.loans_by_borrower.clone(),
            loans_by_lender: state.financial_system.credit_registry.loans_by_lender.clone(),
            applications_by_bank: state.financial_system.credit_registry.applications_by_bank.clone(),
            credit_histories: state.financial_system.credit_registry.credit_histories.clone(),
        };
        let on_markets = OvernightMarketsDto {
            fedfunds_on: state.financial_system.funding_markets.fedfunds_on.clone(),
            repo_gc1d: state.financial_system.funding_markets.repo_gc1d.clone(),
        };
        Ok(FinancialInfrastructureDto {
            csd: csd_dto,
            rtgs: rtgs_dto,
            cred_reg: cred_dto,
            overnight_markets: on_markets,
        })
    }

    pub fn get_cb_actions(&self) -> QueryResult<Vec<SimIntention>> {
        let engine_lock = self.get_engine_lock()?;
        let state = &engine_lock.state;

        let intentions: Vec<SimIntention> = state
            .history
            .tick_records
            .iter()
            .flat_map(|r| r.intentions.clone())
            .filter(|intent| {
                matches!(intent, SimIntention::Monetary(m) if {
                    matches!(
                        m,
                        MonetaryIntention::ConductOMO { .. }
                            | MonetaryIntention::SetPolicyRate { .. }
                            | MonetaryIntention::AdjustReserveRequirement { .. }
                            | MonetaryIntention::ProvideLiquidityFacility { .. }
                    )
                })
            })
            .collect();

        Ok(intentions)
    }
}
