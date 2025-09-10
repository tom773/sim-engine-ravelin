use crate::prelude::*;
use crate::types::money::Money;
use chrono::NaiveDate;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::HashMap;
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
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub instruments: HashMap<InstrumentId, Instrument>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub balance_sheets: HashMap<AgentId, BalanceSheet>,
    pub central_bank: CentralBank,
    pub government: Government,
    pub exchange: Exchange,
    pub clearing_house: ClearingHouse,
    pub yield_curve: YieldCurve,
    pub goods: GoodsRegistry,
    pub credit_registry: CreditRegistry,
    pub rtgs: RtgsQueue,
    pub rtgs_policy: RtgsPolicy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct YieldCurve {
    pub date: chrono::NaiveDate,
    pub yields: HashMap<u16, f64>,
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
            instruments: HashMap::new(),
            balance_sheets,
            central_bank,
            government,
            exchange: Exchange::default(),
            clearing_house: ClearingHouse::default(),
            yield_curve: YieldCurve { date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), yields: HashMap::new() },
            goods: GoodsRegistry::default(),
            credit_registry: CreditRegistry::default(),
            rtgs: RtgsQueue::default(),
            rtgs_policy: RtgsPolicy::default(),
        }
    }
}

impl FinancialSystem {
    pub fn update_yield_curve(&mut self, date: NaiveDate) {
        let mut yields = HashMap::new();

        for bond_type_instruments in &self.exchange.index.by_bond_type {
            if let (BondType::Government, instrument_ids) = bond_type_instruments {
                for inst_id in instrument_ids {
                    if let Some(instrument) = self.instruments.get(inst_id) {
                        if let InstrumentType::Debt(DebtInstrument::Bond(bond_details)) = &instrument.instrument_type {
                            let years = ((bond_details.maturity_date - bond_details.issue_date).num_days() as f64
                                / 365.0)
                                .round() as u16;

                            if let Some(order_book) = self.exchange.financial_market(inst_id) {
                                if let Some(mid_price) = order_book.mid_price() {
                                    let mid_price_f64 = mid_price.to_f64();
                                    let face_value_f64 = bond_details.face_value.to_f64();

                                    let yield_estimate = if mid_price_f64 > 0.0 {
                                        (face_value_f64 / mid_price_f64 - 1.0) / (years as f64)
                                    } else {
                                        bond_details.coupon_rate_bps.to_f64().unwrap_or(0.0) / 10000.0
                                    };

                                    yields.insert(years, yield_estimate);
                                }
                            }
                        }
                    }
                }
            }
        }

        self.yield_curve = YieldCurve { date, yields };
    }

    fn find_inventory_instrument_mut(&mut self, agent_id: &AgentId) -> Option<&mut Instrument> {
        let bs = self.balance_sheets.get(agent_id)?;
        let inventory_inst_id = bs.assets.keys().find(|inst_id| {
            matches!(
                self.instruments.get(inst_id).map(|i| &i.instrument_type),
                Some(InstrumentType::RealAsset(RealAssetType::Inventory { .. }))
            )
        })?;
        self.instruments.get_mut(inventory_inst_id)
    }

    pub fn add_to_inventory(&mut self, owner: &AgentId, good_id: &GoodId, qty: f64, unit_cost: Money) {
        let inst_id = self.ensure_inventory_container(*owner);
        if let Some(Instrument {
            instrument_type: InstrumentType::RealAsset(RealAssetType::Inventory { goods, .. }),
            ..
        }) = self.instruments.get_mut(&inst_id)
        {
            let item = goods.entry(*good_id).or_default();
            
            let new_qty = item.quantity.round() + qty.round();
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
                        self.instruments.get(inst_id).map(|i| &i.instrument_type),
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
        self.instruments.insert(inst_id, inst);
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
                if let Some(instrument) = self.instruments.get(inst_id) {
                    if is_security(instrument) {
                        let quantity = holding.total_position();
                        let book_value = instrument.face_value().unwrap_or(Money::ZERO);
                        total_assets += book_value * quantity;
                    }
                }
            }
        }

        let discrepancy = total_assets - total_liabilities;
        const TOLERANCE: f64 = 1.0; // Allow a small tolerance for floating point inaccuracies

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
