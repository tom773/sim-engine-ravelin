use crate::scenario::{BankConfig, ConsumerConfig, FirmConfig};
use rand::prelude::*;
use sim_prelude::*;
use std::str::FromStr;

const STANDARD_BOND_FACE_VALUE: f64 = 1000.0;

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
        let cash = cash!(bank.id, 200_000.0, cb_id, self.state.current_date);
        self.state.financial_system.create_instrument(reserves).unwrap();
        self.state.financial_system.create_instrument(cash).unwrap();

        let government_id = self.state.financial_system.government.id;
        let policy_rate = self.state.financial_system.central_bank.policy_rate;

        for bond_conf in &config.initial_bonds {
            let tenor = Tenor::from_str(&bond_conf.tenor).unwrap();
            let maturity_date = tenor.add_to_date(self.state.current_date);
            let coupon_rate = policy_rate;
            let quantity = bond_conf.quantity as u64;

            let bond_instrument = FinancialInstrument {
                id: InstrumentId(uuid::Uuid::new_v4()),
                creditor: bank.id,
                debtor: government_id,

                principal: STANDARD_BOND_FACE_VALUE * quantity as f64,
                details: Box::new(BondDetails {
                    bond_type: BondType::Government,
                    coupon_rate,
                    face_value: STANDARD_BOND_FACE_VALUE,
                    maturity_date,
                    frequency: 2,
                    tenor,
                    quantity,
                }),
                originated_date: self.state.current_date,
                accrued_interest: 0.0,
                last_accrual_date: self.state.current_date,
            };
            
            self.state.financial_system.create_or_consolidate_instrument(bond_instrument).unwrap();
        }
        
        self.state.agents.banks.insert(bank.id, bank.clone());
        bank
    }

    pub fn create_consumer(&mut self, config: &ConsumerConfig, bank_id: AgentId, cb_id: AgentId) -> Consumer {
        let personality =
            *vec![PersonalityArchetype::Balanced, PersonalityArchetype::Saver, PersonalityArchetype::Spender]
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
