use crate::prelude::*;
use crate::types::money::Money;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
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
    // For cash instruments and real assets (not securities)
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
                let total: Money = goods.values()
                    .map(|item| item.unit_cost * item.quantity)
                    .sum();
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

    /// Get the full asset position including securities from CSD
    pub fn get_all_asset_positions(&self, system: &FinancialSystem) -> HashMap<InstrumentId, Position> {
        let mut all_positions = self.assets.clone();
        
        // Add securities positions from CSD
        if let Some(csd_positions) = system.clearing_house.csd.custody_accounts.get(&self.agent_id) {
            for (inst_id, holding) in &csd_positions.holdings {
                let quantity = holding.total_position();
                if quantity > 1e-9 {
                    // Get the instrument to determine book value
                    if let Some(inst) = system.instruments.get(inst_id) {
                        let book_value = inst.face_value().unwrap_or(Money::from(1000 as i64));
                        all_positions.entry(*inst_id).or_insert(Position {
                            quantity,
                            book_value_per_unit: book_value,
                            cost_basis_per_unit: book_value,
                        });
                    }
                }
            }
        }
        
        all_positions
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
                    if &c.issuer == bank_id && matches!(c.cash_type, CashType::DemandDeposit | CashType::SavingsDeposit) {
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

    pub fn total_assets(&self, system: &FinancialSystem) -> f64 {
        let all_positions = self.get_all_asset_positions(system);
        
        all_positions
            .iter()
            .map(|(id, pos)| {
                let inst = system.instruments.get(id).unwrap();
                let price = get_market_price(inst, &system.exchange).unwrap_or(pos.book_value_per_unit);
                price.to_f64() * pos.quantity
            })
            .sum()
    }

    pub fn total_liabilities(&self, system: &FinancialSystem) -> f64 {
        self.liabilities
            .iter()
            .map(|(id, pos)| {
                let inst = system.instruments.get(id).unwrap();
                let price = get_market_price(inst, &system.exchange).unwrap_or(pos.book_value_per_unit);
                price.to_f64() * pos.quantity
            })
            .sum()
    }

    pub fn net_worth(&self, system: &FinancialSystem) -> f64 {
        self.total_assets(system) - self.total_liabilities(system)
    }

    pub fn leverage_ratio(&self, system: &FinancialSystem) -> f64 {
        let total_assets = self.total_assets(system);
        let net_worth = self.net_worth(system);
        if net_worth <= 0.0 {
            f64::INFINITY
        } else {
            total_assets / net_worth
        }
    }

    pub fn capital_adequacy_ratio(&self, system: &FinancialSystem) -> f64 {
        let net_worth = self.net_worth(system);
        let risk_weighted_assets = self.calculate_rwa(system);
        if risk_weighted_assets <= 1e-6 {
            1.0
        } else {
            net_worth / risk_weighted_assets
        }
    }

    fn calculate_rwa(&self, system: &FinancialSystem) -> f64 {
        let all_positions = self.get_all_asset_positions(system);
        
        all_positions.iter().map(|(id, pos)| {
            let inst = system.instruments.get(id).expect("Instrument must exist if on BS");
            let risk_weight = self.get_risk_weight(inst, system);
            
            let price = get_market_price(inst, &system.exchange).unwrap_or(pos.book_value_per_unit);
            let exposure = price.to_f64() * pos.quantity;
            exposure * risk_weight
        }).sum()
    }

    fn get_risk_weight(&self, inst: &Instrument, system: &FinancialSystem) -> f64 {
        match &inst.instrument_type {
            InstrumentType::Cash(c) => {
                if c.issuer == system.central_bank.id {
                    0.0
                } else {
                    0.2
                }
            },
            InstrumentType::Bond(b) => {
                match b.bond_type {
                    BondType::Government => 0.0,
                    BondType::InterbankLoan => 0.2,
                    BondType::Corporate => {
                        match b.rating {
                            CreditRating::AAA | CreditRating::AA => 0.2,
                            CreditRating::A => 0.5,
                            CreditRating::BBB => 1.0,
                            _ => 1.5,
                        }
                    }
                }
            },
            InstrumentType::Equity(_) => 1.0,
            InstrumentType::RealAsset(_) => 1.0,
            InstrumentType::Derivative(_) => 1.0,
            InstrumentType::StructuredTranche(st) => {
                 match st.rating {
                    CreditRating::AAA => 0.2,
                    CreditRating::AA => 0.5,
                    CreditRating::A => 0.8,
                    _ => 1.5,
                 }
            },
            InstrumentType::Repo(r) => {
                if self.agent_id == r.lender {
                    let collateral = system.instruments.get(&r.collateral_id);
                    if let Some(InstrumentType::Bond(b)) = collateral.map(|i| &i.instrument_type) {
                        if b.bond_type == BondType::Government {
                            return 0.0;
                        }
                    }
                    0.2
                } else {
                    0.0
                }
            },
        }
    }
}