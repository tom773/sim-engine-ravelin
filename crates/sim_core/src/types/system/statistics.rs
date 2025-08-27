use super::*;
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OvernightRates {
    pub effr: Option<f64>,
    pub sofr: Option<f64>,
    pub iorb: f64,
    pub discount_rate: f64,
    pub overnight_rrp: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoreEconomicStats {
    pub gdp: f64,
    pub consumption: f64,
    pub cpi: f64,
    pub ppi: f64,
    pub unemployment_rate: f64,
    pub labor_force_participation: f64,
    pub job_openings: f64,
    pub capacity_utilization: f64,
    pub industrial_production: f64,
    pub credit_growth: f64,
    pub household_debt: f64,
    pub corporate_debt: f64,
    pub government_debt: f64,
    pub bank_liabilities: f64,
}

impl FinancialStatistics for FinancialSystem {
    fn m0(&self) -> f64 {
        self.balance_sheets.values().map(|bs| {
            bs.assets.values()
                .filter(|inst| inst.details.as_any().is::<CentralBankReservesDetails>())
                .map(|inst| inst.principal).sum::<f64>()
        }).sum()
    }

    fn m1(&self, bank_ids: &HashSet<AgentId>) -> f64 {
        self.balance_sheets.values()
            .filter(|bs| !bank_ids.contains(&bs.agent_id) && bs.agent_id != self.central_bank.id)
            .map(|bs| {
                bs.assets.values()
                    .filter(|inst| inst.details.as_any().is::<CashDetails>() || inst.details.as_any().is::<DemandDepositDetails>())
                    .map(|inst| inst.principal).sum::<f64>()
            }).sum()
    }

    fn m2(&self, bank_ids: &HashSet<AgentId>) -> f64 {
        let m1 = self.m1(bank_ids);
        let savings_deposits: f64 = self.balance_sheets.values()
            .filter(|bs| !bank_ids.contains(&bs.agent_id) && bs.agent_id != self.central_bank.id)
            .map(|bs| {
                bs.assets.values()
                    .filter(|inst| inst.details.as_any().is::<SavingsDepositDetails>())
                    .map(|inst| inst.principal).sum::<f64>()
            }).sum();
        m1 + savings_deposits
    }

    fn all_bank_assets(&self, bank_ids: &HashSet<AgentId>) -> f64 {
        self.balance_sheets.values().filter(|bs| bank_ids.contains(&bs.agent_id)).map(|bs| bs.total_assets()).sum()
    }

    fn all_bank_reserves(&self, bank_ids: &HashSet<AgentId>) -> f64 {
        self.balance_sheets.values().filter(|bs| bank_ids.contains(&bs.agent_id)).map(|bs| {
            bs.assets.values()
                .filter(|inst| inst.details.as_any().is::<CentralBankReservesDetails>())
                .map(|inst| inst.principal).sum::<f64>()
        }).sum()
    }

    fn all_bank_deposits(&self, bank_ids: &HashSet<AgentId>) -> f64 {
        self.balance_sheets.values().filter(|bs| bank_ids.contains(&bs.agent_id)).map(|bs| {
            bs.liabilities.values()
                .filter(|inst| inst.details.as_any().is::<DemandDepositDetails>() || inst.details.as_any().is::<SavingsDepositDetails>())
                .map(|inst| inst.principal).sum::<f64>()
        }).sum()
    }

    fn currency_in_circulation(&self, cb_id: AgentId) -> f64 {
        self.balance_sheets.get(&cb_id).map_or(0.0, |bs| {
            bs.liabilities.values()
                .filter(|inst| inst.details.as_any().is::<CashDetails>())
                .map(|inst| inst.principal).sum()
        })
    }
}

impl FinancialSystem {
    pub fn calculate_overnight_rates(&self) -> OvernightRates {
        let policy_rate_bps = self.central_bank.policy_rate_bps;

        let calculate_rate = |market_id: FinancialMarketId| -> Option<f64> {
            self.exchange.financial_markets.get(&market_id)
                .and_then(|market| market.last_or_mid())
                .map(|price| {
                    let daily_rate = market_id.price_to_daily_rate(price);
                    market_id.daily_rate_to_annual_bps(daily_rate)
                })
        };

        let effr = calculate_rate(FinancialMarketId::FederalFundsOvernight);
        let sofr = calculate_rate(FinancialMarketId::TreasuryRepoOvernight);

        let iorb = policy_rate_bps + 15.0;
        let discount_rate = policy_rate_bps + 25.0;
        let overnight_rrp = policy_rate_bps;

        OvernightRates { effr, sofr, iorb, discount_rate, overnight_rrp }
    }
}