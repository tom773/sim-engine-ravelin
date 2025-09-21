use crate::prelude::*;
use crate::types::instrument::{InstrumentIdentifiers, InstrumentRuntime, Listability, MarketProfile, RealAssetState};
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
        inst.state(),
        InstrumentRuntime::Bond(_)
            | InstrumentRuntime::Equity(_)
            | InstrumentRuntime::Structured(_)
            | InstrumentRuntime::Derivative(_)
    )
}
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialSystem {
    pub instruments: InstrumentCatalog,
    pub instrument_registry: InstrumentRegistry,
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
            instrument_registry: InstrumentRegistry::default(),
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
    pub fn normalise_position_book_values(&mut self) {
        let instruments = &self.instruments.instruments;
        for balance_sheet in self.balance_sheets.values_mut() {
            // Adjust both asset and liability books to reflect per-unit par values.
            for (inst_id, position) in balance_sheet.assets.iter_mut().chain(balance_sheet.liabilities.iter_mut()) {
                if let Some(inst) = instruments.get(inst_id) {
                    if let Some(unit_value) = inst.unit_par_value() {
                        let original_book = position.book_value_per_unit;
                        let original_cost = position.cost_basis_per_unit;

                        if original_book != unit_value {
                            position.book_value_per_unit = unit_value;
                        }

                        if original_cost == original_book || original_cost == unit_value {
                            position.cost_basis_per_unit = unit_value;
                        }
                    }
                }
            }
        }
    }

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
            self.instruments
                .instruments
                .get(inst_id)
                .map(|i| matches!(i.state(), InstrumentRuntime::RealAsset(RealAssetState::Inventory { .. })))
                .unwrap_or(false)
        })?;
        self.instruments.instruments.get_mut(inventory_inst_id)
    }

    pub fn add_to_inventory(&mut self, owner: &AgentId, good_id: &GoodId, qty: f64, unit_cost: Money) {
        let inst_id = self.ensure_inventory_container(*owner);
        if let Some(inst) = self.instruments.instruments.get_mut(&inst_id) {
            if let InstrumentRuntime::RealAsset(RealAssetState::Inventory { goods, .. }) = inst.state_mut() {
                let item = goods.entry(*good_id).or_default();

                let new_qty = item.quantity + qty;
                item.unit_cost = if new_qty > 0.0 {
                    (item.unit_cost * item.quantity + unit_cost * qty) / new_qty
                } else {
                    Money::ZERO
                };
                item.quantity = new_qty;
            }
        }
    }
    pub fn ensure_inventory_container(&mut self, owner: AgentId) -> InstrumentId {
        if let Some(id) = self
            .balance_sheets
            .get(&owner)
            .and_then(|bs| {
                bs.assets.keys().find(|inst_id| {
                    self.instruments
                        .instruments
                        .get(inst_id)
                        .and_then(|i| match &i.state() {
                            InstrumentRuntime::RealAsset(RealAssetState::Inventory { .. }) => Some(()),
                            _ => None,
                        })
                        .is_some()
                })
            })
            .copied()
        {
            return id;
        }

        let inst_id = InstrumentId(uuid::Uuid::new_v4());
        let instrument = Instrument::new(
            InstrumentIdentifiers::from(inst_id),
            MarketProfile::from_market(InstrumentMarket::CapitalMarket(CapitalMarketSegment::StructuredFinance)),
            Listability::Unlisted,
            InstrumentRuntime::RealAsset(RealAssetState::Inventory { owner, goods: HashMap::new() }),
        );
        self.instruments.insert(inst_id, instrument.clone());
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
                        let unit_value =
                            instrument.unit_par_value().or_else(|| instrument.face_value()).unwrap_or(Money::ONE);
                        total_assets += unit_value * quantity;
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
            if let InstrumentRuntime::RealAsset(RealAssetState::Inventory { goods, .. }) = inst.state_mut() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::core_utils::time::BasisPoints;
    use crate::types::instrument::{BondType, Instrument};

    #[test]
    fn normalise_bond_positions_reset_total_notional() {
        let mut system = FinancialSystem::default();
        let issuer = system.government.id;
        let instrument = Instrument::bond(
            InstrumentId(Uuid::new_v4()),
            issuer,
            BondType::Government,
            Money::from(1_000_i64),
            chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(),
        )
        .coupon_bps(BasisPoints::new(250, 1))
        .frequency(2)
        .outstanding_units(2_800.0)
        .build()
        .unwrap();

        let inst_id = instrument.instrument_id();
        system.instruments.insert(inst_id, instrument);

        let mut bs = BalanceSheet::new(issuer);
        bs.liabilities.insert(
            inst_id,
            Position {
                quantity: 2_800.0,
                book_value_per_unit: Money::from(2_800_000_i64),
                cost_basis_per_unit: Money::from(2_800_000_i64),
            },
        );
        system.balance_sheets.insert(issuer, bs);

        system.normalise_position_book_values();

        let pos = &system.balance_sheets[&issuer].liabilities[&inst_id];
        assert_eq!(pos.book_value_per_unit, Money::from(1_000_i64));
        assert_eq!(pos.cost_basis_per_unit, Money::from(1_000_i64));
    }
}
