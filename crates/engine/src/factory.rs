use crate::scenario::{AssetConfig, BankConfig, ConsumerConfig, FirmConfig, LiabilityConfig};
use rand::prelude::*;
use sim_core::*;
use std::{collections::HashMap, str::FromStr};

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

        for asset in &config.initial_assets {
            self.create_asset_for_agent(bank.id, asset, cb_id, &HashMap::new())
                .unwrap_or_else(|e| println!("[ERROR] Failed to create asset for bank {}: {}", bank.id, e));
        }

        for liability in &config.initial_liabilities {
            self.create_liability_for_agent(bank.id, liability, cb_id, &HashMap::new())
                .unwrap_or_else(|e| println!("[ERROR] Failed to create liability for bank {}: {}", bank.id, e));
        }

        self.state.agents.banks.insert(bank.id, bank.clone());
        bank
    }

    pub fn create_consumer(
        &mut self, config: &ConsumerConfig, bank_id: AgentId, cb_id: AgentId, agent_ids: &HashMap<String, AgentId>,
    ) -> Consumer {
        let personality =
            *vec![PersonalityArchetype::Balanced, PersonalityArchetype::Saver, PersonalityArchetype::Spender]
                .choose(self.rng)
                .unwrap();
        let mut consumer = Consumer::new(self.rng.random_range(25..65), bank_id, personality);
        consumer.income = config.income;

        self.state.financial_system.balance_sheets.insert(consumer.id, BalanceSheet::new(consumer.id));

        for asset in &config.initial_assets {
            self.create_asset_for_agent(consumer.id, asset, cb_id, agent_ids)
                .unwrap_or_else(|e| println!("[ERROR] Failed to create asset for consumer {}: {}", consumer.id, e));
        }

        for liability in &config.initial_liabilities {
            self.create_liability_for_agent(consumer.id, liability, cb_id, agent_ids)
                .unwrap_or_else(|e| println!("[ERROR] Failed to create liability for consumer {}: {}", consumer.id, e));
        }

        self.state.agents.consumers.insert(consumer.id, consumer.clone());
        consumer
    }

    pub fn create_firm(
        &mut self, config: &FirmConfig, bank_id: AgentId, cb_id: AgentId, agent_ids: &HashMap<String, AgentId>,
    ) -> Firm {
        let recipe_id =
            config.recipe_name.as_ref().and_then(|name| self.state.financial_system.goods.get_recipe_id_by_name(name));

        let mut firm = Firm::new(bank_id, config.name.clone(), recipe_id, 25.0);

        if let Some(markup) = config.desired_markup {
            firm.desired_markup = markup;
        }

        self.state.financial_system.balance_sheets.insert(firm.id, BalanceSheet::new(firm.id));

        for asset in &config.initial_assets {
            match asset {
                AssetConfig::Inventory { good_slug, quantity, unit_cost } => {
                    if let Some(good_id) = self.state.financial_system.goods.get_good_id_by_slug(good_slug) {
                        let bs = self.state.financial_system.balance_sheets.get_mut(&firm.id).unwrap();
                        bs.add_to_inventory(&good_id, *quantity, *unit_cost);
                    } else {
                        println!("[ERROR] Unknown good slug for firm {}: {}", firm.id, good_slug);
                    }
                }
                _ => {
                    self.create_asset_for_agent(firm.id, asset, cb_id, agent_ids)
                        .unwrap_or_else(|e| println!("[ERROR] Failed to create asset for firm {}: {}", firm.id, e));
                }
            }
        }

        for liability in &config.initial_liabilities {
            self.create_liability_for_agent(firm.id, liability, cb_id, agent_ids)
                .unwrap_or_else(|e| println!("[ERROR] Failed to create liability for firm {}: {}", firm.id, e));
        }

        self.state.agents.firms.insert(firm.id, firm.clone());
        firm
    }

    fn create_asset_for_agent(
        &mut self, agent_id: AgentId, asset: &AssetConfig, cb_id: AgentId, agent_ids: &HashMap<String, AgentId>,
    ) -> Result<(), String> {
        match asset {
            AssetConfig::Cash { amount } => {
                let cash = cash!(agent_id, *amount, cb_id, self.state.current_date);
                self.state
                    .financial_system
                    .create_or_consolidate_instrument(cash)
                    .map_err(|e| format!("Failed to create cash: {}", e))?;
            }

            AssetConfig::Deposit { bank_id, amount } => {
                let bank_agent_id = agent_ids.get(bank_id).ok_or_else(|| format!("Bank ID {} not found", bank_id))?;

                let bank = self
                    .state
                    .agents
                    .get_bank(bank_agent_id)
                    .ok_or_else(|| format!("Bank not found for ID {}", bank_agent_id))?;
                let policy_rate_bps = self.state.financial_system.central_bank.policy_rate_bps;
                let deposit_rate_bps = (policy_rate_bps + bank.deposit_spread_bps).max(0.0);

                let deposit = deposit!(agent_id, *bank_agent_id, *amount, deposit_rate_bps, self.state.current_date);
                self.state
                    .financial_system
                    .create_or_consolidate_instrument(deposit)
                    .map_err(|e| format!("Failed to create deposit: {}", e))?;

                let reserves =
                    reserves!(*bank_agent_id, cb_id, *amount, self.state.current_date, policy_rate_bps + 15.0);
                self.state
                    .financial_system
                    .create_or_consolidate_instrument(reserves)
                    .map_err(|e| format!("Failed to create reserves for deposit: {}", e))?;
            }

            AssetConfig::Reserves { amount } => {
                let reserves = reserves!(
                    agent_id,
                    cb_id,
                    *amount,
                    self.state.current_date,
                    self.state.financial_system.central_bank.policy_rate_bps + 15.0
                );
                self.state
                    .financial_system
                    .create_or_consolidate_instrument(reserves)
                    .map_err(|e| format!("Failed to create reserves: {}", e))?;
            }

            AssetConfig::Bond { tenor, quantity } => {
                let tenor_enum = Tenor::from_str(tenor).map_err(|e| format!("Invalid tenor {}: {}", tenor, e))?;
                let maturity_date = tenor_enum.add_to_date(self.state.current_date);
                let coupon_rate_bps = self.state.financial_system.central_bank.policy_rate_bps;
                let government_id = self.state.financial_system.government.id;

                let bond_instrument = FinancialInstrument {
                    id: InstrumentId(uuid::Uuid::new_v4()),
                    creditor: agent_id,
                    debtor: government_id,
                    principal: STANDARD_BOND_FACE_VALUE * (*quantity as f64),
                    details: Box::new(BondDetails {
                        bond_type: BondType::Government,
                        coupon_rate_bps,
                        face_value: STANDARD_BOND_FACE_VALUE,
                        maturity_date,
                        frequency: 2,
                        tenor: tenor_enum,
                        quantity: *quantity as u64,
                        day_count: DayCount::ActAct,
                    }),
                    originated_date: self.state.current_date,
                    accrued_interest: 0.0,
                    last_accrual_date: self.state.current_date,
                };

                self.state
                    .financial_system
                    .create_or_consolidate_instrument(bond_instrument)
                    .map_err(|e| format!("Failed to create bond: {}", e))?;
            }

            AssetConfig::Inventory { .. } => {
                return Err("Inventory assets should be handled in create_firm only".to_string());
            }
        }
        Ok(())
    }

    fn create_liability_for_agent(
        &mut self, agent_id: AgentId, liability: &LiabilityConfig, _cb_id: AgentId,
        agent_ids: &HashMap<String, AgentId>,
    ) -> Result<(), String> {
        match liability {
            LiabilityConfig::Deposit { creditor_id, amount, rate_bps } => {
                let creditor_agent_id =
                    agent_ids.get(creditor_id).ok_or_else(|| format!("Creditor ID {} not found", creditor_id))?;

                let deposit_rate = rate_bps.unwrap_or_else(|| {
                    let policy_rate_bps = self.state.financial_system.central_bank.policy_rate_bps;
                    let bank = self.state.agents.get_bank(&agent_id);
                    let spread = bank.map(|b| b.deposit_spread_bps).unwrap_or(-50.0);
                    (policy_rate_bps + spread).max(0.0)
                });

                let deposit = deposit!(*creditor_agent_id, agent_id, *amount, deposit_rate, self.state.current_date);
                self.state
                    .financial_system
                    .create_or_consolidate_instrument(deposit)
                    .map_err(|e| format!("Failed to create liability deposit: {}", e))?;
            }

            LiabilityConfig::Loan { creditor_id, amount, rate_bps, maturity_days } => {
                let creditor_agent_id =
                    agent_ids.get(creditor_id).ok_or_else(|| format!("Creditor ID {} not found", creditor_id))?;

                let maturity_date =
                    maturity_days.map(|days| self.state.current_date + chrono::Duration::days(days as i64));

                let loan = FinancialInstrument {
                    id: InstrumentId(uuid::Uuid::new_v4()),
                    creditor: *creditor_agent_id,
                    debtor: agent_id,
                    principal: *amount,
                    details: Box::new(LoanDetails {
                        loan_type: LoanType::Personal,
                        interest_rate_bps: *rate_bps,
                        maturity_date: maturity_date.unwrap_or(self.state.current_date + chrono::Duration::days(365)),
                        collateral: None,
                    }),
                    originated_date: self.state.current_date,
                    accrued_interest: 0.0,
                    last_accrual_date: self.state.current_date,
                };

                self.state
                    .financial_system
                    .create_or_consolidate_instrument(loan)
                    .map_err(|e| format!("Failed to create loan: {}", e))?;
            }
        }
        Ok(())
    }
}