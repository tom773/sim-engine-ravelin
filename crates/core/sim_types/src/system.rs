use crate::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialSystem {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub instruments: HashMap<InstrumentId, FinancialInstrument>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub balance_sheets: HashMap<AgentId, BalanceSheet>,
    pub central_bank: CentralBank,
    pub government: Government,
    pub exchange: Exchange,
    pub goods: GoodsRegistry,
    pub yield_curve: YieldCurve,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct YieldCurve {
    pub date: chrono::NaiveDate,
    pub yields: HashMap<Tenor, f64>,
}

impl Default for FinancialSystem {
    fn default() -> Self {
        let central_bank =
            CentralBank { id: AgentId(uuid::Uuid::new_v4()), policy_rate: 430.0, reserve_requirement: 0.1 };
        let government = Government {
            id: AgentId(uuid::Uuid::new_v4()),
            tax_rates: TaxRates::default(),
            spending_targets: SpendingTargets::default(),
            debt_ceiling: Some(1_000_000_000.0),
            fiscal_policy: FiscalPolicy::default(),
        };
        let mut balance_sheets = HashMap::new();
        balance_sheets.insert(central_bank.id, BalanceSheet::new(central_bank.id));
        balance_sheets.insert(government.id, BalanceSheet::new(government.id));
        Self {
            instruments: HashMap::new(),
            balance_sheets,
            central_bank,
            government,
            exchange: Exchange::default(),
            goods: GoodsRegistry::new(),
            yield_curve: YieldCurve {
                date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                yields: HashMap::new(),
            },
        }
    }
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
    fn get_total_deposits(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map(|bs| bs.total_deposits()).unwrap_or(0.0)
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
    fn split_and_transfer_instrument(
        &mut self, instrument_id: &InstrumentId, buyer: AgentId, quantity_to_transfer: u64,
    ) -> Result<InstrumentId, String> {
        let seller_instrument = self.instruments.get(instrument_id).ok_or("Instrument not found")?.clone();

        let bond_details =
            seller_instrument.details.as_any().downcast_ref::<BondDetails>().ok_or("Instrument is not a bond")?;

        if bond_details.quantity < quantity_to_transfer {
            return Err(format!(
                "Insufficient bond quantity: have {}, need {}",
                bond_details.quantity, quantity_to_transfer
            ));
        }

        let seller = seller_instrument.creditor;
        let remaining_quantity = bond_details.quantity - quantity_to_transfer;
        let principal_per_bond = seller_instrument.principal / bond_details.quantity as f64;
        let transfer_principal = principal_per_bond * quantity_to_transfer as f64;
        let remaining_principal = seller_instrument.principal - transfer_principal;

        if remaining_quantity == 0 {
            self.remove_instrument(instrument_id)?;
        } else {
            let updated_instrument =
                self.instruments.get_mut(instrument_id).ok_or("Instrument not found for update")?;
            updated_instrument.principal = remaining_principal;

            if let Some(updated_details) = updated_instrument.details.as_any_mut().downcast_mut::<BondDetails>() {
                updated_details.quantity = remaining_quantity;
            }

            if let Some(seller_bs) = self.balance_sheets.get_mut(&seller) {
                if let Some(asset) = seller_bs.assets.get_mut(instrument_id) {
                    asset.principal = remaining_principal;
                    if let Some(asset_details) = asset.details.as_any_mut().downcast_mut::<BondDetails>() {
                        asset_details.quantity = remaining_quantity;
                    }
                }
            }

            let debtor = updated_instrument.debtor;
            if let Some(debtor_bs) = self.balance_sheets.get_mut(&debtor) {
                if let Some(liability) = debtor_bs.liabilities.get_mut(instrument_id) {
                    liability.principal = remaining_principal;
                    if let Some(liability_details) = liability.details.as_any_mut().downcast_mut::<BondDetails>() {
                        liability_details.quantity = remaining_quantity;
                    }
                }
            }
        }

        let mut buyer_bond_details = bond_details.clone();
        buyer_bond_details.quantity = quantity_to_transfer;

        let buyer_instrument = FinancialInstrument {
            id: InstrumentId(Uuid::new_v4()),
            creditor: buyer,
            debtor: seller_instrument.debtor,
            principal: transfer_principal,
            details: Box::new(buyer_bond_details),
            originated_date: seller_instrument.originated_date,
            accrued_interest: (seller_instrument.accrued_interest / bond_details.quantity as f64)
                * quantity_to_transfer as f64,
            last_accrual_date: seller_instrument.last_accrual_date,
        };

        self.create_or_consolidate_instrument(buyer_instrument)
    }
    fn pay_interest(&mut self, instrument_id: InstrumentId, payment_date: NaiveDate) -> Result<(), String> {
        let instrument = self.instruments.get_mut(&instrument_id).ok_or("Instrument not found")?;
        let bond_details =
            instrument.details.as_any_mut().downcast_mut::<BondDetails>().ok_or("Instrument is not a bond")?;

        let interest_payment = bond_details.coupon_rate * instrument.principal / 100.0;

        instrument.accrued_interest += interest_payment;

        instrument.last_accrual_date = payment_date;

        if let Some(bs) = self.balance_sheets.get_mut(&instrument.creditor) {
            bs.assets.entry(instrument_id).and_modify(|inst| {
                inst.accrued_interest += interest_payment;
            });
        }

        Ok(())
    }
}
impl FinancialStatistics for FinancialSystem {
    fn m0(&self) -> f64 {
        self.balance_sheets
            .values()
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
    fn m1(&self, bank_ids: &HashSet<AgentId>) -> f64 {
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

    fn m2(&self, bank_ids: &HashSet<AgentId>) -> f64 {
        let m1 = self.m1(bank_ids); // <-- Pass the hashset through

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

impl FinancialSystem {
    pub fn update_yield_curve(&mut self, date: chrono::NaiveDate) {
        let mut yields = HashMap::new();
        for (market_id, market) in &self.exchange.financial_markets {
            if let FinancialMarketId::Treasury { tenor } = market_id {
                if let (Some(bid), Some(ask)) = (market.order_book.best_bid(), market.order_book.best_ask()) {
                    let price = (bid.price + ask.price) / 2.0;
                    let daily_rate = market_id.price_to_daily_rate(price);
                    let annual_rate = (1.0 + daily_rate).powf(365.0) - 1.0;
                    yields.insert(*tenor, annual_rate);
                }
            }
        }
        self.yield_curve = YieldCurve { date, yields };
    }
}
