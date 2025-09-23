use super::dto::*;
use chrono::NaiveDate;
use engine_v3::SimulationEngine;
use http::StatusCode;
use parking_lot::{RwLock, RwLockReadGuard};
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use sim_core::prelude::*;
use sim_core::types::instrument::{
    InstrumentIdentifiers, Listability, MarketProfile,
    archetypes::{ConsumerCreditRating, CreditRating, SpCreditRating},
};
use sim_core::types::markets::market::{ListingKey, TenorBucket, listing_key_from_instrument};
use std::sync::Arc;
use std::{cmp::Ordering, collections::HashMap};
use uuid::Uuid;

pub struct QueryService {
    engine: Arc<RwLock<SimulationEngine>>,
}

impl Clone for QueryService {
    fn clone(&self) -> Self {
        Self { engine: self.engine.clone() }
    }
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

fn safe_book_value(position: &PopulatedPositionDto) -> f64 {
    let quantity = position.position.quantity;

    if let Some(price) = position.market_price.as_ref() {
        return price.to_f64() * quantity;
    }

    if let Some(unit_value) = position.instrument.unit_par_value() {
        return unit_value.to_f64() * quantity;
    }

    let per_unit = position.position.book_value_per_unit.to_f64();
    if per_unit.is_finite() { quantity * per_unit } else { quantity }
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

fn consumer_loan_label(category: ConsumerLoanCategory) -> &'static str {
    match category {
        ConsumerLoanCategory::ResidentialMortgage => "Residential Mortgages",
        ConsumerLoanCategory::AutoLoan => "Auto Loans",
        ConsumerLoanCategory::PersonalLoan => "Consumer Personal Loans",
        ConsumerLoanCategory::StudentLoan => "Student Loans",
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

fn cash_bucket_label(cash_type: CashType) -> Option<&'static str> {
    match cash_type {
        CashType::DemandDeposit => Some("Demand Deposits"),
        CashType::SavingsDeposit => Some("Savings Deposits"),
        CashType::TimeDeposit => Some("Time Deposits"),
        _ => None,
    }
}

fn tenor_bucket_label(bucket: &TenorBucket) -> &'static str {
    match bucket {
        TenorBucket::LT1Y => "<1Y",
        TenorBucket::Y1_3 => "1-3Y",
        TenorBucket::Y3_5 => "3-5Y",
        TenorBucket::Y5_7 => "5-7Y",
        TenorBucket::Y7_10 => "7-10Y",
        TenorBucket::GT10 => ">10Y",
    }
}

fn sp_rating_label(rating: SpCreditRating) -> &'static str {
    match rating {
        SpCreditRating::AAA => "AAA",
        SpCreditRating::AA => "AA",
        SpCreditRating::A => "A",
        SpCreditRating::BBB => "BBB",
        SpCreditRating::BB => "BB",
        SpCreditRating::B => "B",
        SpCreditRating::CCC => "CCC",
    }
}

fn consumer_rating_label(rating: ConsumerCreditRating) -> &'static str {
    match rating {
        ConsumerCreditRating::Prime => "Prime",
        ConsumerCreditRating::NearPrime => "Near-Prime",
        ConsumerCreditRating::Subprime => "Subprime",
        ConsumerCreditRating::DeepSubprime => "Deep Subprime",
    }
}

fn credit_rating_symbol(rating: CreditRating) -> String {
    match rating {
        CreditRating::Government(sp) | CreditRating::Corporate(sp) => sp_rating_label(sp).to_string(),
        CreditRating::Consumer(consumer) => consumer_rating_label(consumer).to_string(),
    }
}

