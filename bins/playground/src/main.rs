use chrono::NaiveDate;
use sim_core::prelude::*;
use uuid::Uuid;

fn main() {
    tracing_subscriber::fmt::init();
    let mut catalog = InstrumentCatalog::new();

    let bond_template = make_template();
    let template_id = catalog.instrument_registry.register_template(bond_template).unwrap();
    let series_id = make_bond(&mut catalog, template_id);
    let _lot_id = make_lot(&mut catalog, series_id);

    tracing::info!("Registry {:#?}", catalog);
}

fn make_template() -> InstrumentTemplate {
    InstrumentTemplate {
        id: TemplateId(Uuid::new_v4()),
        product_family: ProductFamily::FixedIncome,
        template_type: TemplateType::BondTemplate {
            bond_class: BondClass::Corporate,
            cash_flow_type: CashFlow::Zero,
            day_count: DayCount::Act360,
        },
        market_classification: MarketClassification {
            primary_market: InstrumentMarket::MoneyMarket(MoneyMarketSegment::CorporateShortTerm),
            default_venue_type: Some(VenueType::CentralLimitOrderBook),
            is_exchange_tradeable: true,
            requires_csd_custody: true,
        },
        lifecycle_rules: LifecycleRules {
            requires_authorization: true,
            supports_partial_redemption: false,
            accrual_method: None,
            settlement_lag_days: 1,
        },
        created_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
    }
}

fn make_bond(catalog: &mut InstrumentCatalog, template_id: TemplateId) -> SeriesId {
    let issuer = AgentId(Uuid::new_v4());
    let issue_date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let maturity_date = NaiveDate::from_ymd_opt(2029, 6, 1).unwrap();

    let bond_details = Instrument::bond(
        InstrumentId(Uuid::new_v4()),
        issuer,
        BondType::Corporate,
        Money::from_f64(1_000.0).unwrap(),
        issue_date,
        maturity_date,
    )
    .coupon_bps(BasisPoints::new(550, 1)) // etc
    .build()
    .unwrap()
    .instrument_type
    .as_bond()
    .unwrap()
    .clone();

    let x = catalog.instrument_registry.find_or_create_series_for_bond(template_id, issuer, &bond_details);
    x.unwrap()
}

fn make_lot(catalog: &mut InstrumentCatalog, series_id: SeriesId) -> InstrumentId {
    let lot_id = catalog.instrument_registry.mint_lot(
        series_id,
        LotType::Fungible { lot_size: 1_000.0 },
        LotQuantity::Units(5_000.0),
    );
    lot_id.unwrap()
}
