use crate::*;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;
use uuid::Uuid;

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub agent_id: AgentId,
    #[serde_as(as = "HashMap<_, _>")]
    pub assets: HashMap<InstrumentId, FinancialInstrument>,
    #[serde_as(as = "HashMap<_, _>")]
    pub liabilities: HashMap<InstrumentId, FinancialInstrument>,
    #[serde_as(as = "HashMap<_, _>")]
    pub real_assets: HashMap<AssetId, RealAsset>,
}

impl BalanceSheet {
    pub fn new(owner: AgentId) -> Self {
        Self { agent_id: owner, assets: HashMap::new(), liabilities: HashMap::new(), real_assets: HashMap::new() }
    }

    pub fn liquid_assets(&self) -> f64 {
        self.assets
            .values()
            .filter(|inst| {
                inst.details.as_any().is::<CashDetails>() || inst.details.as_any().is::<DemandDepositDetails>()
            })
            .map(|inst| inst.principal)
            .sum()
    }

    pub fn deposits_at_bank(&self, bank_id: &AgentId) -> f64 {
        self.assets
            .values()
            .filter(|inst| {
                inst.debtor == *bank_id && (
                    inst.details.as_any().is::<DemandDepositDetails>() 
                    || inst.details.as_any().is::<SavingsDepositDetails>()
                )
            })
            .map(|inst| inst.principal)
            .sum()
    }

    pub fn total_assets(&self) -> f64 {
        let financial = self.assets.values().map(|inst| inst.principal).sum::<f64>();
        let real = self.real_assets.values().map(|asset| asset.market_value).sum::<f64>();
        financial + real
    }

    pub fn total_liabilities(&self) -> f64 {
        self.liabilities.values().map(|inst| inst.principal).sum()
    }

    pub fn net_worth(&self) -> f64 {
        self.total_assets() - self.total_liabilities()
    }
}
pub trait BalanceSheetQuery {
    fn get_bs_by_id(&self, agent_id: &AgentId) -> Option<&BalanceSheet>;
    fn get_bs_mut_by_id(&mut self, agent_id: &AgentId) -> Option<&mut BalanceSheet>;
    fn get_total_assets(&self, agent_id: &AgentId) -> f64;
    fn get_total_liabilities(&self, agent_id: &AgentId) -> f64;
    fn get_liquid_assets(&self, agent_id: &AgentId) -> f64;
    fn get_deposits_at_bank(&self, agent_id: &AgentId, bank_id: &AgentId) -> f64;
    fn get_cash_assets(&self, agent_id: &AgentId) -> f64;
    fn liquidity(&self, agent_id: &AgentId) -> f64;
    fn get_bank_reserves(&self, agent_id: &AgentId) -> Option<f64>;
}

impl BalanceSheetQuery for FinancialSystem {
    fn get_bs_by_id(&self, agent_id: &AgentId) -> Option<&BalanceSheet> {
        self.balance_sheets.get(agent_id)
    }
    fn get_bs_mut_by_id(&mut self, agent_id: &AgentId) -> Option<&mut BalanceSheet> {
        self.balance_sheets.get_mut(agent_id)
    }
    fn get_total_assets(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map(|bs| bs.total_assets()).unwrap_or(0.0)
    }
    fn get_cash_assets(&self, agent_id: &AgentId) -> f64 {
        self.get_bs_by_id(agent_id)
            .map(|bs| {
                bs.assets
                    .values()
                    .filter(|inst| inst.details.as_any().is::<CashDetails>())
                    .map(|inst| inst.principal)
                    .sum::<f64>()
            })
            .unwrap_or(0.0)
    }
    fn get_total_liabilities(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map(|bs| bs.total_liabilities()).unwrap_or(0.0)
    }
    fn get_liquid_assets(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map(|bs| bs.liquid_assets()).unwrap_or(0.0)
    }
    fn get_deposits_at_bank(&self, agent_id: &AgentId, bank_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map(|bs| bs.deposits_at_bank(bank_id)).unwrap_or(0.0)
    }
    fn liquidity(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map(|bs| bs.liquid_assets()).unwrap_or(0.0)
    }
    fn get_bank_reserves(&self, agent_id: &AgentId) -> Option<f64> {
        self.balance_sheets.get(agent_id).map(|bs| {
            bs.assets
                .values()
                .filter(|inst| inst.details.as_any().is::<CentralBankReservesDetails>())
                .map(|inst| inst.principal)
                .sum::<f64>()
        })
    }
}

