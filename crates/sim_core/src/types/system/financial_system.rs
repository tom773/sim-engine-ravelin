use crate::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::HashMap;
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
        let government = Government {
            id: AgentId(Uuid::new_v4()),
            tax_rates: TaxRates::default(),
            spending_targets: SpendingTargets::default(),
            debt_ceiling: Some(1_000_000_000.0),
            fiscal_policy: FiscalPolicy::default(),
        };
        let central_bank = CentralBank {
            id: AgentId(Uuid::new_v4()),
            policy_rate_bps: 425.0,
            reserve_requirement: 0.1,
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
                date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                yields: HashMap::new(),
            },
        }
    }
}

impl FinancialSystem {
    pub fn update_yield_curve(&mut self, date: NaiveDate) {
        let mut yields = HashMap::new();
        for (market_id, market) in &self.exchange.financial_markets {
            if let FinancialMarketId::Treasury { tenor } = market_id {
                if let (Some(bid), Some(ask)) = (market.order_book.best_bid(), market.order_book.best_ask()) {
                    let price = (bid.price + ask.price) / 2.0;
                    if let Some(ytm) = market.calculate_ytm_with_price(self, price) {
                        yields.insert(*tenor, ytm);
                    }
                }
            }
        }
        self.yield_curve = YieldCurve { date, yields };
    }
}