fn listing_asset_label(key: &ListingKey) -> Option<String> {
    match key {
        ListingKey::Cash { .. } => None,
        ListingKey::CreditLoan { loan_type } => Some(format!("{} Loan Book", loan_type_label(*loan_type))),
        ListingKey::ConsumerLoan { category } => Some(format!("{} Portfolio", consumer_loan_label(*category))),
        ListingKey::CreditFacility { facility_type } => {
            Some(format!("{} Portfolio", facility_type_label(*facility_type)))
        }
        ListingKey::CreditCard => Some("Consumer Credit Cards".to_string()),
        ListingKey::TradeCredit => Some("Trade Credit Assets".to_string()),
        ListingKey::GovBond { tenor_years } => Some(format!("Government Bonds {}Y", tenor_years)),
        ListingKey::CorpBond { rating, tenor_bucket } => {
            Some(format!("Corporate Bonds {} {}", credit_rating_symbol(*rating), tenor_bucket_label(tenor_bucket)))
        }
        ListingKey::StructuredProduct { rating, tranche_type } => {
            Some(format!("Structured {} {}", credit_rating_symbol(*rating), tranche_type.label()))
        }
        ListingKey::Equity { .. } => Some("Equity Holdings".to_string()),
        ListingKey::Derivative { .. } => Some("Derivatives".to_string()),
        ListingKey::RealAsset => Some("Real Assets".to_string()),
        ListingKey::Repo => Some("Repo Financing".to_string()),
    }
}

fn listing_liability_label(key: &ListingKey) -> Option<String> {
    match key {
        ListingKey::Cash { cash_type } => cash_bucket_label(*cash_type).map(|s| s.to_string()),
        ListingKey::CreditLoan { loan_type } => Some(loan_type_label(*loan_type).to_string()),
        ListingKey::CreditFacility { facility_type } => Some(facility_type_label(*facility_type).to_string()),
        ListingKey::TradeCredit => Some("Trade Payables".to_string()),
        ListingKey::GovBond { tenor_years } => Some(format!("Government Bonds {}Y", tenor_years)),
        ListingKey::CorpBond { rating, tenor_bucket } => {
            Some(format!("Corporate Bonds {} {}", credit_rating_symbol(*rating), tenor_bucket_label(tenor_bucket)))
        }
        ListingKey::StructuredProduct { rating, tranche_type } => {
            Some(format!("Structured {} {}", credit_rating_symbol(*rating), tranche_type.label()))
        }
        ListingKey::Derivative { .. } => Some("Derivative Exposures".to_string()),
        ListingKey::Repo => Some("Repo Obligations".to_string()),
        ListingKey::ConsumerLoan { .. }
        | ListingKey::CreditCard
        | ListingKey::Equity { .. }
        | ListingKey::RealAsset => None,
    }
}

struct AggregationDescriptor {
    label: String,
    rate_bps: Option<f64>,
    original_term_days: Option<f64>,
    remaining_term_days: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_labels_match_expected_categories() {
        let term_loan_key = ListingKey::CreditLoan { loan_type: LoanType::TermLoan };
        assert_eq!(listing_asset_label(&term_loan_key).as_deref(), Some("Term Loans Loan Book"));
        assert_eq!(listing_liability_label(&term_loan_key).as_deref(), Some("Term Loans"));

        let cash_key = ListingKey::Cash { cash_type: CashType::DemandDeposit };
        assert!(listing_asset_label(&cash_key).is_none());
        assert_eq!(listing_liability_label(&cash_key).as_deref(), Some("Demand Deposits"));

        let corp_bond_key =
            ListingKey::CorpBond { rating: CreditRating::corporate_bbb(), tenor_bucket: TenorBucket::Y3_5 };
        assert!(listing_asset_label(&corp_bond_key).unwrap().contains("Corporate Bonds"));
        assert!(listing_liability_label(&corp_bond_key).unwrap().contains("Corporate Bonds"));
    }

