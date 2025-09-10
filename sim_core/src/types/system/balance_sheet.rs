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
        InstrumentType::Debt(debt) => match debt {
            DebtInstrument::Loan(l) => Some(l.outstanding_principal),
            DebtInstrument::CreditLine(c) => Some(c.drawn_amount),
            DebtInstrument::TradeCredit(t) => Some(t.amount),
            _ => None,
        },
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
                    _ => continue,                       // Securities handled below
                };

                let market_value =
                    get_market_price(inst, &system.exchange).unwrap_or(pos.book_value_per_unit).to_f64() * pos.quantity;
                rwa += market_value * risk_weight;
            }
        }

        let positions = system.clearing_house.csd.get_all_positions(&self.agent_id);
        for (id, qty) in positions {
            if let Some(inst) = system.instruments.get(&id) {
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
                    InstrumentType::Debt(DebtInstrument::Loan(l)) => match l.credit_rating {
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
