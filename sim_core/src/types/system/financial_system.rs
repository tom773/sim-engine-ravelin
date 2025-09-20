use crate::prelude::*;
use crate::types::money::Money;
use ordered_float::NotNan;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

pub fn is_security(inst: &Instrument) -> bool {
    matches!(
        inst.instrument_type,
        InstrumentType::Debt(DebtInstrument::Bond(_))
            | InstrumentType::Equity(_)
            | InstrumentType::StructuredTranche(_)
            | InstrumentType::Derivative(_)
    )
}
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialSystem {
    pub instruments: InstrumentCatalog,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub balance_sheets: HashMap<AgentId, BalanceSheet>,
    pub central_bank: CentralBank,
    pub government: Government,
    pub exchange: Exchange,
    pub clearing_house: ClearingHouse,
    pub goods: GoodsRegistry,
    pub credit_registry: CreditRegistry,
    pub rtgs: RtgsQueue,
    pub rtgs_policy: RtgsPolicy,
    #[serde(skip)]
    pub pricing_feeds: PricingFeeds,
    pub funding_markets: OvernightFundingBooks,
}
#[derive(Clone, Default, Debug)]
pub struct GoodsMetrics {
    pub last_avg_wage: f64,
    pub avg_wage: f64,
    pub per_good: std::collections::HashMap<GoodId, GoodMetric>,
}

#[derive(Clone, Default, Debug)]
pub struct GoodMetric {
    pub weighted_unit_cost: f64,
    pub inventory_qty: f64,
    pub avg_daily_sales: f64,
    pub supply_shock: f64,
    pub base_markup: f64,
}

#[derive(Clone, Default, Debug)]
pub struct YieldCurve {
    pub date: chrono::NaiveDate,
    pub points: BTreeMap<NotNan<f64>, f64>,
}

#[derive(Clone, Default, Debug)]
pub struct PricingFeeds {
    pub policy_rate_bps: Arc<RwLock<f64>>,
    pub yield_curve: Arc<RwLock<YieldCurve>>,
    pub goods: Arc<RwLock<GoodsMetrics>>,
    pub current_date: Arc<RwLock<chrono::NaiveDate>>,
}
impl PricingFeeds {
    pub fn with_date(&self, date: chrono::NaiveDate) -> Self {
        PricingFeeds {
            policy_rate_bps: self.policy_rate_bps.clone(),
            yield_curve: self.yield_curve.clone(),
            goods: self.goods.clone(),
            current_date: std::sync::Arc::new(std::sync::RwLock::new(date)),
        }
    }
}
impl Default for FinancialSystem {
    fn default() -> Self {
        let government = Government {
            id: AgentId(Uuid::new_v4()),
            tax_rates: TaxRates::default(),
            spending_targets: SpendingTargets::default(),
            debt_ceiling: Some(Money::from(1_000_000_000 as i64)),
            fiscal_policy: FiscalPolicy::default(),
        };
        let central_bank =
            CentralBank { id: AgentId(Uuid::new_v4()), policy_rate_bps: dec!(425), reserve_requirement: 0.1 };
        let mut balance_sheets = HashMap::new();
        balance_sheets.insert(central_bank.id, BalanceSheet::new(central_bank.id));
        balance_sheets.insert(government.id, BalanceSheet::new(government.id));

        Self {
            instruments: InstrumentCatalog::default(),
            balance_sheets,
            central_bank,
            government,
            exchange: Exchange::default(),
            clearing_house: ClearingHouse::default(),
            goods: GoodsRegistry::default(),
            credit_registry: CreditRegistry::default(),
            rtgs: RtgsQueue::default(),
            rtgs_policy: RtgsPolicy::default(),
            pricing_feeds: PricingFeeds::default(),
            funding_markets: OvernightFundingBooks::default(),
        }
    }
}

impl FinancialSystem {
    pub fn attach_default_pricing_feeds(&mut self, now: chrono::NaiveDate) {
        self.pricing_feeds = PricingFeeds {
            policy_rate_bps: Arc::new(RwLock::new(self.central_bank.policy_rate_bps.to_f64().unwrap_or(69.0))),
            yield_curve: Arc::new(RwLock::new(YieldCurve { date: now, points: BTreeMap::new() })),
            goods: Arc::new(RwLock::new(GoodsMetrics::default())),
            current_date: Arc::new(RwLock::new(now)),
        };
        // Use the instrument registry to re-inject financial pricers as well.
        self.exchange.attach_pricing_feeds_with_registry(self.pricing_feeds.clone(), &self.instruments.instruments);
    }
    fn find_inventory_instrument_mut(&mut self, agent_id: &AgentId) -> Option<&mut Instrument> {
        let bs = self.balance_sheets.get(agent_id)?;
        let inventory_inst_id = bs.assets.keys().find(|inst_id| {
            matches!(
                self.instruments.instruments.get(inst_id).map(|i| &i.instrument_type),
                Some(InstrumentType::RealAsset(RealAssetType::Inventory { .. }))
            )
        })?;
        self.instruments.instruments.get_mut(inventory_inst_id)
    }