    #[test]
    fn listing_bucket_aggregates_match_raw_totals() {
        fn make_position(inst: &Instrument, quantity: f64) -> PopulatedPositionDto {
            let unit = inst.unit_par_value().expect("bond has par value");
            PopulatedPositionDto {
                position: Position { quantity, book_value_per_unit: unit, cost_basis_per_unit: unit },
                instrument: inst.clone(),
                market_price: None,
            }
        }

        let issuer = AgentId(Uuid::new_v4());
        let issue_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let maturity_date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let face_value = Money::from(1_000_i64);
        let coupon_bps = dec!(500.0);

        let build_corp_bond = |outstanding_units: f64| {
            Instrument::bond(
                InstrumentId(Uuid::new_v4()),
                issuer,
                BondType::Corporate,
                face_value,
                issue_date,
                maturity_date,
            )
            .coupon_bps(coupon_bps)
            .frequency(2)
            .rating(CreditRating::Corporate(SpCreditRating::BBB))
            .outstanding_units(outstanding_units)
            .auto_market()
            .build()
            .expect("corporate bond builds")
        };

        let bond_a = build_corp_bond(150.0);
        let bond_b = build_corp_bond(90.0);
        let current_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

        let asset_positions = vec![make_position(&bond_a, 10.0), make_position(&bond_b, 4.0)];

        let mut asset_groups: HashMap<String, AggregatedAccumulator> = HashMap::new();
        for position in &asset_positions {
            let descriptor = classify_asset_position(position, current_date).expect("should classify asset");
            accumulate_entry(&mut asset_groups, descriptor, position);
        }
        let asset_aggregates: Vec<_> = asset_groups.into_iter().map(|(label, acc)| acc.into_entry(label)).collect();

        assert_eq!(asset_aggregates.len(), 1);
        let asset_entry = &asset_aggregates[0];
        let expected_assets: f64 = asset_positions.iter().map(|pos| safe_book_value(pos)).sum();
        assert_eq!(asset_entry.position_count, asset_positions.len());
        assert!((asset_entry.total_book_value - expected_assets).abs() < 1e-6);
        assert!((asset_entry.total_quantity - 14.0).abs() < 1e-6);
        let avg_rate = asset_entry.average_rate_bps.expect("avg rate");
        assert!((avg_rate - 500.0).abs() < 1e-6);
        assert!(asset_entry.label.contains("Corporate Bonds"));

        let liability_positions = vec![make_position(&bond_a, 6.0), make_position(&bond_b, 2.5)];
        let mut liability_groups: HashMap<String, AggregatedAccumulator> = HashMap::new();
        for position in &liability_positions {
            let descriptor = classify_liability_position(position, current_date).expect("should classify liability");
            accumulate_entry(&mut liability_groups, descriptor, position);
        }
        let liability_aggregates: Vec<_> =
            liability_groups.into_iter().map(|(label, acc)| acc.into_entry(label)).collect();

        assert_eq!(liability_aggregates.len(), 1);
        let liability_entry = &liability_aggregates[0];
        let expected_liabilities: f64 = liability_positions.iter().map(|pos| safe_book_value(pos)).sum();
        assert_eq!(liability_entry.position_count, liability_positions.len());
        assert!((liability_entry.total_book_value - expected_liabilities).abs() < 1e-6);
    }
}

