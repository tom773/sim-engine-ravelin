use crate::prelude::*;
use crate::types::money::Money;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::HashMap;

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct Position {
    pub quantity: f64,
    pub book_value_per_unit: Money,
    pub cost_basis_per_unit: Money,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BalanceSheet {
    pub agent_id: AgentId,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub assets: HashMap<InstrumentId, Position>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub liabilities: HashMap<InstrumentId, Position>,
    pub income_statement: IncomeStatement,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct IncomeStatement {
    pub revenue: Money,
    pub cost_of_goods_sold: Money,
    pub operating_expenses: Money,
    pub interest_income: Money,
    pub interest_expense: Money,
    pub net_income: Money,
}

fn get_market_price(inst: &Instrument, exchange: &Exchange) -> Option<Money> {
    if let Some(market) = exchange.markets.get(&inst.id) {
        if let Some(mid) = market.book.mid_price() {
            return Some(mid);
        }
    }

    match &inst.instrument_type {
        InstrumentType::Repo(r) => Some(r.loan_amount),
        InstrumentType::RealAsset(asset_type) => match asset_type {
            RealAssetType::Inventory { goods, .. } => {
                let total: Money = goods.values().map(|item| item.unit_cost * item.quantity).sum();
                Some(total)
            }
            RealAssetType::Property { market_value, .. } => Some(*market_value),
        },
        InstrumentType::Cash(_) => Some(Money::from(1 as i64)),
        _ => None,
    }
}

impl BalanceSheet {
    pub fn new(owner: AgentId) -> Self {
        Self {
            agent_id: owner,
            assets: HashMap::new(),
            liabilities: HashMap::new(),
            income_statement: IncomeStatement::default(),
        }
    }
    pub fn liquid_assets(&self, system: &FinancialSystem) -> Money {
        self.assets
            .iter()
            .filter_map(|(id, pos)| {
                let inst = system.instruments.get(id)?;
                if let InstrumentType::Cash(_) = inst.instrument_type {
                    Some(Money::from(1 as i64) * pos.quantity)
                } else {
                    None
                }
            })
            .sum()
    }

    pub fn deposits_at_bank(&self, system: &FinancialSystem, bank_id: &AgentId) -> f64 {
        self.assets
            .iter()
            .filter_map(|(id, pos)| {
                let inst = system.instruments.get(id)?;
                if let InstrumentType::Cash(c) = &inst.instrument_type {
                    if &c.issuer == bank_id && matches!(c.cash_type, CashType::DemandDeposit | CashType::SavingsDeposit)
                    {
                        return Some(pos.quantity);
                    }
                }
                None
            })
            .sum()
    }

    pub fn total_deposits(&self, system: &FinancialSystem) -> f64 {
        self.assets
            .iter()
            .filter_map(|(id, pos)| {
                let inst = system.instruments.get(id)?;
                if let InstrumentType::Cash(c) = &inst.instrument_type {
                    if matches!(c.cash_type, CashType::DemandDeposit | CashType::SavingsDeposit) {
                        return Some(pos.quantity);
                    }
                }
                None
            })
            .sum()
    }
    pub fn get_cash_and_real_positions(&self) -> &HashMap<InstrumentId, Position> {
        &self.assets
    }

    pub fn total_assets(&self, system: &FinancialSystem) -> f64 {
        let cash_and_real: f64 = self.assets
            .iter()
            .filter_map(|(id, pos)| {
                let inst = system.instruments.get(id)?;
                match &inst.instrument_type {
                    InstrumentType::Cash(_) | InstrumentType::RealAsset(_) => {
                        let price = get_market_price(inst, &system.exchange)
                            .unwrap_or(pos.book_value_per_unit);
                        Some(price.to_f64() * pos.quantity)
                    }
                    _ => None, // Securities are tracked in CSD
                }
            })
            .sum();

        let securities: f64 = system.clearing_house.csd
            .get_all_positions(&self.agent_id)
            .iter()
            .filter_map(|(id, qty)| {
                let inst = system.instruments.get(id)?;
                let price = get_market_price(inst, &system.exchange)
                    .unwrap_or_else(|| inst.face_value().unwrap_or(Money::from(1000)));
                Some(price.to_f64() * qty)
            })
            .sum();

        cash_and_real + securities
    }

    pub fn total_liabilities(&self, system: &FinancialSystem) -> f64 {
        self.liabilities
            .iter()
            .map(|(id, pos)| {
                let inst = system.instruments.get(id).unwrap();
                let price = get_market_price(inst, &system.exchange)
                    .unwrap_or(pos.book_value_per_unit);
                price.to_f64() * pos.quantity
            })
            .sum()
    }

    pub fn calculate_rwa(&self, system: &FinancialSystem) -> f64 {
        let mut rwa = 0.0;

        for (id, pos) in &self.assets {
            if let Some(inst) = system.instruments.get(id) {
                let risk_weight = match &inst.instrument_type {
                    InstrumentType::Cash(c) => match c.cash_type {
                        CashType::CentralBankReserves => 0.0,
                        CashType::TreasuryGeneralAccount => 0.0,
                        _ => 0.2, // Regular deposits have some risk
                    },
                    InstrumentType::RealAsset(_) => 1.0, // Full risk weight
                    _ => continue, // Securities handled below
                };
                
                let market_value = get_market_price(inst, &system.exchange)
                    .unwrap_or(pos.book_value_per_unit)
                    .to_f64() * pos.quantity;
                rwa += market_value * risk_weight;
            }
        }

        let positions = system.clearing_house.csd.get_all_positions(&self.agent_id);
        for (id, qty) in positions {
            if let Some(inst) = system.instruments.get(&id) {
                let risk_weight = match &inst.instrument_type {
                    InstrumentType::Bond(b) => match b.bond_type {
                        BondType::Government => 0.0,
                        BondType::Corporate => {
                            match b.rating {
                                CreditRating::AAA | CreditRating::AA => 0.2,
                                CreditRating::A => 0.5,
                                CreditRating::BBB => 1.0,
                                _ => 1.5,
                            }
                        },
                        BondType::InterbankLoan => 0.2,
                    },
                    InstrumentType::Equity(_) => 1.0,
                    InstrumentType::StructuredTranche(t) => {
                        match t.rating {
                            CreditRating::AAA => 0.2,
                            CreditRating::AA => 0.5,
                            CreditRating::A => 1.0,
                            _ => 2.0,
                        }
                    },
                    _ => 1.0,
                };

                let market_value = get_market_price(inst, &system.exchange)
                    .unwrap_or_else(|| inst.face_value().unwrap_or(Money::from(1000)))
                    .to_f64() * qty;
                rwa += market_value * risk_weight;
            }
        }

        rwa
    }

    pub fn net_worth(&self, system: &FinancialSystem) -> f64 {
        self.total_assets(system) - self.total_liabilities(system)
    }

    pub fn leverage_ratio(&self, system: &FinancialSystem) -> f64 {
        let total_assets = self.total_assets(system);
        let net_worth = self.net_worth(system);
        if net_worth <= 0.0 { f64::INFINITY } else { total_assets / net_worth }
    }
}
