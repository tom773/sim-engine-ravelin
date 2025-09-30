use chrono::NaiveDate;
use engine_v3::SimulationEngine;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use sim_core::prelude::*;
use sim_core::types::core_utils::time::BasisPoints;
use sim_core::types::instrument::archetypes::{BondType, CashType, FacilityType, RateIndex, RateStructure};
use sim_core::types::instrument::{
    BondState, CashState, ConsumerLoanCategory, CreditFacilityState, CreditState, Currency, DerivativeContract,
    DerivativeState, EquityClass, EquityState, Instrument, InstrumentRuntime, LoanState, LoanType, Money, OptionStyle,
    RealAssetState, RepoState, StructuredTrancheState, TradeCreditState, UnderlyingAsset,
};
use sim_core::types::system::{
    balance_sheet::{IncomeStatement, Position},
    financial_system::FinancialSystem,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;


#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsDigest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalogue: Option<AgentsCatalogueDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsCatalogueDigest {
    pub roster: Vec<AgentBalanceDigest>,
    #[serde(serialize_with = "serialize_groups_as_map")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<AgentGroupDigest>,
}

fn serialize_groups_as_map<S>(groups: &Vec<AgentGroupDigest>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let map: HashMap<&str, Vec<String>> = groups
        .iter()
        .map(|g| (g.agent_type.as_str(), g.agent_ids.iter().map(|uuid| uuid.to_string()).collect()))
        .collect();
    map.serialize(serializer)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentGroupDigest {
    pub agent_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub agent_ids: Vec<Uuid>,
}

use serde_with::{DisplayFromStr, serde_as};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentBalanceDigest {
    #[serde_as(as = "DisplayFromStr")]
    pub agent_id: Uuid,
    pub agent_type: String,
    pub name: String,
    pub total_assets: f64,
    pub total_liabilities: f64,
    pub net_worth: f64,
    pub liquidity: f64,
    pub balance_sheet: BalanceSheetDigest,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BalanceSheetDigest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<BalanceEntryDigest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub liabilities: Vec<BalanceEntryDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub income_statement: Option<IncomeStatementDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BalanceEntrySource {
    BalanceSheet,
    Custody,
}

impl Default for BalanceEntrySource {
    fn default() -> Self {
        BalanceEntrySource::BalanceSheet
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BalanceEntryDigest {
    pub instrument_id: String,
    pub instrument_type: String,
    pub label: String,
    pub quantity: f64,
    pub mark_to_market_value: f64,
    pub book_value: f64,
    pub cost_basis: f64,
    pub source: BalanceEntrySource,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IncomeStatementDigest {
    pub revenue: f64,
    pub cost_of_goods_sold: f64,
    pub operating_expenses: f64,
    pub interest_income: f64,
    pub interest_expense: f64,
    pub net_income: f64,
}

impl From<&IncomeStatement> for IncomeStatementDigest {
    fn from(src: &IncomeStatement) -> Self {
        Self {
            revenue: money_to_f64(&src.revenue),
            cost_of_goods_sold: money_to_f64(&src.cost_of_goods_sold),
            operating_expenses: money_to_f64(&src.operating_expenses),
            interest_income: money_to_f64(&src.interest_income),
            interest_expense: money_to_f64(&src.interest_expense),
            net_income: money_to_f64(&src.net_income),
        }
    }
}

pub(crate) fn money_to_f64(value: &Money) -> f64 {
    let raw = value.to_f64();
    if raw.is_finite() { raw } else { 0.0 }
}

pub(crate) fn sanitize_f64(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

pub(crate) fn compute_agents(engine: &SimulationEngine, _limit: usize) -> AgentsDigest {
    let state = &engine.state;
    let financial_system = &state.financial_system;

    let estimated_agents = state.agents.banks.len() + state.agents.firms.len() + state.agents.consumers.len() + 2;
    let mut entries: Vec<AgentBalanceDigest> = Vec::with_capacity(estimated_agents);
    let mut seen = HashSet::new();

    for agent_id in state.agents.all_agent_ids() {
        if seen.insert(agent_id) {
            entries.push(build_agent_entry(engine, financial_system, &agent_id));
        }
    }

    let government_id = state.financial_system.government.id;
    if seen.insert(government_id) {
        entries.push(build_agent_entry(engine, financial_system, &government_id));
    }

    let central_bank_id = state.financial_system.central_bank.id;
    if seen.insert(central_bank_id) {
        entries.push(build_agent_entry(engine, financial_system, &central_bank_id));
    }

    let roster = entries;

    let catalogue = AgentsCatalogueDigest { roster: roster.clone(), groups: build_agent_groups(&roster) };

    AgentsDigest { catalogue: Some(catalogue) }
}

pub(crate) fn build_agent_groups(entries: &[AgentBalanceDigest]) -> Vec<AgentGroupDigest> {
    let mut grouped: HashMap<String, Vec<Uuid>> = HashMap::new();
    for entry in entries {
        grouped.entry(entry.agent_type.clone()).or_default().push(entry.agent_id);
    }

    let mut groups: Vec<AgentGroupDigest> = grouped
        .into_iter()
        .map(|(agent_type, mut agent_ids)| {
            agent_ids.sort();
            AgentGroupDigest { agent_type, agent_ids }
        })
        .collect();

    groups.sort_by(|a, b| a.agent_type.cmp(&b.agent_type));
    groups
}

pub(crate) fn build_agent_entry(
    engine: &SimulationEngine, financial_system: &FinancialSystem, agent_id: &AgentId,
) -> AgentBalanceDigest {
    let (agent_type, name) = engine.get_agent_info(agent_id);
    let total_assets = financial_system.get_total_assets(agent_id);
    let total_liabilities = financial_system.get_total_liabilities(agent_id);
    let liquidity = financial_system.get_liquid_assets(agent_id);
    let net_worth = total_assets - total_liabilities;
    let balance_sheet = build_balance_sheet(financial_system, agent_id);

    AgentBalanceDigest {
        agent_id: agent_id.0,
        agent_type,
        name: name.unwrap_or_else(|| "N/A".into()),
        total_assets,
        total_liabilities,
        net_worth,
        liquidity,
        balance_sheet,
    }
}

/// Determines if an instrument type should be aggregated
fn should_aggregate_instrument_type(instrument_type: &str) -> bool {
    matches!(
        instrument_type,
        "Cash"
            | "Consumer Loan"
            | "Corporate Loan"
            | "Credit Card"
            | "Trade Credit"
            | "Credit Facility"
    )
}

/// Aggregates balance entries by instrument type and label
fn aggregate_balance_entries(entries: Vec<BalanceEntryDigest>) -> Vec<BalanceEntryDigest> {
    let mut aggregatable: HashMap<(String, String), Vec<BalanceEntryDigest>> = HashMap::new();
    let mut non_aggregatable: Vec<BalanceEntryDigest> = Vec::new();

    // Separate entries into aggregatable and non-aggregatable
    for entry in entries {
        if should_aggregate_instrument_type(&entry.instrument_type) {
            aggregatable
                .entry((entry.instrument_type.clone(), entry.label.clone()))
                .or_default()
                .push(entry);
        } else {
            non_aggregatable.push(entry);
        }
    }

    // Build aggregated entries
    let mut result = Vec::with_capacity(aggregatable.len() + non_aggregatable.len());

    for ((instrument_type, label), group) in aggregatable {
        if group.len() == 1 {
            // Don't aggregate single entries
            result.push(group.into_iter().next().unwrap());
        } else {
            // Aggregate multiple entries
            let count = group.len();
            let quantity: f64 = group.iter().map(|e| e.quantity).sum();
            let mark_to_market_value: f64 = group.iter().map(|e| e.mark_to_market_value).sum();
            let book_value: f64 = group.iter().map(|e| e.book_value).sum();
            let cost_basis: f64 = group.iter().map(|e| e.cost_basis).sum();
            let source = group.first().map(|e| e.source.clone()).unwrap_or_default();

            // Create aggregated entry with a synthetic ID
            result.push(BalanceEntryDigest {
                instrument_id: format!("aggregated_{}_{}", instrument_type, count),
                instrument_type,
                label: format!("{} (×{})", label, count),
                quantity,
                mark_to_market_value,
                book_value,
                cost_basis,
                source,
            });
        }
    }

    // Add non-aggregatable entries
    result.extend(non_aggregatable);

    result
}

pub(crate) fn build_balance_sheet(financial_system: &FinancialSystem, agent_id: &AgentId) -> BalanceSheetDigest {
    let mut assets: Vec<BalanceEntryDigest> = Vec::new();
    let mut liabilities: Vec<BalanceEntryDigest> = Vec::new();

    if let Some(sheet) = financial_system.balance_sheets.get(agent_id) {
        for (instrument_id, position) in &sheet.assets {
            assets.push(balance_entry_from_position(
                financial_system,
                instrument_id,
                position,
                BalanceEntrySource::BalanceSheet,
            ));
        }

        for (instrument_id, position) in &sheet.liabilities {
            liabilities.push(balance_entry_from_position(
                financial_system,
                instrument_id,
                position,
                BalanceEntrySource::BalanceSheet,
            ));
        }
    }

    for (instrument_id, quantity) in financial_system.clearing_house.csd.get_all_positions(agent_id) {
        if quantity <= 1e-9 {
            continue;
        }
        assets.push(balance_entry_from_custody(financial_system, &instrument_id, quantity));
    }

    // Aggregate entries before sorting
    assets = aggregate_balance_entries(assets);
    liabilities = aggregate_balance_entries(liabilities);

    assets.sort_by(|a, b| {
        b.mark_to_market_value.partial_cmp(&a.mark_to_market_value).unwrap_or(std::cmp::Ordering::Equal)
    });
    liabilities.sort_by(|a, b| b.book_value.partial_cmp(&a.book_value).unwrap_or(std::cmp::Ordering::Equal));

    BalanceSheetDigest { assets, liabilities, income_statement: None }
}

fn balance_entry_from_position(
    financial_system: &FinancialSystem, instrument_id: &InstrumentId, position: &Position, source: BalanceEntrySource,
) -> BalanceEntryDigest {
    let (label, instrument_type) = instrument_metadata(financial_system, instrument_id);
    let unit_price = resolve_unit_price(financial_system, instrument_id, position.book_value_per_unit);
    let mark_to_market_value = unit_price.to_f64() * position.quantity;
    let book_value = position.book_value_per_unit.to_f64() * position.quantity;
    let cost_basis = position.cost_basis_per_unit.to_f64() * position.quantity;

    BalanceEntryDigest {
        instrument_id: instrument_id.to_string(),
        instrument_type,
        label,
        quantity: position.quantity,
        mark_to_market_value: sanitize_f64(mark_to_market_value),
        book_value: sanitize_f64(book_value),
        cost_basis: sanitize_f64(cost_basis),
        source,
    }
}

fn balance_entry_from_custody(
    financial_system: &FinancialSystem, instrument_id: &InstrumentId, quantity: f64,
) -> BalanceEntryDigest {
    let (label, instrument_type) = instrument_metadata(financial_system, instrument_id);
    let fallback = financial_system
        .instruments
        .instruments
        .get(instrument_id)
        .and_then(|inst| inst.unit_par_value())
        .unwrap_or(Money::ONE);
    let unit_price = resolve_unit_price(financial_system, instrument_id, fallback);
    let mtm = unit_price.to_f64() * quantity;
    let fallback_value = fallback.to_f64() * quantity;

    BalanceEntryDigest {
        instrument_id: instrument_id.to_string(),
        instrument_type,
        label,
        quantity,
        mark_to_market_value: sanitize_f64(mtm),
        book_value: sanitize_f64(fallback_value),
        cost_basis: sanitize_f64(fallback_value),
        source: BalanceEntrySource::Custody,
    }
}

fn resolve_unit_price(financial_system: &FinancialSystem, instrument_id: &InstrumentId, fallback: Money) -> Money {
    financial_system
        .get_market_price(instrument_id)
        .or_else(|| financial_system.instruments.instruments.get(instrument_id).and_then(|inst| inst.unit_par_value()))
        .unwrap_or(fallback)
}

pub(crate) fn instrument_metadata(
    financial_system: &FinancialSystem, instrument_id: &InstrumentId,
) -> (String, String) {
    match financial_system.instruments.instruments.get(instrument_id) {
        Some(inst) => {
            let instrument_type = inst.type_as_string().to_string();
            let label = instrument_label(inst, financial_system);
            (label, instrument_type)
        }
        None => ("Unknown Instrument".into(), "unknown".into()),
    }
}

pub(crate) fn instrument_label(inst: &Instrument, financial_system: &FinancialSystem) -> String {
    match inst.state() {
        InstrumentRuntime::Cash(cash) => cash_label(cash),
        InstrumentRuntime::Bond(bond) => bond_label(bond),
        InstrumentRuntime::Credit(credit) => credit_label(credit),
        InstrumentRuntime::Equity(equity) => equity_label(equity),
        InstrumentRuntime::Structured(tranche) => structured_label(tranche),
        InstrumentRuntime::Derivative(derivative) => derivative_label(derivative, financial_system),
        InstrumentRuntime::Repo(repo) => repo_label(repo),
        InstrumentRuntime::RealAsset(asset) => real_asset_label(asset),
    }
}

fn cash_label(cash: &CashState) -> String {
    let mut parts = vec![currency_code(cash.currency).to_string(), cash_type_label(cash.cash_type).to_string()];

    if let Some(rate) = format_rate_bps(cash.interest_bps) {
        parts.push(format!("@ {rate}"));
    }

    parts.join(" ")
}

fn bond_label(bond: &BondState) -> String {
    let mut parts = vec![bond_type_prefix(bond.bond_type()).to_string()];

    if let Some(tenor) = format_tenor(bond.issue_date, bond.maturity_date) {
        parts.push(tenor);
    } else {
        parts.push(format!("Mat {}", bond.maturity_date.format("%Y-%m-%d")));
    }

    parts.join("")
}

fn credit_label(credit: &CreditState) -> String {
    match credit {
        CreditState::Loan(loan) => corporate_loan_label(loan),
        CreditState::ConsumerLoan { category, loan } => consumer_loan_label(*category, loan),
        CreditState::ConsumerCreditCard(facility) => credit_card_label(facility),
        CreditState::TradeCredit(trade) => trade_credit_label(trade),
        CreditState::Facility(facility) => credit_facility_label(facility),
    }
}

fn corporate_loan_label(loan: &LoanState) -> String {
    let mut parts = Vec::new();

    if let Some(term) = loan_term_label(loan) {
        parts.push(term);
    }

    parts.push(rate_structure_label(&loan.archetype.rate_structure));
    parts.push(loan_type_label_name(loan.loan_type).to_string());

    parts.join(" ")
}

fn consumer_loan_label(category: ConsumerLoanCategory, loan: &LoanState) -> String {
    let mut parts = Vec::new();

    if let Some(term) = loan_term_label(loan) {
        parts.push(term);
    }

    parts.push(rate_structure_label(&loan.archetype.rate_structure));
    parts.push(category.as_str().to_string());

    parts.join(" | ")
}

fn credit_card_label(facility: &CreditFacilityState) -> String {
    if let Some(limit) = format_money_compact(facility.commitment_amount) {
        format!("Credit Card {limit}")
    } else {
        "Credit Card".to_string()
    }
}

fn credit_facility_label(facility: &CreditFacilityState) -> String {
    let mut parts = vec![format!("{} Facility", facility_type_label_name(facility.facility_type))];

    if let Some(limit) = format_money_compact(facility.commitment_amount) {
        parts.push(limit);
    }

    if let Some(term) = format_tenor(facility.commitment_date, facility.expiry_date) {
        parts.push(term);
    }

    parts.join(" ")
}

fn trade_credit_label(trade: &TradeCreditState) -> String {
    let mut label = "Trade Credit".to_string();

    if trade.payment_terms.net_days > 0 {
        label.push_str(&format!(" NET{}", trade.payment_terms.net_days));
    }

    label
}

fn equity_label(equity: &EquityState) -> String {
    let class = match &equity.profile.share_class {
        EquityClass::Common => "Common Equity".to_string(),
        EquityClass::Preferred => "Preferred Equity".to_string(),
        EquityClass::Restricted => "Restricted Equity".to_string(),
        EquityClass::Treasury => "Treasury Shares".to_string(),
        EquityClass::DepositaryReceipt => "Depositary Receipt".to_string(),
        EquityClass::Custom(name) => name.clone(),
    };

    if let Some(par) = equity.profile.par_value.and_then(format_money_compact) {
        format!("{class} ({par} par)")
    } else {
        class
    }
}

fn structured_label(tranche: &StructuredTrancheState) -> String {
    let mut base = tranche.tranche_label.clone().unwrap_or_else(|| tranche.tranche_type.label().to_string());

    base.push_str(&format!(" {}", credit_rating_label(tranche.rating)));
    if let Some(rate) = format_rate_bps(tranche.coupon_rate_bps) {
        base.push_str(&format!(" @ {rate}"));
    }

    base.trim().to_string()
}

fn derivative_label(derivative: &DerivativeState, financial_system: &FinancialSystem) -> String {
    let mut parts = Vec::new();

    let contract = match &derivative.contract {
        DerivativeContract::Option(option) => format!("{} Option", option_style_label(option.style)),
        DerivativeContract::Future(_) => "Future".to_string(),
        DerivativeContract::Custom { description } => description.clone(),
    };
    parts.push(contract);

    if let Some(expiry) = derivative.expiry_date {
        parts.push(format!("exp {}", expiry.format("%Y-%m-%d")));
    }

    let underlying = format_underlying(&derivative.underlying, financial_system);
    if !underlying.is_empty() {
        parts.push(format!("on {underlying}"));
    }

    parts.join(" ")
}

fn repo_label(repo: &RepoState) -> String {
    if repo.open_term {
        let mut label = "Open Repo".to_string();
        if let Some(rate) = format_rate_bps(repo.interest_bps) {
            label.push_str(&format!(" @ {rate}"));
        }
        return label;
    }

    let mut parts = vec!["Repo".to_string()];

    if let Some(term) = format_duration_days(repo.start_date, repo.end_date) {
        parts.push(term);
    }

    if let Some(rate) = format_rate_bps(repo.interest_bps) {
        parts.push(format!("@ {rate}"));
    }

    parts.join(" ")
}

fn real_asset_label(asset: &RealAssetState) -> String {
    match asset {
        RealAssetState::Inventory { goods, .. } => format!("Inventory ({} items)", goods.len()),
        RealAssetState::Property { address, .. } => format!("Property {address}"),
        RealAssetState::Custom { description, .. } => description.clone(),
    }
}

pub(crate) fn format_underlying(underlying: &UnderlyingAsset, financial_system: &FinancialSystem) -> String {
    match underlying {
        UnderlyingAsset::Instrument(id) => financial_system
            .instruments
            .instruments
            .get(id)
            .map(|inst| instrument_label(inst, financial_system))
            .unwrap_or_else(|| format!("Instrument {}", id.0)),
        UnderlyingAsset::Good(good_id) => format!("Good {}", good_id.0),
        UnderlyingAsset::Index(index) => index.clone(),
    }
}

fn option_style_label(style: OptionStyle) -> &'static str {
    match style {
        OptionStyle::Call => "Call",
        OptionStyle::Put => "Put",
    }
}

fn bond_type_prefix(bond_type: BondType) -> &'static str {
    match bond_type {
        BondType::Government => "US",
        BondType::Corporate => "Corp",
        BondType::InterbankLoan => "Interbank",
        BondType::Municipal => "Muni",
        BondType::Agency => "Agency",
        BondType::Supranational => "Supra",
    }
}

fn cash_type_label(cash_type: CashType) -> &'static str {
    match cash_type {
        CashType::DemandDeposit => "Demand Deposit",
        CashType::SavingsDeposit => "Savings Deposit",
        CashType::TimeDeposit => "Time Deposit",
        CashType::Currency => "Currency",
        CashType::CentralBankReserves => "Central Bank Reserves",
        CashType::VaultCash => "Vault Cash",
        CashType::TreasuryGeneralAccount => "Treasury General Account",
    }
}

fn loan_type_label_name(loan_type: LoanType) -> &'static str {
    match loan_type {
        LoanType::TermLoan => "Term Loan",
        LoanType::WorkingCapital => "Working Capital Loan",
        LoanType::BridgeLoan => "Bridge Loan",
        LoanType::MortgageLoan => "Mortgage Loan",
        LoanType::ProjectFinance => "Project Finance Loan",
        LoanType::AssetFinance => "Asset Finance Loan",
        LoanType::PersonalLoan => "Personal Loan",
    }
}

fn facility_type_label_name(facility_type: FacilityType) -> &'static str {
    match facility_type {
        FacilityType::Revolver => "Revolver",
        FacilityType::TermLoanFacility => "Term Loan",
        FacilityType::Overdraft => "Overdraft",
        FacilityType::LetterOfCredit => "Letter of Credit",
        FacilityType::MultiCurrency => "Multi-Currency",
    }
}

fn rate_structure_label(rate: &RateStructure) -> String {
    match rate.base_rate {
        RateIndex::Fixed => {
            format!("Fixed {}", format_rate_bps(rate.spread_bps).unwrap_or_else(|| "".to_string()).trim())
        }
        other => {
            let spread = format_rate_bps(rate.spread_bps).map(|s| format!(" + {s}")).unwrap_or_default();
            format!("Floating {}{}", rate_index_code(other), spread)
        }
    }
    .trim()
    .to_string()
}

fn rate_index_code(rate: RateIndex) -> &'static str {
    match rate {
        RateIndex::RBACashRate => "RBA Cash",
        RateIndex::BBSW1M => "BBSW 1M",
        RateIndex::BBSW3M => "BBSW 3M",
        RateIndex::BBSW6M => "BBSW 6M",
        RateIndex::SOFR => "SOFR",
        RateIndex::Fixed => "Fixed",
    }
}