fn accumulate_entry(
    groups: &mut HashMap<String, AggregatedAccumulator>, descriptor: AggregationDescriptor,
    position: &PopulatedPositionDto,
) {
    let AggregationDescriptor { label, rate_bps, original_term_days, remaining_term_days } = descriptor;
    let book_value = safe_book_value(position);
    groups.entry(label).or_default().add(
        position.position.quantity,
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
    I: IntoIterator<Item = &'a PopulatedPositionDto>,
{
    positions.into_iter().map(|pos| safe_book_value(pos)).sum()
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
    let inst = &position.instrument;
    let listing_key = listing_key_from_instrument(inst);
    let label = listing_asset_label(&listing_key)?;
    let runtime = inst.state();

    let (rate_bps, original_term_days, remaining_term_days) = match (&listing_key, runtime) {
        (ListingKey::CreditLoan { .. }, InstrumentRuntime::Credit(CreditState::Loan(loan))) => (
            loan.spread_bps().to_f64(),
            Some(non_negative_days((loan.maturity_date - loan.origination_date).num_days())),
            Some(non_negative_days((loan.maturity_date - current_date).num_days())),
        ),
        (ListingKey::ConsumerLoan { .. }, InstrumentRuntime::Credit(CreditState::ConsumerLoan { loan, .. })) => (
            loan.spread_bps().to_f64(),
            Some(non_negative_days((loan.maturity_date - loan.origination_date).num_days())),
            Some(non_negative_days((loan.maturity_date - current_date).num_days())),
        ),
        (ListingKey::CreditCard, InstrumentRuntime::Credit(CreditState::ConsumerCreditCard(facility))) => (
            facility.spread_bps.to_f64(),
            Some(non_negative_days((facility.expiry_date - facility.commitment_date).num_days())),
            Some(non_negative_days((facility.expiry_date - current_date).num_days())),
        ),
        (ListingKey::CreditFacility { .. }, InstrumentRuntime::Credit(CreditState::Facility(facility))) => (
            facility.spread_bps.to_f64(),
            Some(non_negative_days((facility.expiry_date - facility.commitment_date).num_days())),
            Some(non_negative_days((facility.expiry_date - current_date).num_days())),
        ),
        (ListingKey::TradeCredit, InstrumentRuntime::Credit(CreditState::TradeCredit(details))) => (
            None,
            Some(non_negative_days((details.due_date - details.invoice_date).num_days())),
            Some(non_negative_days((details.due_date - current_date).num_days())),
        ),
        (ListingKey::GovBond { .. }, InstrumentRuntime::Bond(bond))
        | (ListingKey::CorpBond { .. }, InstrumentRuntime::Bond(bond)) => (
            bond.archetype.coupon_rate_bps.to_f64(),
            Some(non_negative_days((bond.maturity_date - bond.issue_date).num_days())),
            Some(non_negative_days((bond.maturity_date - current_date).num_days())),
        ),
        (ListingKey::StructuredProduct { .. }, InstrumentRuntime::Structured(tranche)) => (
            tranche.coupon_rate_bps.to_f64(),
            Some(non_negative_days((tranche.maturity_date - current_date).num_days())),
            Some(non_negative_days((tranche.maturity_date - current_date).num_days())),
        ),
        (ListingKey::Equity { .. }, InstrumentRuntime::Equity(_)) => (None, None, None),
        (ListingKey::RealAsset, InstrumentRuntime::RealAsset(_)) => (None, None, None),
        (ListingKey::Derivative { .. }, InstrumentRuntime::Derivative(derivative)) => {
            let term = derivative.expiry_date.map(|expiry| non_negative_days((expiry - current_date).num_days()));
            (None, term, term)
        }
        (ListingKey::Repo, InstrumentRuntime::Repo(repo)) => (
            repo.interest_bps.to_f64(),
            Some(non_negative_days((repo.end_date - repo.start_date).num_days())),
            Some(non_negative_days((repo.end_date - current_date).num_days())),
        ),
        (ListingKey::Cash { .. }, _) => return None,
        _ => return None,
    };

    Some(AggregationDescriptor { label, rate_bps, original_term_days, remaining_term_days })
}

fn classify_liability_position(
    position: &PopulatedPositionDto, current_date: NaiveDate,
) -> Option<AggregationDescriptor> {
    let inst = &position.instrument;
    let listing_key = listing_key_from_instrument(inst);
    let label = listing_liability_label(&listing_key)?;
    let runtime = inst.state();

    let (rate_bps, original_term_days, remaining_term_days) = match (&listing_key, runtime) {
        (ListingKey::Cash { .. }, InstrumentRuntime::Cash(details)) => {
            if details.cash_type == CashType::TreasuryGeneralAccount {
                return None;
            }
            (details.interest_bps.to_f64(), None, None)
        }
        (ListingKey::CreditLoan { .. }, InstrumentRuntime::Credit(CreditState::Loan(loan))) => (
            loan.spread_bps().to_f64(),
            Some(non_negative_days((loan.maturity_date - loan.origination_date).num_days())),
            Some(non_negative_days((loan.maturity_date - current_date).num_days())),
        ),
        (ListingKey::CreditFacility { .. }, InstrumentRuntime::Credit(CreditState::Facility(facility))) => (
            facility.spread_bps.to_f64(),
            Some(non_negative_days((facility.expiry_date - facility.commitment_date).num_days())),
            Some(non_negative_days((facility.expiry_date - current_date).num_days())),
        ),
        (ListingKey::TradeCredit, InstrumentRuntime::Credit(CreditState::TradeCredit(details))) => (
            None,
            Some(non_negative_days((details.due_date - details.invoice_date).num_days())),
            Some(non_negative_days((details.due_date - current_date).num_days())),
        ),
        (ListingKey::GovBond { .. }, InstrumentRuntime::Bond(bond))
        | (ListingKey::CorpBond { .. }, InstrumentRuntime::Bond(bond)) => (
            bond.archetype.coupon_rate_bps.to_f64(),
            Some(non_negative_days((bond.maturity_date - bond.issue_date).num_days())),
            Some(non_negative_days((bond.maturity_date - current_date).num_days())),
        ),
        (ListingKey::StructuredProduct { .. }, InstrumentRuntime::Structured(tranche)) => (
            tranche.coupon_rate_bps.to_f64(),
            Some(non_negative_days((tranche.maturity_date - current_date).num_days())),
            Some(non_negative_days((tranche.maturity_date - current_date).num_days())),
        ),
        (ListingKey::Derivative { .. }, InstrumentRuntime::Derivative(derivative)) => {
            let term = derivative.expiry_date.map(|expiry| non_negative_days((expiry - current_date).num_days()));
            (None, term, term)
        }
        (ListingKey::Repo, InstrumentRuntime::Repo(repo)) => (
            repo.interest_bps.to_f64(),
            Some(non_negative_days((repo.end_date - repo.start_date).num_days())),
            Some(non_negative_days((repo.end_date - current_date).num_days())),
        ),
        (ListingKey::Cash { .. }, _) => return None,
        _ => (None, None, None),
    };

    Some(AggregationDescriptor { label, rate_bps, original_term_days, remaining_term_days })
}

type QueryResult<T> = Result<T, (StatusCode, String)>;

impl QueryService {
    pub fn new(engine: Arc<RwLock<SimulationEngine>>) -> Self {
        Self { engine }
    }

    fn get_engine_lock(&self) -> Result<RwLockReadGuard<'_, SimulationEngine>, (StatusCode, String)> {
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
                accumulate_entry(&mut asset_groups, descriptor, asset);
            }
        }

        let mut liability_groups: HashMap<String, AggregatedAccumulator> = HashMap::new();
        for liability in liabilities {
            if let Some(descriptor) = classify_liability_position(liability, current_date) {
                accumulate_entry(&mut liability_groups, descriptor, liability);
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

        let total_assets = total_book_value(assets.iter());
        let total_liabilities = total_book_value(liabilities.iter());

        let net_worth = total_assets - total_liabilities;

        let equity = if net_worth.abs() > 1e-2 {
            let unit_money = Money::from(1_i64);
            let position =
                Position { quantity: net_worth, book_value_per_unit: unit_money, cost_basis_per_unit: unit_money };
            let instrument = Instrument::new(
                InstrumentIdentifiers::from(InstrumentId(Uuid::new_v4())),
                MarketProfile::from_market(InstrumentMarket::CapitalMarket(CapitalMarketSegment::Equity)),
                Listability::Unlisted,
                InstrumentRuntime::Equity(EquityState {
                    profile: EquityProfile {
                        issuer: *agent_id,
                        share_class: EquityClass::Common,
                        authorized_shares: Some(1),
                        par_value: Some(unit_money),
                        dividend_policy: None,
                    },
                    outstanding_shares: 1,
                }),
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
            return Err((StatusCode::NOT_FOUND, format!("Agent with ID {} not found", agent_id)));
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
            market_index: MarketIndexDto::from(&engine.state.financial_system.exchange.index),
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
            .ok_or((StatusCode::NOT_FOUND, "Market not found".to_string()))?;

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
            .ok_or((StatusCode::NOT_FOUND, format!("Tick {} not found in history.", tick_number)))?;

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
        Ok(ExchangeDto::from(exchange))
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
