use chrono::Datelike;
use rand::prelude::*;
use sim_prelude::*;
use std::str::FromStr;
use crate::scenario::{BankConfig, ConsumerConfig, FirmConfig};

pub struct AgentFactory<'a> {
    pub state: &'a mut SimState,
    pub rng: &'a mut ThreadRng,
}

impl<'a> AgentFactory<'a> {
    pub fn new(state: &'a mut SimState, rng: &'a mut ThreadRng) -> Self {
        Self { state, rng }
    }

    pub fn create_bank(&mut self, config: &BankConfig, cb_id: AgentId) -> Bank {
        let bank = Bank::new(config.name.clone(), 200.0, -70.0);
        self.state.financial_system.balance_sheets.insert(bank.id, BalanceSheet::new(bank.id));

        let reserves = reserves!(bank.id, cb_id, config.initial_reserves, self.state.current_date);
        self.state.financial_system.create_instrument(reserves).unwrap();

        for bond_conf in &config.initial_bonds {
            let tenor = Tenor::from_str(&bond_conf.tenor).unwrap();
            let years_to_maturity = match tenor {
                Tenor::T2Y => 2,
                Tenor::T5Y => 5,
                Tenor::T10Y => 10,
                Tenor::T30Y => 30,
            };
            let maturity_date = self
                .state
                .current_date
                .with_year(self.state.current_date.year() + years_to_maturity)
                .expect("Failed to calculate maturity date");

            let bond = bond!(
                bank.id,
                cb_id,
                bond_conf.face_value,
                0.04,
                maturity_date,
                bond_conf.face_value,
                BondType::Government,
                2,
                self.state.current_date
            );
            self.state.financial_system.create_instrument(bond).unwrap();
        }

        self.state.agents.banks.insert(bank.id, bank.clone());
        bank
    }

    pub fn create_consumer(&mut self, config: &ConsumerConfig, bank_id: AgentId, cb_id: AgentId) -> Consumer {
        let personality = *vec![PersonalityArchetype::Balanced, PersonalityArchetype::Saver, PersonalityArchetype::Spender]
            .choose(self.rng)
            .unwrap();
        let mut consumer = Consumer::new(self.rng.random_range(25..65), bank_id, personality);
        consumer.income = config.income;

        self.state.financial_system.balance_sheets.insert(consumer.id, BalanceSheet::new(consumer.id));
        let cash = cash!(consumer.id, config.initial_cash, cb_id, self.state.current_date);
        self.state.financial_system.create_instrument(cash).unwrap();

        self.state.agents.consumers.insert(consumer.id, consumer.clone());
        consumer
    }

    pub fn create_firm(&mut self, config: &FirmConfig, bank_id: AgentId, cb_id: AgentId) -> Firm {
        let recipe_id = self.state.financial_system.goods.get_recipe_id_by_name(&config.recipe_name);
        let firm = Firm::new(bank_id, config.name.clone(), recipe_id);

        self.state.financial_system.balance_sheets.insert(firm.id, BalanceSheet::new(firm.id));
        let cash = cash!(firm.id, config.initial_cash, cb_id, self.state.current_date);
        self.state.financial_system.create_instrument(cash).unwrap();

        let inventory_to_add: Vec<_> = config
            .initial_inventory
            .iter()
            .map(|inv_conf| {
                let good_id = self.state.financial_system.goods.get_good_id_by_slug(&inv_conf.good_slug).unwrap();
                (good_id, inv_conf.quantity, inv_conf.unit_cost)
            })
            .collect();

        let bs = self.state.financial_system.balance_sheets.get_mut(&firm.id).unwrap();
        for (good_id, quantity, unit_cost) in inventory_to_add {
            bs.add_to_inventory(&good_id, quantity, unit_cost);
        }

        self.state.agents.firms.insert(firm.id, firm.clone());
        firm
    }
}