pub trait InventoryQuery {
    fn update_inventory_market_value(&mut self);
    fn get_or_create_inventory_mut(&mut self) -> &mut HashMap<GoodId, InventoryItem>;
    fn get_inventory(&self) -> Option<&HashMap<GoodId, InventoryItem>>;
    fn add_to_inventory(&mut self, good_id: &GoodId, quantity: f64, unit_cost: f64);
    fn remove_from_inventory(&mut self, good_id: &GoodId, quantity: f64) -> Result<(), String>;
}

impl InventoryQuery for BalanceSheet {
    fn update_inventory_market_value(&mut self) {
        let mut inventory_value = 0.0;
        let mut inventory_asset_id: Option<AssetId> = None;

        for asset in self.real_assets.values() {
            if let RealAssetType::Inventory { goods } = &asset.asset_type {
                inventory_asset_id = Some(asset.id);
                inventory_value = goods.values().map(|item| item.quantity * item.unit_cost).sum();
                break;
            }
        }

        if let Some(id) = inventory_asset_id {
            if let Some(asset) = self.real_assets.get_mut(&id) {
                asset.market_value = inventory_value;
            }
        }
    }
    fn get_inventory(&self) -> Option<&HashMap<GoodId, InventoryItem>> {
        let inventory_asset_id = self
            .real_assets
            .values()
            .find(|asset| matches!(asset.asset_type, RealAssetType::Inventory { .. }))
            .map(|asset| asset.id);

        if let Some(id) = inventory_asset_id {
            if let RealAssetType::Inventory { goods } = &self.real_assets[&id].asset_type {
                return Some(goods);
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    fn get_or_create_inventory_mut(&mut self) -> &mut HashMap<GoodId, InventoryItem> {
        let inventory_asset_id = self
            .real_assets
            .values()
            .find(|asset| matches!(asset.asset_type, RealAssetType::Inventory { .. }))
            .map(|asset| asset.id);

        let id_to_use = inventory_asset_id.unwrap_or_else(|| {
            let new_inventory_asset = RealAsset {
                id: AssetId(Uuid::new_v4()),
                asset_type: RealAssetType::Inventory { goods: HashMap::new() },
                owner: self.agent_id,
                market_value: 0.0,
                acquired_date: 0,
            };
            let new_id = new_inventory_asset.id;
            self.real_assets.insert(new_id, new_inventory_asset);
            new_id
        });

        if let RealAssetType::Inventory { goods } = &mut self.real_assets.get_mut(&id_to_use).unwrap().asset_type {
            goods
        } else {
            unreachable!();
        }
    }

    fn add_to_inventory(&mut self, good_id: &GoodId, quantity: f64, unit_cost: f64) {
        let inventory = self.get_or_create_inventory_mut();
        let item = inventory.entry(*good_id).or_insert(InventoryItem { quantity: 0.0, unit_cost: 0.0 });

        let new_total_quantity = item.quantity + quantity;
        if new_total_quantity > 0.0 {
            item.unit_cost = (item.quantity * item.unit_cost + quantity * unit_cost) / new_total_quantity;
        } else {
            item.unit_cost = 0.0;
        }
        item.quantity = new_total_quantity;

        self.update_inventory_market_value();
    }

    fn remove_from_inventory(&mut self, good_id: &GoodId, quantity: f64) -> Result<(), String> {
        let inventory = self.get_or_create_inventory_mut();
        if let Some(item) = inventory.get_mut(good_id) {
            if item.quantity >= quantity {
                item.quantity -= quantity;
                self.update_inventory_market_value();
                Ok(())
            } else {
                Err(format!(
                    "Insufficient inventory for good {:?}: have {}, need {}",
                    good_id.0, item.quantity, quantity
                ))
            }
        } else {
            Err(format!("No inventory for good {:?}", good_id.0))
        }
    }
}
