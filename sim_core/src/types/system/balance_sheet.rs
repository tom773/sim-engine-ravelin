use crate::prelude::*;
use crate::types::money::Money;
use rust_decimal::prelude::*;
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

impl Position {
    pub fn par(quantity: f64) -> Self {
        Self { quantity, book_value_per_unit: Money::from(1 as i64), cost_basis_per_unit: Money::from(1 as i64) }
    }
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
impl IncomeStatement {
    pub fn add_revenue(&mut self, x: f64) {
        self.revenue += Money::from_f64(x.max(0.0)).unwrap();
        self.recompute();
    }
    pub fn add_cogs(&mut self, x: f64) {
        self.cost_of_goods_sold += Money::from_f64(x.max(0.0)).unwrap();
        self.recompute();
    }
    pub fn add_opex(&mut self, x: f64) {
        self.operating_expenses += Money::from_f64(x.max(0.0)).unwrap();
        self.recompute();
    }
    pub fn add_interest_income(&mut self, x: f64) {
        self.interest_income += Money::from_f64(x.max(0.0)).unwrap();
        self.recompute();
    }
    pub fn add_interest_expense(&mut self, x: f64) {
        self.interest_expense += Money::from_f64(x.max(0.0)).unwrap();
        self.recompute();
    }
    fn recompute(&mut self) {
        self.net_income = self.revenue - self.cost_of_goods_sold - self.operating_expenses + self.interest_income
            - self.interest_expense;
    }
    pub fn gross_margin_rate(&self) -> Decimal {
        if self.revenue.to_f64().abs() < 1e-9 {
            Decimal::from_f64(0.0_f64).unwrap()
        } else {
            (self.revenue - self.cost_of_goods_sold) / self.revenue
        }
    }
}

fn get_market_price(inst: &Instrument, exchange: &Exchange) -> Option<Money> {
    let symbol = exchange.inst_to_symbol.get(&inst.id)?;

    if let Some(market) = exchange.markets.get(symbol) {
        match market {
            MarketType::Financial(m) => m.book.mid_price(),
            MarketType::Goods(m) => m.book.mid_price(),
            MarketType::Labour(_) => None,
        }
    } else {
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
            InstrumentType::Debt(debt) => match debt {
                DebtInstrument::Loan(l) => Some(l.outstanding_principal),
                DebtInstrument::CreditLine(c) => Some(c.drawn_amount),
                DebtInstrument::TradeCredit(t) => Some(t.amount),
                _ => None,
            },
            _ => None,
        }
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
                let inst = system.instruments.instruments.get(id)?;
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
                let inst = system.instruments.instruments.get(id)?;
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
                let inst = system.instruments.instruments.get(id)?;
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

    pub fn calculate_rwa(&self, system: &FinancialSystem) -> f64 {
        let mut rwa = 0.0;

        for (id, pos) in &self.assets {
            if let Some(inst) = system.instruments.instruments.get(id) {
                let risk_weight = match &inst.instrument_type {
                    InstrumentType::Cash(c) => match c.cash_type {
                        CashType::CentralBankReserves => 0.0,
                        CashType::TreasuryGeneralAccount => 0.0,
                        _ => 0.2,
                    },
                    InstrumentType::RealAsset(_) => 1.0,
                    _ => continue,
                };

                let market_value =
                    get_market_price(inst, &system.exchange).unwrap_or(pos.book_value_per_unit).to_f64() * pos.quantity;
                rwa += market_value * risk_weight;
            }
        }

        let positions = system.clearing_house.csd.get_all_positions(&self.agent_id);
        for (id, qty) in positions {
            if let Some(inst) = system.instruments.instruments.get(&id) {
                let risk_weight = match &inst.instrument_type {
                    InstrumentType::Cash(c) => match c.cash_type {
                        CashType::CentralBankReserves => 0.0,
                        CashType::TreasuryGeneralAccount => 0.0,
                        _ => 0.2,
                    },
                    InstrumentType::Debt(DebtInstrument::Bond(b)) => match b.bond_type {
                        BondType::Government => 0.0,
                        BondType::Corporate => match b.rating {
                            CreditRating::Corporate(SpCreditRating::AAA)
                            | CreditRating::Corporate(SpCreditRating::AA) => 0.2,
                            CreditRating::Corporate(SpCreditRating::A) => 0.5,
                            CreditRating::Corporate(SpCreditRating::BBB) => 1.0,
                            _ => 1.5,
                        },
                        BondType::InterbankLoan => 0.2,
                    },
                    InstrumentType::Debt(DebtInstrument::Loan(l)) => match l.rating {
                        Some(CreditRating::Corporate(SpCreditRating::AAA))
                        | Some(CreditRating::Corporate(SpCreditRating::AA)) => 0.5,
                        Some(CreditRating::Corporate(SpCreditRating::A))
                        | Some(CreditRating::Corporate(SpCreditRating::BBB)) => 1.0,
                        Some(CreditRating::Consumer(ConsumerCreditRating::Prime)) => 0.75,
                        Some(CreditRating::Consumer(ConsumerCreditRating::NearPrime)) => 1.0,
                        Some(CreditRating::Consumer(ConsumerCreditRating::Subprime)) => 1.5,
                        Some(CreditRating::Consumer(ConsumerCreditRating::DeepSubprime)) => 2.0,
                        _ => 1.5,
                    },
                    InstrumentType::StructuredTranche(t) => match t.rating {
                        CreditRating::Corporate(SpCreditRating::AAA)
                        | CreditRating::Government(SpCreditRating::AAA) => 0.2,
                        CreditRating::Corporate(SpCreditRating::AA) | CreditRating::Government(SpCreditRating::AA) => {
                            0.5
                        }
                        CreditRating::Corporate(SpCreditRating::A) | CreditRating::Government(SpCreditRating::A) => 1.0,
                        _ => 2.0,
                    },
                    InstrumentType::RealAsset(_) => 1.0,
                    _ => 1.0,
                };

                let market_value = get_market_price(inst, &system.exchange)
                    .unwrap_or_else(|| inst.face_value().unwrap_or(Money::from(1000)))
                    .to_f64()
                    * qty;
                rwa += market_value * risk_weight;
            }
        }

        rwa
    }
}
