use crate::prelude::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConsolidationKey {
    pub issuer: AgentId,
    pub instrument_type: String,
    pub subtype: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentInfo {
    pub instrument_id: InstrumentId,
    pub instrument_type: &'static str,
    pub issuer_id: Option<AgentId>,
    pub issuer_name: Option<String>,
    pub face_value: Option<Money>,
    pub coupon_rate_bps: Option<BasisPoints>,
    pub maturity_date: Option<NaiveDate>,
    pub remaining_years: Option<f64>,
    pub currency: Option<Currency>,
    pub market_id: Option<MarketId>,
}

impl Instrument {
    pub fn get_consolidation_key(&self) -> ConsolidationKey {
        match &self.instrument_type {
            InstrumentType::Cash(d) => ConsolidationKey {
                issuer: d.issuer,
                instrument_type: "Cash".to_string(),
                subtype: format!("{:?}", d.cash_type),
            },
            InstrumentType::Bond(d) => ConsolidationKey {
                issuer: d.issuer,
                instrument_type: "Bond".to_string(),
                subtype: format!("{:?}_{:?}_{:?}_{:?}", d.bond_type, d.rating, d.maturity_date, d.coupon_rate_bps),
            },
            InstrumentType::RealAsset(d) => {
                let (owner, subtype) = match d {
                    RealAssetType::Inventory { owner, .. } => (*owner, "Inventory".to_string()),
                    RealAssetType::Property { owner, .. } => (*owner, "Property".to_string()),
                };
                ConsolidationKey { issuer: owner, instrument_type: "RealAsset".to_string(), subtype }
            }
            InstrumentType::Equity(d) => ConsolidationKey {
                issuer: d.issuer,
                instrument_type: "Equity".to_string(),
                subtype: "CommonStock".to_string(),
            },
            InstrumentType::Repo(d) => ConsolidationKey {
                issuer: d.borrower,
                instrument_type: "Repo".to_string(),
                subtype: "Repo".to_string(),
            },
            InstrumentType::Derivative(d) => ConsolidationKey {
                issuer: AgentId(Uuid::nil()),
                instrument_type: "Derivative".to_string(),
                subtype: format!("{:?}", d.underlying),
            },
            InstrumentType::StructuredTranche(d) => ConsolidationKey {
                issuer: d.issuer,
                instrument_type: "StructuredTranche".to_string(),
                subtype: format!("{:?}_{:?}", d.tranche_type, d.rating),
            },
        }
    }
}

impl FinancialSystem {
    pub fn find_bank_reserves_account(&self, bank_id: &AgentId) -> Option<InstrumentId> {
        let bs = self.balance_sheets.get(bank_id)?;
        bs.assets.iter().find_map(|(id, _pos)| {
            let inst = self.instruments.get(id)?;
            match &inst.instrument_type {
                InstrumentType::Cash(details) if details.cash_type == CashType::CentralBankReserves => Some(*id),
                _ => None,
            }
        })
    }

    pub fn find_agent_liquid_account(&self, agent_id: &AgentId) -> Option<(InstrumentId, AgentId)> {
        if *agent_id == self.government.id {
            return self.find_government_tga_account();
        }

        let is_bank = self.find_bank_reserves_account(agent_id).is_some();
        let is_central_bank = *agent_id == self.central_bank.id;

        if is_bank || is_central_bank {
            return self.find_bank_reserves_account(agent_id).map(|reserves_id| (reserves_id, self.central_bank.id));
        }

        let bs = self.balance_sheets.get(agent_id)?;
        bs.assets.iter().find_map(|(id, _)| {
            let inst = self.instruments.get(id)?;
            match &inst.instrument_type {
                InstrumentType::Cash(details) if details.cash_type == CashType::DemandDeposit => {
                    Some((*id, details.issuer))
                }
                _ => None,
            }
        })
    }

    pub fn find_government_tga_account(&self) -> Option<(InstrumentId, AgentId)> {
        let gov_bs = self.balance_sheets.get(&self.government.id)?;
        gov_bs.assets.iter().find_map(|(id, _pos)| {
            let inst = self.instruments.get(id)?;
            match &inst.instrument_type {
                InstrumentType::Cash(details) if details.cash_type == CashType::TreasuryGeneralAccount => {
                    Some((*id, self.central_bank.id))
                }
                _ => None,
            }
        })
    }

    pub fn find_any_bank_account(&self) -> Option<(InstrumentId, AgentId)> {
        self.instruments.values().find_map(|inst| {
            if let InstrumentType::Cash(details) = &inst.instrument_type {
                if details.cash_type == CashType::DemandDeposit {
                    return Some((inst.id, details.issuer));
                }
            }
            None
        })
    }
    pub fn create_or_consolidate_position(
        &mut self, creditor_id: &AgentId, debtor_id: &AgentId, instrument_id: &InstrumentId, quantity_change: f64,
        book_value_per_unit: f64,
    ) -> Result<(), String> {
        if let Some(inst) = self.instruments.get(instrument_id) {
            match inst.instrument_type {
                InstrumentType::Bond(_)
                | InstrumentType::Equity(_)
                | InstrumentType::StructuredTranche(_)
                | InstrumentType::Derivative(_) => {
                    return Err(format!(
                        "Security {} must use CSD, not balance sheets. Use CreateInstrument effect.",
                        instrument_id
                    ));
                }
                _ => {}
            }
        }
        let book_value_money = Money::from_f64(book_value_per_unit).unwrap_or(Money::ZERO);

        let creditor_bs = self.balance_sheets.get_mut(creditor_id).ok_or("Creditor not found")?;
        let asset_pos = creditor_bs.assets.entry(*instrument_id).or_insert_with(|| Position {
            quantity: 0.0,
            book_value_per_unit: book_value_money,
            cost_basis_per_unit: book_value_money,
        });
        asset_pos.quantity += quantity_change;

        let debtor_bs = self.balance_sheets.get_mut(debtor_id).ok_or("Debtor not found")?;
        let liability_pos = debtor_bs.liabilities.entry(*instrument_id).or_insert_with(|| Position {
            quantity: 0.0,
            book_value_per_unit: book_value_money,
            cost_basis_per_unit: book_value_money,
        });
        liability_pos.quantity += quantity_change;

        Ok(())
    }
    pub fn get_instrument_info(
        &self, instrument_id: &InstrumentId, agents: &AgentRegistry, current_date: NaiveDate,
    ) -> Option<InstrumentInfo> {
        let instrument = self.instruments.get(instrument_id)?;

        let mut info = InstrumentInfo {
            instrument_id: *instrument_id,
            instrument_type: instrument.type_as_string(),
            issuer_id: None,
            issuer_name: None,
            face_value: None,
            coupon_rate_bps: None,
            maturity_date: None,
            remaining_years: None,
            currency: None,
            market_id: Some(MarketId::Financial(*instrument_id)),
        };

        let issuer_id = match &instrument.instrument_type {
            InstrumentType::Cash(d) => {
                info.currency = Some(d.currency);
                Some(d.issuer)
            }
            InstrumentType::Bond(d) => {
                info.face_value = Some(d.face_value);
                info.coupon_rate_bps = Some(d.coupon_rate_bps);
                info.maturity_date = Some(d.maturity_date);
                info.remaining_years = Some(d.remaining_tenor_years(current_date));
                Some(d.issuer)
            }
            InstrumentType::Equity(d) => Some(d.issuer),
            InstrumentType::Repo(d) => Some(d.borrower),
            InstrumentType::StructuredTranche(d) => {
                info.face_value = Some(d.face_value);
                info.coupon_rate_bps = Some(d.coupon_rate_bps);
                info.maturity_date = Some(d.maturity_date);
                Some(d.issuer)
            }
            _ => None,
        };

        if let Some(id) = issuer_id {
            info.issuer_id = Some(id);
            info.issuer_name = agents
                .banks
                .get(&id)
                .map(|a| a.name.clone())
                .or_else(|| agents.firms.get(&id).map(|a| a.name.clone()))
                .or_else(|| {
                    if id == self.government.id {
                        Some("Government".to_string())
                    } else if id == self.central_bank.id {
                        Some("Central Bank".to_string())
                    } else {
                        None
                    }
                });
        }

        Some(info)
    }
    pub fn get_agent_inventory(&self, agent_id: &AgentId) -> HashMap<GoodId, InventoryItem> {
        if let Some(bs) = self.balance_sheets.get(agent_id) {
            for inst_id in bs.assets.keys() {
                if let Some(inst) = self.instruments.get(inst_id) {
                    if let InstrumentType::RealAsset(RealAssetType::Inventory { goods, .. }) = &inst.instrument_type {
                        return goods.clone();
                    }
                }
            }
        }
        HashMap::new()
    }
    pub fn find_general_labour_market(&self) -> Option<LabourMarketId> {
        self.exchange.labour_markets.keys().next().cloned()
    }
    pub fn get_agent_total_positions(&self, agent_id: &AgentId) -> HashMap<InstrumentId, Position> {
        let mut positions = HashMap::new();

        if let Some(bs) = self.balance_sheets.get(agent_id) {
            for (inst_id, pos) in &bs.assets {
                if let Some(_inst) = self.instruments.get(inst_id) {
                    if !self.clearing_house.csd.is_security(inst_id) {
                        positions.insert(*inst_id, pos.clone());
                    }
                }
            }
        }

        if let Some(csd_account) = self.clearing_house.csd.custody_accounts.get(agent_id) {
            for (inst_id, holding) in &csd_account.holdings {
                let quantity = holding.total_position();
                if quantity > 1e-9 {
                    if let Some(inst) = self.instruments.get(inst_id) {
                        let book_value = inst.face_value().unwrap_or(Money::from(1000 as i64));
                        positions.insert(
                            *inst_id,
                            Position { quantity, book_value_per_unit: book_value, cost_basis_per_unit: book_value },
                        );
                    }
                }
            }
        }

        positions
    }

    pub fn validate_security_availability(
        &self, agent_id: &AgentId, instrument_id: &InstrumentId, required_quantity: f64,
    ) -> Result<(), String> {
        if !self.clearing_house.csd.is_security(instrument_id) {
            return Ok(());
        }

        let available = self
            .clearing_house
            .csd
            .custody_accounts
            .get(agent_id)
            .and_then(|account| account.holdings.get(instrument_id))
            .map(|holding| holding.available)
            .unwrap_or(0.0);

        if available < required_quantity {
            return Err(format!(
                "Insufficient securities: {} available, {} required for agent {}",
                available, required_quantity, agent_id
            ));
        }

        Ok(())
    }
    pub fn get_total_assets(&self, agent_id: &AgentId) -> f64 {
        let bs_assets_value = self
            .balance_sheets
            .get(agent_id)
            .map(|bs| {
                bs.assets
                    .iter()
                    .map(|(id, pos)| {
                        let price = self.get_market_price(id).unwrap_or(pos.book_value_per_unit);
                        price.to_f64() * pos.quantity
                    })
                    .sum::<f64>()
            })
            .unwrap_or(0.0);

        let csd_securities_value = self
            .clearing_house
            .csd
            .get_all_positions(agent_id)
            .iter()
            .map(|(id, qty)| {
                let price = self
                    .get_market_price(id)
                    .or_else(|| self.instruments.get(id).and_then(|i| i.face_value()))
                    .unwrap_or(Money::ZERO);
                price.to_f64() * qty
            })
            .sum::<f64>();

        bs_assets_value + csd_securities_value
    }

    pub fn get_total_liabilities(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets
            .get(agent_id)
            .map(|bs| {
                bs.liabilities
                    .iter()
                    .map(|(id, pos)| {
                        let price = self.get_market_price(id).unwrap_or(pos.book_value_per_unit);
                        price.to_f64() * pos.quantity
                    })
                    .sum()
            })
            .unwrap_or(0.0)
    }

    pub fn get_liquid_assets(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets
            .get(agent_id)
            .map(|bs| {
                bs.assets
                    .iter()
                    .filter_map(|(id, pos)| {
                        self.instruments.get(id).and_then(|inst| {
                            if let InstrumentType::Cash(_) = &inst.instrument_type {
                                Some(pos.quantity) // Cash has a price of 1.0
                            } else {
                                None
                            }
                        })
                    })
                    .sum()
            })
            .unwrap_or(0.0)
    }

    pub fn get_market_price(&self, instrument_id: &InstrumentId) -> Option<Money> {
        self.exchange.financial_market(instrument_id).and_then(|market| market.representative_price())
    }
}
