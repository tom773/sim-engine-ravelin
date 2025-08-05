use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use serde_with::{serde_as, DisplayFromStr};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimState {
    pub ticknum: u32,
    pub current_date: chrono::NaiveDate,
    pub financial_system: FinancialSystem,
    pub agents: AgentRegistry,
    pub config: SimConfig,
    pub history: SimHistory,
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            ticknum: 0,
            current_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            financial_system: FinancialSystem::default(),
            agents: AgentRegistry::default(),
            config: SimConfig::default(),
            history: SimHistory::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimConfig {
    pub iterations: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimHistory {
    pub transactions: Vec<Transaction>,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialSystem {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub instruments: HashMap<InstrumentId, FinancialInstrument>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub balance_sheets: HashMap<AgentId, BalanceSheet>,
    pub central_bank: CentralBank,
    pub exchange: Exchange,
    pub goods: GoodsRegistry,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AgentRegistry {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub banks: HashMap<AgentId, Bank>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub consumers: HashMap<AgentId, Consumer>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub firms: HashMap<AgentId, Firm>,
}
impl AgentRegistry {
    pub fn agent_exists(&self, id: &AgentId) -> bool {
        self.banks.contains_key(id) || self.consumers.contains_key(id) || self.firms.contains_key(id)
    }
    pub fn get_bank(&self, id: &AgentId) -> Option<&Bank> { self.banks.get(id) }
    pub fn get_consumer(&self, id: &AgentId) -> Option<&Consumer> { self.consumers.get(id) }
    pub fn get_firm(&self, id: &AgentId) -> Option<&Firm> { self.firms.get(id) }
    pub fn get_bank_mut(&mut self, id: &AgentId) -> Option<&mut Bank> { self.banks.get_mut(id) }
    pub fn get_consumer_mut(&mut self, id: &AgentId) -> Option<&mut Consumer> { self.consumers.get_mut(id) }
    pub fn get_firm_mut(&mut self, id: &AgentId) -> Option<&mut Firm> { self.firms.get_mut(id) }
}
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
        #[serde_as(as = "HashMap<DisplayFromStr, _>")]
        goods: HashMap<GoodId, InventoryItem>
    },
    Equipment { description: String, depreciation_rate: f64 },
    IntellectualProperty { description: String },
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Exchange {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub goods_markets: HashMap<GoodId, GoodsMarket>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub financial_markets: HashMap<FinancialMarketId, FinancialMarket>,
}

impl Exchange {
    pub fn register_goods_market(&mut self, good_id: GoodId, goods_registry: &GoodsRegistry) {
        let name = goods_registry.get_good_name(&good_id).unwrap_or("Unknown Good").to_string();
        self.goods_markets.entry(good_id).or_insert_with(|| GoodsMarket::new(good_id, name));
    }

    pub fn register_financial_market(&mut self, market_id: FinancialMarketId) {
        let name = match &market_id {
            FinancialMarketId::SecuredOvernightFinancing => "Secured Overnight Financing".to_string(),
            FinancialMarketId::Treasury { tenor } => format!("Treasury {}", tenor),
            FinancialMarketId::CorporateBond { rating } => format!("Corporate Bond {:?}", rating),
        };
        self.financial_markets.entry(market_id.clone()).or_insert_with(|| FinancialMarket::new(market_id, name));
    }

    pub fn goods_market(&self, good_id: &GoodId) -> Option<&GoodsMarket> {
        self.goods_markets.get(good_id)
    }

    pub fn goods_market_mut(&mut self, good_id: &GoodId) -> Option<&mut GoodsMarket> {
        self.goods_markets.get_mut(good_id)
    }

    pub fn financial_market(&self, market_id: &FinancialMarketId) -> Option<&FinancialMarket> {
        self.financial_markets.get(market_id)
    }

    pub fn financial_market_mut(&mut self, market_id: &FinancialMarketId) -> Option<&mut FinancialMarket> {
        self.financial_markets.get_mut(market_id)
    }

    pub fn clear_markets(&mut self) -> Vec<Trade> {
        let mut all_trades = Vec::new();
        for (id, market) in self.goods_markets.iter_mut() {
            all_trades.extend(market.order_book.clear_and_match(&MarketId::Goods(*id)));
        }
        for (id, market) in self.financial_markets.iter_mut() {
            all_trades.extend(market.order_book.clear_and_match(&MarketId::Financial(id.clone())));
        }
        all_trades
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoodsMarket {
    pub good_id: GoodId,
    pub name: String,
    pub order_book: OrderBook,
}

impl GoodsMarket {
    pub fn new(good_id: GoodId, name: String) -> Self {
        Self { good_id, name, order_book: OrderBook::new() }
    }

    pub fn best_ask(&self) -> Option<&Ask> {
        self.order_book.asks.iter().min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialMarket {
    pub market_id: FinancialMarketId,
    pub name: String,
    pub order_book: OrderBook,
}

impl FinancialMarket {
    pub fn new(market_id: FinancialMarketId, name: String) -> Self {
        Self { market_id, name, order_book: OrderBook::new() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub id: uuid::Uuid,
    pub date: u32,
    pub qty: f64,
    pub from: AgentId,
    pub to: AgentId,
    pub tx_type: TransactionType,
    pub instrument_id: Option<InstrumentId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionType {
    Deposit { holder: AgentId, bank: AgentId, amount: f64 },
    Withdrawal { holder: AgentId, bank: AgentId, amount: f64 },
    Transfer { from: AgentId, to: AgentId, amount: f64 },
    InterestPayment,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self { iterations: 100 }
    }
}

impl Default for SimHistory {
    fn default() -> Self {
        Self { transactions: Vec::new() }
    }
}

impl Default for FinancialSystem {
    fn default() -> Self {
        let central_bank =
            CentralBank { id: AgentId(uuid::Uuid::new_v4()), policy_rate: 430.0, reserve_requirement: 0.1 };
        let mut balance_sheets = HashMap::new();
        balance_sheets.insert(central_bank.id, BalanceSheet::new(central_bank.id));

        Self {
            instruments: HashMap::new(),
            balance_sheets,
            central_bank,
            exchange: Exchange::default(),
            goods: GoodsRegistry::new(),
        }
    }
}

impl Default for Exchange {
    fn default() -> Self {
        Self { goods_markets: HashMap::new(), financial_markets: HashMap::new() }
    }
}

impl SimState {
    pub fn advance_time(&mut self) {
        self.ticknum += 1;
        self.current_date = self.current_date + chrono::Duration::days(1);
    }
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
    fn m1(&self, agents: &AgentRegistry) -> f64;
    fn m2(&self, agents: &AgentRegistry) -> f64;
}

impl InstrumentManager for FinancialSystem {
    fn create_instrument(&mut self, instrument: FinancialInstrument) -> Result<(), String> {
        let id = instrument.id;

        self.balance_sheets
            .get_mut(&instrument.creditor)
            .ok_or("Creditor not found")?
            .assets
            .insert(id, instrument.clone());

        self.balance_sheets
            .get_mut(&instrument.debtor)
            .ok_or("Debtor not found")?
            .liabilities
            .insert(id, instrument.clone());

        self.instruments.insert(id, instrument);
        Ok(())
    }

    fn transfer_instrument(&mut self, instrument_id: &InstrumentId, new_creditor: AgentId) -> Result<(), String> {
        let instrument = self.instruments.get_mut(instrument_id).ok_or("Instrument not found")?;
        let old_creditor = instrument.creditor;

        self.balance_sheets.get_mut(&old_creditor).ok_or("Old creditor not found")?.assets.remove(instrument_id);

        instrument.creditor = new_creditor;
        self.balance_sheets
            .get_mut(&new_creditor)
            .ok_or("New creditor not found")?
            .assets
            .insert(*instrument_id, instrument.clone());

        Ok(())
    }

    fn find_consolidatable_instrument(&self, new_inst: &FinancialInstrument) -> Option<InstrumentId> {
        if let Some(key) = new_inst.consolidation_key() {
            if let Some(creditor_bs) = self.balance_sheets.get(&new_inst.creditor) {
                for (id, existing) in &creditor_bs.assets {
                    if existing.consolidation_key() == Some(key.clone()) {
                        return Some(*id);
                    }
                }
            }
        }
        None
    }

    fn create_or_consolidate_instrument(&mut self, instrument: FinancialInstrument) -> Result<InstrumentId, String> {
        if let Some(existing_id) = self.find_consolidatable_instrument(&instrument) {
            let principal_change = instrument.principal;
            let existing =
                self.instruments.get_mut(&existing_id).ok_or("Consolidatable instrument not found in main registry")?;
            existing.principal += principal_change;

            self.balance_sheets
                .get_mut(&existing.creditor)
                .and_then(|bs| bs.assets.get_mut(&existing_id))
                .map(|inst| inst.principal += principal_change);
            self.balance_sheets
                .get_mut(&existing.debtor)
                .and_then(|bs| bs.liabilities.get_mut(&existing_id))
                .map(|inst| inst.principal += principal_change);

            Ok(existing_id)
        } else {
            let id = instrument.id;
            self.create_instrument(instrument)?;
            Ok(id)
        }
    }

    fn update_instrument(&mut self, id: &InstrumentId, new_principal: f64) -> Result<(), String> {
        let instrument = self.instruments.get_mut(id).ok_or("Instrument not found")?;
        instrument.principal = new_principal;

        self.balance_sheets
            .get_mut(&instrument.creditor)
            .and_then(|bs| bs.assets.get_mut(id))
            .map(|inst| inst.principal = new_principal);
        self.balance_sheets
            .get_mut(&instrument.debtor)
            .and_then(|bs| bs.liabilities.get_mut(id))
            .map(|inst| inst.principal = new_principal);

        Ok(())
    }

    fn remove_instrument(&mut self, id: &InstrumentId) -> Result<(), String> {
        if let Some(instrument) = self.instruments.remove(id) {
            self.balance_sheets.get_mut(&instrument.creditor).and_then(|bs| bs.assets.remove(id));
            self.balance_sheets.get_mut(&instrument.debtor).and_then(|bs| bs.liabilities.remove(id));
            Ok(())
        } else {
            Err("Instrument not found".to_string())
        }
    }

    fn swap_instrument(
        &mut self, id: &InstrumentId, new_debtor: &AgentId, new_creditor: &AgentId,
    ) -> Result<(), String> {
        let instrument = self.instruments.get_mut(id).ok_or("Instrument not found")?;
        let old_debtor = instrument.debtor;
        let old_creditor = instrument.creditor;

        instrument.debtor = *new_debtor;
        instrument.creditor = *new_creditor;

        if let Some(liability) = self.balance_sheets.get_mut(&old_debtor).and_then(|bs| bs.liabilities.remove(id)) {
            self.balance_sheets.get_mut(new_debtor).and_then(|bs| bs.liabilities.insert(*id, liability));
        }

        if let Some(asset) = self.balance_sheets.get_mut(&old_creditor).and_then(|bs| bs.assets.remove(id)) {
            self.balance_sheets.get_mut(new_creditor).and_then(|bs| bs.assets.insert(*id, asset));
        }

        Ok(())
    }
}

impl FinancialStatistics for FinancialSystem {
    fn m0(&self) -> f64 {
        self.balance_sheets
            .get(&self.central_bank.id)
            .map(|cb_bs| cb_bs.liabilities.values().map(|inst| inst.principal).sum())
            .unwrap_or(0.0)
    }

    fn m1(&self, agents: &AgentRegistry) -> f64 {
        let bank_ids: HashSet<AgentId> = agents.banks.keys().cloned().collect();

        self.balance_sheets
            .values()
            .filter(|bs| !bank_ids.contains(&bs.agent_id) && bs.agent_id != self.central_bank.id)
            .map(|bs| {
                bs.assets
                    .values()
                    .filter(|inst| {
                        inst.details.as_any().is::<CashDetails>() || inst.details.as_any().is::<DemandDepositDetails>()
                    })
                    .map(|inst| inst.principal)
                    .sum::<f64>()
            })
            .sum()
    }

    fn m2(&self, agents: &AgentRegistry) -> f64 {
        let m1 = self.m1(agents);

        let bank_ids: HashSet<AgentId> = agents.banks.keys().cloned().collect();

        let savings_deposits: f64 = self
            .balance_sheets
            .values()
            .filter(|bs| !bank_ids.contains(&bs.agent_id) && bs.agent_id != self.central_bank.id)
            .map(|bs| {
                bs.assets
                    .values()
                    .filter(|inst| inst.details.as_any().is::<SavingsDepositDetails>())
                    .map(|inst| inst.principal)
                    .sum::<f64>()
            })
            .sum();

        m1 + savings_deposits
    }
}