fn loan_term_label(loan: &LoanState) -> Option<String> {
    let months = loan.archetype.repayment_schedule.term_months;
    if let Some(label) = months_to_term_label(months) {
        return Some(label);
    }

    format_tenor(loan.origination_date, loan.maturity_date)
}

fn months_to_term_label(months: u32) -> Option<String> {
    if months == 0 {
        return None;
    }

    if months % 12 == 0 {
        let years = months / 12;
        if years == 0 {
            return None;
        }
        Some(format!("{}Y", years))
    } else if months > 12 {
        let years = months as f64 / 12.0;
        Some(format!("{years:.1}Y"))
    } else {
        Some(format!("{}M", months))
    }
}

pub(crate) fn format_rate_bps(bps: BasisPoints) -> Option<String> {
    let value = bps.to_f64().unwrap_or_default();
    if value.abs() < 1e-6 {
        return None;
    }
    Some(format!("{:.2}%", value / 100.0))
}

pub(crate) fn format_money_compact(amount: Money) -> Option<String> {
    let value = amount.to_f64();
    if !value.is_finite() {
        return None;
    }

    let abs = value.abs();
    if abs < 1.0 {
        return None;
    }

    let (scaled, suffix) = if abs >= 1_000_000_000.0 {
        (value / 1_000_000_000.0, "B")
    } else if abs >= 1_000_000.0 {
        (value / 1_000_000.0, "M")
    } else if abs >= 1_000.0 {
        (value / 1_000.0, "K")
    } else {
        (value, "")
    };

    Some(format!("${:.1}{}", scaled, suffix))
}

