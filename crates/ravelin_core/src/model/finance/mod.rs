use crate::*;
use ravelin_macros::CollectAgents;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;
use uuid::Uuid;

pub mod balance_sheet;
pub use balance_sheet::*;

pub mod market;
pub use market::*;

pub mod statistics;
pub use statistics::*;

pub mod instruments;
pub use instruments::*;

pub mod markets;
pub use markets::*;

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, Copy, Default)]
pub struct AgentId(pub Uuid);
prep_serde_as!(AgentId, Uuid);

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, Copy, Default)]
pub struct InstrumentId(pub Uuid);
prep_serde_as!(InstrumentId, Uuid);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy, Default)]
pub struct AssetId(pub Uuid);
prep_serde_as!(AssetId, Uuid);

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RealAsset {
    pub id: AssetId,
    pub asset_type: RealAssetType,
    pub owner: AgentId,
    pub market_value: f64,
    pub acquired_date: u32,
}

#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RealAssetType {
    RealEstate { address: String, property_type: String },
    Inventory {
        #[serde_as(as = "HashMap<_, _>")]
        goods: HashMap<GoodId, InventoryItem>
    },
    Equipment { description: String, depreciation_rate: f64 },
    IntellectualProperty { description: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InventoryItem {
    pub quantity: f64,
    pub unit_cost: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub date: u32,
    pub qty: f64,
    pub from: AgentId,
    pub to: AgentId,
    pub tx_type: TransactionType,
    pub instrument_id: Option<InstrumentId>,
}
impl Transaction {
    pub fn new(
        tx_type: TransactionType,
        inst: InstrumentId,
        from: AgentId,
        to: AgentId,
        amount: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            date: chrono::Utc::now().timestamp() as u32,
            from,
            to,
            qty: amount,
            tx_type,
            instrument_id: Some(inst),
        }
    }
}
impl Default for Transaction {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            date: 0,
            qty: 0.0,
            from: Default::default(),
            to: Default::default(),
            tx_type: TransactionType::Transfer { from: Default::default(), to: Default::default(), amount: 0.0 },
            instrument_id: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionType {
    Deposit { holder: AgentId, bank: AgentId, amount: f64 },
    Withdrawal { holder: AgentId, bank: AgentId, amount: f64 },
    Transfer { from: AgentId, to: AgentId, amount: f64 },
    InterestPayment,
}

// --- The Main Financial System Struct ---

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, CollectAgents)]
pub struct FinancialSystem {
    #[serde_as(as = "HashMap<_, _>")]
    pub instruments: HashMap<InstrumentId, FinancialInstrument>,
    #[serde_as(as = "HashMap<_, _>")]
    pub balance_sheets: HashMap<AgentId, BalanceSheet>,
    #[serde_as(as = "HashMap<_, _>")]
    #[agent_collection]
    pub commercial_banks: HashMap<AgentId, Bank>,
    pub central_bank: CentralBank,
    pub exchange: Exchange,
    pub goods: GoodsRegistry,
}

pub trait InstrumentManager {
    fn update_instrument(&mut self, id: &InstrumentId, new_principal: f64) -> Result<(), String>;
    fn create_instrument(&mut self, instrument: FinancialInstrument) -> Result<(), String>;
    fn create_or_consolidate_instrument(&mut self, instrument: FinancialInstrument) -> Result<InstrumentId, String>;
    fn find_consolidatable_instrument(&self, new_inst: &FinancialInstrument) -> Option<InstrumentId>;
    fn remove_instrument(&mut self, id: &InstrumentId) -> Result<(), String>;
    fn transfer_instrument(&mut self, id: &InstrumentId, new_creditor: AgentId) -> Result<(), String>;
    fn swap_instrument(
        &mut self, id: &InstrumentId, new_debtor: &AgentId, new_creditor: &AgentId,
    ) -> Result<(), String>;
}

pub trait FinancialStatistics {
    fn m0(&self) -> f64;
    fn m1(&self) -> f64;
    fn m2(&self) -> f64;
}

impl InstrumentManager for FinancialSystem {
    fn create_instrument(&mut self, instrument: FinancialInstrument) -> Result<(), String> {
        let id = instrument.id.clone();

        if let Some(creditor_bs) = self.balance_sheets.get_mut(&instrument.creditor) {
            creditor_bs.assets.insert(id.clone(), instrument.clone());
        } else {
            return Err("Creditor not found".to_string());
        }
        if let Some(debtor_bs) = self.balance_sheets.get_mut(&instrument.debtor) {
            debtor_bs.liabilities.insert(id.clone(), instrument.clone());
        } else {
            if let Some(creditor_bs) = self.balance_sheets.get_mut(&instrument.creditor) {
                creditor_bs.assets.remove(&id);
            }
            return Err("Debtor not found".to_string());
        }
        self.instruments.insert(id, instrument);
        Ok(())
    }

