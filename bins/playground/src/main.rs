use chrono::NaiveDate;
use sim_core::prelude::*;
use uuid::Uuid;

fn main() {
    tracing_subscriber::fmt::init();

    let mut catalog = InstrumentCatalog::new();
    let mut registry = InstrumentRegistry::new();

    let bond_template = make_template();
    let template_id = registry.register_template(bond_template).unwrap();

    let issuer = AgentId(Uuid::new_v4());
    let spec = BondIssuanceSpec {
        template_id,
        bond_type: BondType::Corporate,
        face_value: Money::from_f64(1_000.0).unwrap(),
        coupon_rate_bps: BasisPoints::new(550, 1),
        issue_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        maturity_date: NaiveDate::from_ymd_opt(2029, 6, 1).unwrap(),
        frequency_per_year: 2,
        rating: CreditRating::Corporate(SpCreditRating::BBB),
        units: 5_000.0,
    };

    let _issued = issue_corporate_bond(&mut registry, &mut catalog, issuer, spec).expect("bond issuance");

    tracing::info!("Issued bond instrument {:#?}", catalog);
}

fn make_template() -> InstrumentTemplate {
    InstrumentTemplate {
        id: TemplateId(Uuid::new_v4()),
        product_family: ProductFamily::FixedIncome,
        archetype: InstrumentArchetype::Bond(BondArchetype {
            bond_type: BondType::Corporate,
            cash_flow_type: CashFlow::Fixed,
            day_count: DayCount::Act360,
            face_value: Money::from(1_000_i64),
            coupon_rate_bps: BasisPoints::new(500, 1),
            frequency_per_year: 2,
            rating: Some(CreditRating::Corporate(SpCreditRating::BBB)),
            covenants: Vec::new(),
        }),
        market_profile: MarketProfile::from_market(InstrumentMarket::MoneyMarket(
            MoneyMarketSegment::CorporateShortTerm,
        )),
        lifecycle_rules: LifecycleRules {
            requires_authorization: true,
            supports_partial_redemption: false,
            accrual_method: None,
            settlement_lag_days: 1,
        },
        created_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
    }
}
