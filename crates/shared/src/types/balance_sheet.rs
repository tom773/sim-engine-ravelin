use crate::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub agent_id: AgentId,
    pub assets: HashMap<InstrumentId, FinancialInstrument>,
    pub liabilities: HashMap<InstrumentId, FinancialInstrument>,
    pub real_assets: HashMap<InstrumentId, RealAsset>,
}

impl BalanceSheet {
    pub fn new(owner: AgentId) -> Self {
        Self {
            agent_id: owner,
            assets: HashMap::new(),
            liabilities: HashMap::new(),
            real_assets: HashMap::new(),
        }
    }

    pub fn liquid_assets(&self) -> f64 {
        self.assets.values()
            .filter(|inst| matches!(
                inst.instrument_type, 
                InstrumentType::Cash | InstrumentType::DemandDeposit
            ))
            .map(|inst| inst.principal)
            .sum()
    }

    pub fn deposits_at_bank(&self, bank_id: &AgentId) -> f64 {
        self.assets.values()
            .filter(|inst| {
                inst.debtor == *bank_id && 
                matches!(inst.instrument_type, 
                    InstrumentType::DemandDeposit | InstrumentType::SavingsDeposit { .. }
                )
            })
            .map(|inst| inst.principal)
            .sum()
    }

    pub fn total_assets(&self) -> f64 {
        let financial = self.assets.values()
            .map(|inst| inst.principal)
            .sum::<f64>();
        let real = self.real_assets.values()
            .map(|asset| asset.market_value)
            .sum::<f64>();
        financial + real
    }

    pub fn total_liabilities(&self) -> f64 {
        self.liabilities.values()
            .map(|inst| inst.principal)
            .sum()
    }

    pub fn net_worth(&self) -> f64 {
        self.total_assets() - self.total_liabilities()
    }
}

pub trait BalanceSheetQuery {
    fn get_total_assets(&self, agent_id: &AgentId) -> f64;
    fn get_total_liabilities(&self, agent_id: &AgentId) -> f64;
    fn get_liquid_assets(&self, agent_id: &AgentId) -> f64;
    fn get_deposits_at_bank(&self, agent_id: &AgentId, bank_id: &AgentId) -> f64;
}

impl BalanceSheetQuery for FinancialSystem {
    fn get_total_assets(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id)
            .map(|bs| bs.total_assets())
            .unwrap_or(0.0)
    }
    fn get_total_liabilities(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id)
            .map(|bs| bs.total_liabilities())
            .unwrap_or(0.0)
    }
    fn get_liquid_assets(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id)
            .map(|bs| bs.liquid_assets())
            .unwrap_or(0.0)
    }
    fn get_deposits_at_bank(&self, agent_id: &AgentId, bank_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id)
            .map(|bs| bs.deposits_at_bank(bank_id))
            .unwrap_or(0.0)
    } 
}