    pub fn add_to_inventory(&mut self, owner: &AgentId, good_id: &GoodId, qty: f64, unit_cost: Money) {
        let inst_id = self.ensure_inventory_container(*owner);
        if let Some(Instrument {
            instrument_type: InstrumentType::RealAsset(RealAssetType::Inventory { goods, .. }),
            ..
        }) = self.instruments.instruments.get_mut(&inst_id)
        {
            let item = goods.entry(*good_id).or_default();

            let new_qty = item.quantity + qty;
            item.unit_cost =
                if new_qty > 0.0 { (item.unit_cost * item.quantity + unit_cost * qty) / new_qty } else { Money::ZERO };
            item.quantity = new_qty;
        }
    }
    pub fn ensure_inventory_container(&mut self, owner: AgentId) -> InstrumentId {
        if let Some(id) = self
            .balance_sheets
            .get(&owner)
            .and_then(|bs| {
                bs.assets.keys().find(|inst_id| {
                    matches!(
                        self.instruments.instruments.get(inst_id).map(|i| &i.instrument_type),
                        Some(InstrumentType::RealAsset(RealAssetType::Inventory { .. }))
                    )
                })
            })
            .copied()
        {
            return id;
        }

        let inst_id = InstrumentId(uuid::Uuid::new_v4());
        let inst = Instrument::new(
            inst_id,
            InstrumentType::RealAsset(RealAssetType::Inventory { owner, goods: std::collections::HashMap::new() }),
            InstrumentMarket::CapitalMarket(CapitalMarketSegment::StructuredFinance),
        );
        self.instruments.instruments.insert(inst_id, inst);
        let bs = self.balance_sheets.entry(owner).or_insert_with(|| BalanceSheet::new(owner));
        bs.assets.insert(
            inst_id,
            Position { quantity: 1.0, book_value_per_unit: Money::ZERO, cost_basis_per_unit: Money::ZERO },
        );
        inst_id
    }
    pub fn validate_accounting_identity(&self) -> Result<(), String> {
        let mut total_assets = Money::ZERO;
        let mut total_liabilities = Money::ZERO;

        for bs in self.balance_sheets.values() {
            for position in bs.assets.values() {
                total_assets += position.book_value_per_unit * position.quantity;
            }
            for position in bs.liabilities.values() {
                total_liabilities += position.book_value_per_unit * position.quantity;
            }
        }

        for account in self.clearing_house.csd.custody_accounts.values() {
            for (inst_id, holding) in &account.holdings {
                if let Some(instrument) = self.instruments.instruments.get(inst_id) {
                    if is_security(instrument) {
                        let quantity = holding.total_position();
                        let book_value = instrument.face_value().unwrap_or(Money::ZERO);
                        total_assets += book_value * quantity;
                    }
                }
            }
        }

        let discrepancy = total_assets - total_liabilities;
        const TOLERANCE: f64 = 1.0;

        if discrepancy.to_f64().abs() > TOLERANCE {
            Err(format!(
                "Accounting identity violated! Total Assets = {:.2}, Total Liabilities = {:.2}, Discrepancy = {:.2}",
                total_assets, total_liabilities, discrepancy
            ))
        } else {
            Ok(())
        }
    }
    pub fn remove_from_inventory(&mut self, agent_id: &AgentId, good_id: &GoodId, quantity: f64) -> Result<(), String> {
        if let Some(inst) = self.find_inventory_instrument_mut(agent_id) {
            if let InstrumentType::RealAsset(RealAssetType::Inventory { goods, .. }) = &mut inst.instrument_type {
                if let Some(item) = goods.get_mut(good_id) {
                    if item.quantity >= quantity {
                        item.quantity -= quantity;
                        return Ok(());
                    } else {
                        return Err(format!(
                            "Insufficient inventory for good {:?}: have {}, need {}",
                            good_id.0, item.quantity, quantity
                        ));
                    }
                }
            }
        }
        Err(format!("No inventory for good {:?} found for agent {:?}", good_id.0, agent_id.0))
    }
}