    fn transfer_instrument(&mut self, instrument_id: &InstrumentId, new_creditor: AgentId) -> Result<(), String> {
        let instrument = self.instruments.get_mut(instrument_id).ok_or("Instrument not found")?;

        let old_creditor = instrument.creditor.clone();

        if let Some(old_bs) = self.balance_sheets.get_mut(&old_creditor) {
            old_bs.assets.remove(instrument_id);
        }

        instrument.creditor = new_creditor.clone();
        if let Some(new_bs) = self.balance_sheets.get_mut(&new_creditor) {
            new_bs.assets.insert(instrument_id.clone(), instrument.clone());
        }

        Ok(())
    }
    fn find_consolidatable_instrument(&self, new_inst: &FinancialInstrument) -> Option<InstrumentId> {
        if let Some(key) = new_inst.consolidation_key() {
            if let Some(creditor_bs) = self.balance_sheets.get(&new_inst.creditor) {
                for (id, existing) in &creditor_bs.assets {
                    if existing.consolidation_key() == Some(key.clone()) {
                        return Some(id.clone());
                    }
                }
            }
        }
        None
    }
    fn create_or_consolidate_instrument(&mut self, instrument: FinancialInstrument) -> Result<InstrumentId, String> {
        if let Some(existing_id) = self.find_consolidatable_instrument(&instrument) {
            if let Some(existing) = self.instruments.get_mut(&existing_id) {
                existing.principal += instrument.principal;

                if let Some(creditor_bs) = self.balance_sheets.get_mut(&instrument.creditor) {
                    if let Some(asset) = creditor_bs.assets.get_mut(&existing_id) {
                        asset.principal += instrument.principal;
                    }
                }

                if let Some(debtor_bs) = self.balance_sheets.get_mut(&instrument.debtor) {
                    if let Some(liability) = debtor_bs.liabilities.get_mut(&existing_id) {
                        liability.principal += instrument.principal;
                    }
                }

                Ok(existing_id)
            } else {
                Err("Found consolidatable instrument but couldn't access it".to_string())
            }
        } else {
            let id = instrument.id.clone();
            self.create_instrument(instrument)?;
            Ok(id)
        }
    }
    fn update_instrument(&mut self, id: &InstrumentId, new_principal: f64) -> Result<(), String> {
        if let Some(instrument) = self.instruments.get_mut(id) {
            instrument.principal = new_principal;
            if let Some(creditor_bs) = self.balance_sheets.get_mut(&instrument.creditor) {
                if let Some(asset) = creditor_bs.assets.get_mut(id) {
                    asset.principal = new_principal;
                }
            }
            if let Some(debtor_bs) = self.balance_sheets.get_mut(&instrument.debtor) {
                if let Some(liability) = debtor_bs.liabilities.get_mut(id) {
                    liability.principal = new_principal;
                }
            }
            Ok(())
        } else {
            Err("Instrument not found".to_string())
        }
    }
    fn remove_instrument(&mut self, id: &InstrumentId) -> Result<(), String> {
        if let Some(instrument) = self.instruments.remove(id) {
            if let Some(creditor_bs) = self.balance_sheets.get_mut(&instrument.creditor) {
                creditor_bs.assets.remove(id);
            }
            if let Some(debtor_bs) = self.balance_sheets.get_mut(&instrument.debtor) {
                debtor_bs.liabilities.remove(id);
            }
            Ok(())
        } else {
            Err("Instrument not found".to_string())
        }
    }
    fn swap_instrument(
        &mut self, id: &InstrumentId, new_debtor: &AgentId, new_creditor: &AgentId,
    ) -> Result<(), String> {
        if let Some(instrument) = self.instruments.get_mut(id) {
            let old_debtor = instrument.debtor.clone();
            let old_creditor = instrument.creditor.clone();

            instrument.debtor = new_debtor.clone();
            instrument.creditor = new_creditor.clone();

            if let Some(old_bs) = self.balance_sheets.get_mut(&old_debtor) {
                old_bs.liabilities.remove(id);
            }
            if let Some(new_bs) = self.balance_sheets.get_mut(&new_debtor) {
                new_bs.liabilities.insert(id.clone(), instrument.clone());
            }

            if let Some(old_bs) = self.balance_sheets.get_mut(&old_creditor) {
                old_bs.assets.remove(id);
            }
            if let Some(new_bs) = self.balance_sheets.get_mut(&new_creditor) {
                new_bs.assets.insert(id.clone(), instrument.clone());
            }

            Ok(())
        } else {
            Err("Instrument not found".to_string())
        }
    }
}

impl Default for FinancialSystem {
    fn default() -> Self {
        let central_bank = CentralBank::new(430.0, 0.1);
        let cb_id = central_bank.id.clone();

        let mut balance_sheets = HashMap::new();
        balance_sheets.insert(cb_id.clone(), BalanceSheet::new(cb_id.clone()));

        Self {
            exchange: Exchange::new(),
            instruments: HashMap::new(),
            balance_sheets,
            central_bank,
            commercial_banks: HashMap::new(),
            goods: GoodsRegistry::new(),
        }
    }
}