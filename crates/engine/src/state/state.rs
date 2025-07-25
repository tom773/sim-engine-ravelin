use shared::*;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use rand::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimState {
    pub ticknum: u32,
    pub consumers: Vec<Consumer>,
    pub firms: Vec<Firm>,
    pub financial_system: FinancialSystem,
    pub config: SimConfig,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimConfig {
    pub iterations: u32,
    pub consumer_count: u32,
    pub firm_count: u32,
    pub scenario: String,
}
impl Default for SimConfig {
    fn default() -> Self {
        Self {
            iterations: 5,
            consumer_count: 2,
            firm_count: 1,
            scenario: "default".to_string(),
        }
    }
}
impl Default for SimState {
    fn default() -> Self {
        Self {
            ticknum: 0,
            consumers: Vec::new(),
            firms: Vec::new(),
            financial_system: FinancialSystem::default(),
            config: SimConfig::default(),
        }
    }
}

pub fn initialize_economy(config: &SimConfig, rng: &mut StdRng) -> SimState {
    let mut financial_system = FinancialSystem {
        instruments: HashMap::new(),
        balance_sheets: HashMap::new(),
        central_bank: CentralBank {
            id: AgentId(Uuid::new_v4()),
            policy_rate: 500.0,
            reserve_requirement: 0.10,
        },
        commercial_banks: HashMap::new(),
    };

    // Create balance sheet for central bank
    let cb_id = financial_system.central_bank.id.clone();
    financial_system.balance_sheets.insert(
        cb_id.clone(),
        BalanceSheet::new(cb_id.clone())
    );

    // Create commercial banks
    let mut bank_ids = Vec::new();
    for i in 0..3 {
        let bank_agent_id = AgentId(Uuid::new_v4());
        let bank = Bank {
            id: bank_agent_id.clone(),
            name: format!("Bank {}", i),
            deposit_spread: -200.0,
            lending_spread: 500.0,
        };
        
        // Create balance sheet in financial system
        financial_system.balance_sheets.insert(
            bank_agent_id.clone(),
            BalanceSheet::new(bank_agent_id.clone())
        );
        
        // Create reserves
        let reserves = FinancialInstrument {
            id: InstrumentId(Uuid::new_v4()),
            instrument_type: InstrumentType::CentralBankReserves,
            creditor: bank_agent_id.clone(),
            debtor: cb_id.clone(),
            principal: 1_000_000.0,
            interest_rate: financial_system.central_bank.policy_rate,
            maturity: None,
            originated_date: 0,
        };
        
        financial_system.commercial_banks.insert(bank_agent_id.clone(), bank);
        financial_system.create_instrument(reserves).unwrap();
        bank_ids.push(bank_agent_id);
    }

    let ml_model: Option<Box<dyn DecisionModel>> = None;
    // Create consumers
    let mut consumers = Vec::new();
    for _ in 0..config.consumer_count {
        let agent_id = AgentId(Uuid::new_v4());
        let primary_bank = bank_ids[rng.random_range(0..bank_ids.len())].clone();
        
        let decision_model: Box<dyn DecisionModel> = ml_model.as_ref()
        .map(|m| dyn_clone::clone_box(&**m) as Box<dyn DecisionModel>)
        .unwrap_or_else(|| Box::new(BasicDecisionModel { wdf: 0.5 }) as Box<dyn DecisionModel>);

        let consumer = Consumer::new(
            rng.random_range(18..65),
            agent_id.clone(),
            primary_bank.clone(),
            decision_model,
        );
        
        // Create balance sheet for consumer
        financial_system.balance_sheets.insert(
            agent_id.clone(),
            BalanceSheet::new(agent_id.clone())
        );
        
        // Create initial deposit
        let initial_deposit = FinancialInstrument {
            id: InstrumentId(Uuid::new_v4()),
            instrument_type: InstrumentType::DemandDeposit,
            creditor: agent_id.clone(),
            debtor: primary_bank.clone(),
            principal: rng.random_range(1000.0..5000.0),
            interest_rate: 300.0,
            maturity: None,
            originated_date: 0,
        };
        
        // Create credit line
        let credit_line = FinancialInstrument {
            id: InstrumentId(Uuid::new_v4()),
            instrument_type: InstrumentType::Loan{
                loan_type: LoanType::CreditCard, 
                collateral: None
            },
            creditor: primary_bank.clone(),
            debtor: agent_id.clone(),
            principal: rng.random_range(1000.0..5000.0),
            interest_rate: 1700.0,
            maturity: None,
            originated_date: 0,
        };
        
        financial_system.create_instrument(initial_deposit).unwrap();
        financial_system.create_instrument(credit_line).unwrap();
        
        consumers.push(consumer);
    }

    // Create firms
    let mut firms = Vec::new();
    for i in 0..config.firm_count {
        let agent_id = AgentId(Uuid::new_v4());
        let primary_bank = bank_ids[rng.random_range(0..bank_ids.len())].clone();
        
        // Create balance sheet for firm
        financial_system.balance_sheets.insert(
            agent_id.clone(),
            BalanceSheet::new(agent_id.clone())
        );
        
        let firm = Firm::new(agent_id.clone(), primary_bank.clone(), format!("Firm {}", i));
        
        // Create working capital
        let working_capital = FinancialInstrument {
            id: InstrumentId(Uuid::new_v4()),
            instrument_type: InstrumentType::DemandDeposit,
            creditor: agent_id.clone(),
            debtor: primary_bank,
            principal: rng.random_range(10000.0..50000.0),
            interest_rate: 0.02,
            maturity: None,
            originated_date: 0,
        };
        
        financial_system.create_instrument(working_capital).unwrap();
        firms.push(firm);
    }

    SimState {
        ticknum: 0,
        consumers,
        firms,
        financial_system,
        config: config.clone(),
    }
}

pub trait AgentBalanceSheet {
    fn balance_sheet<'a>(&self, fs: &'a FinancialSystem) -> Option<&'a BalanceSheet>;
    fn balance_sheet_mut<'a>(&self, fs: &'a mut FinancialSystem) -> Option<&'a mut BalanceSheet>;
}

impl AgentBalanceSheet for AgentId {
    fn balance_sheet<'a>(&self, fs: &'a FinancialSystem) -> Option<&'a BalanceSheet> {
        fs.balance_sheets.get(self)
    }
    
    fn balance_sheet_mut<'a>(&self, fs: &'a mut FinancialSystem) -> Option<&'a mut BalanceSheet> {
        fs.balance_sheets.get_mut(self)
    }
}