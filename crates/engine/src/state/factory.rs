use rand::prelude::*;
use shared::{*, types::finance::{BondType, CreditRating}};
use uuid::Uuid;

use crate::{state::scenario::{BankConfig, ConsumerConfig, FirmConfig}, *};

pub struct AgentFactory<'a> {
    pub ss: &'a mut SimState,
    pub rng: &'a mut StdRng,
}

impl<'a> AgentFactory<'a> {
    pub fn new(ss: &'a mut SimState, rng: &'a mut StdRng) -> Self {
        Self { ss, rng }
    }

    pub fn create_bank_from_config(&mut self, config: &BankConfig, cb_id: &AgentId) -> Bank {
        let bank = Bank::new(config.name.clone(), 250.0, 50.0);
        self.ss.financial_system.balance_sheets.insert(bank.id, BalanceSheet::new(bank.id));

        let reserves = reserves!(bank.id, *cb_id, config.initial_reserves, 4.0, 0);
        self.ss.financial_system.create_instrument(reserves).unwrap();

        for bond_conf in &config.initial_bonds {
            let bond = bond!(bank.id, *cb_id, bond_conf.face_value, 2.5, 365 * 10, bond_conf.face_value, CreditRating::AAA, 0, BondType::Government);
            self.ss.financial_system.create_instrument(bond).unwrap();
        }

        self.ss.financial_system.commercial_banks.insert(bank.id, bank.clone());
        bank
    }

    pub fn create_consumer_from_config(&mut self, config: &ConsumerConfig, bank_id: AgentId, cb_id: &AgentId) -> Consumer {
        let agent_id = AgentId(Uuid::new_v4());
        self.ss.financial_system.balance_sheets.insert(agent_id, BalanceSheet::new(agent_id));

        let cash = cash!(agent_id, config.initial_cash, *cb_id, 0);
        self.ss.financial_system.create_or_consolidate_instrument(cash).unwrap();

        let decision_model = Box::new(BasicDecisionModel{});
        let mut c = Consumer::new(35, agent_id, bank_id, decision_model);
        c.income = config.income / 52.0; // Convert annual income to weekly

        self.ss.consumers.push(c.clone());
        c
    }

    pub fn create_firm_from_config(&mut self, config: &FirmConfig, bank_id: AgentId, recipe_id: Option<RecipeId>, employee_id: &AgentId, cb_id: &AgentId) -> Firm {
        let firm_id = AgentId(Uuid::new_v4());
        self.ss.financial_system.balance_sheets.insert(firm_id, BalanceSheet::new(firm_id));

        let cash = cash!(firm_id, config.initial_cash, *cb_id, 0);
        self.ss.financial_system.create_or_consolidate_instrument(cash).unwrap();

        let bs = self.ss.financial_system.balance_sheets.get_mut(&firm_id).unwrap();
        for inv_conf in &config.initial_inventory {
            let good_id = self.ss.financial_system.goods.get_good_id_by_slug(&inv_conf.good_slug).unwrap();
            bs.add_to_inventory(&good_id, inv_conf.quantity, inv_conf.unit_cost);
        }

        let mut f = Firm::new(firm_id, bank_id, config.name.clone(), recipe_id);
        f.employees.push(*employee_id);
        f.wage_rate = 20.0;
        
        self.ss.firms.push(f.clone());
        f
    }
}