pub(crate) fn format_duration_days(start: NaiveDate, end: NaiveDate) -> Option<String> {
    if end <= start {
        return None;
    }
    Some(format!("{}D", (end - start).num_days()))
}

pub(crate) fn format_tenor(issue: NaiveDate, maturity: NaiveDate) -> Option<String> {
    if maturity <= issue {
        return None;
    }

    let days = (maturity - issue).num_days();
    if days <= 0 {
        return None;
    }

    let years = days as f64 / 365.25;
    if years >= 1.0 {
        let rounded_years = (years * 10.0).round() / 10.0;
        let almost_integer = (rounded_years - rounded_years.round()).abs() < 1e-6;
        if almost_integer {
            Some(format!("{}Y", rounded_years.round() as i64))
        } else {
            Some(format!("{rounded_years:.1}Y"))
        }
    } else {
        let months = ((days as f64) / 30.4375).round().max(1.0);
        Some(format!("{}M", months as i64))
    }
}

pub(crate) fn currency_code(currency: Currency) -> &'static str {
    match currency {
        Currency::USD => "USD",
        Currency::AUD => "AUD",
        Currency::EUR => "EUR",
        Currency::JPY => "JPY",
        Currency::GBP => "GBP",
    }
}

pub(crate) fn credit_rating_label(rating: CreditRating) -> String {
    match rating {
        CreditRating::Government(inner) | CreditRating::Corporate(inner) => format!("{:?}", inner),
        CreditRating::Consumer(inner) => format!("{:?}", inner),
    }
}

pub(crate) fn credit_rating_label_opt(rating: Option<CreditRating>) -> Option<String> {
    rating.map(credit_rating_label)
}


