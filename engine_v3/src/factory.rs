use crate::scenario::{AssetConfig, BankConfig, ConsumerConfig, FirmConfig, LiabilityConfig};
use chrono::{Duration, NaiveDate};
use rand::prelude::*;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use sim_core::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

fn is_security(inst: &Instrument) -> bool {
    matches!(
        inst.instrument_type,
        InstrumentType::Debt(DebtInstrument::Bond(_))
            | InstrumentType::Equity(_)
            | InstrumentType::StructuredTranche(_)
            | InstrumentType::Derivative(_)
    )
}
enum SeedMode {
    RtgsFromTreasury,
}
pub struct AgentFactory<'a> {
    pub state: &'a mut SimState,
    pub rng: &'a mut StdRng,
}

impl<'a> AgentFactory<'a> {
    pub fn new(state: &'a mut SimState, rng: &'a mut StdRng) -> Self {
        Self { state, rng }
    }
    fn resolve_recipe_id(&self, selector: &str) -> Option<RecipeId> {
        let recipes = &self.state.financial_system.goods.recipes;

        let as_id = RecipeId(selector.to_string());
        if recipes.contains_key(&as_id) {
            return Some(as_id);
        }

        if let Some((rid, _)) = recipes.iter().find(|(_, r)| r.name == selector) {
            return Some(rid.clone());
        }

        if let Some((rid, _)) = recipes.iter().find(|(_, r)| r.name.eq_ignore_ascii_case(selector)) {
            return Some(rid.clone());
        }

        let norm = selector.to_ascii_lowercase().replace(['_', '-'], " ");
        recipes.iter().find_map(|(rid, r)| (r.name.to_ascii_lowercase() == norm).then(|| rid.clone()))
    }
    fn create_and_register_instrument(
        &mut self, owner_id: AgentId, issuer_id: AgentId, instrument: Instrument, quantity: f64,
        book_value_per_unit: f64,
    ) -> Result<(), String> {
        let inst_id = instrument.id;
        let is_sec = is_security(&instrument);

        let instrument_clone_for_csd = instrument.clone();
        self.state.financial_system.instruments.insert(inst_id, instrument);

        if is_sec {
            self.state
                .financial_system
                .clearing_house
                .csd
                .register_security(inst_id, &instrument_clone_for_csd, self.state.current_date)
                .map_err(|e| e.to_string())?;

            if quantity > 0.0 {
                self.state
                    .financial_system
                    .clearing_house
                    .csd
                    .credit_securities(owner_id, inst_id, quantity)
                    .map_err(|e| e.to_string())?;
            }

            let book_value_money = Money::from_f64(book_value_per_unit).unwrap_or(Money::ZERO);
            let issuer_bs = self.state.financial_system.balance_sheets.get_mut(&issuer_id).ok_or("Issuer not found")?;
            let liability_pos = issuer_bs.liabilities.entry(inst_id).or_insert_with(|| Position {
                quantity: 0.0,
                book_value_per_unit: book_value_money,
                cost_basis_per_unit: book_value_money,
            });
            liability_pos.quantity += quantity;
        } else {
            self.state.financial_system.create_or_consolidate_position(
                &owner_id,
                &issuer_id,
                &inst_id,
                quantity,
                book_value_per_unit,
            )?;
        }
        Ok(())
    }

    pub fn create_bank(&mut self, config: &BankConfig, cb_id: AgentId) -> Bank {
        let bank = Bank::new(config.name.clone(), dec!(200.0), dec!(-70.0));
        self.state.financial_system.balance_sheets.insert(bank.id, BalanceSheet::new(bank.id));
        for asset in &config.initial_assets {
            self.create_asset_for_agent(bank.id, asset, cb_id, &HashMap::new()).unwrap();
        }
        for liability in &config.initial_liabilities {
            self.create_liability_for_agent(bank.id, liability, cb_id, &HashMap::new()).unwrap();
        }
        self.state.agents.banks.insert(bank.id, bank.clone());
        bank
    }

    pub fn create_consumer(
        &mut self, config: &ConsumerConfig, bank_id: AgentId, cb_id: AgentId, agent_ids: &HashMap<String, AgentId>, count: usize
    ) -> Consumer {
        let personality =
            *vec![PersonalityArchetype::Balanced, PersonalityArchetype::Saver, PersonalityArchetype::Spender]
                .choose(self.rng)
                .unwrap();
        let is_golden = count == 0; // Make the first consumer golden
        let mut consumer = Consumer::new(self.rng.random_range(25..65), bank_id, personality, is_golden);
        consumer.income = config.income;
        self.state.financial_system.balance_sheets.insert(consumer.id, BalanceSheet::new(consumer.id));
        for asset in &config.initial_assets {
            self.create_asset_for_agent(consumer.id, asset, cb_id, agent_ids).unwrap();
        }
        for liability in &config.initial_liabilities {
            self.create_liability_for_agent(consumer.id, liability, cb_id, agent_ids).unwrap();
        }
        self.state.agents.consumers.insert(consumer.id, consumer.clone());
        consumer
    }

    pub fn create_firm(
        &mut self, config: &FirmConfig, bank_id: AgentId, cb_id: AgentId, agent_ids: &HashMap<String, AgentId>,
    ) -> Firm {
        let recipe_id = config.recipe_name.as_deref().and_then(|s| self.resolve_recipe_id(s));

        let mut firm = Firm::new(bank_id, config.name.clone(), recipe_id, 25.0);
        if let Some(markup) = config.desired_markup {
            firm.desired_markup = markup;
        }
        self.state.financial_system.balance_sheets.insert(firm.id, BalanceSheet::new(firm.id));

        for asset in &config.initial_assets {
            if !matches!(asset, AssetConfig::Inventory { .. }) {
                self.create_asset_for_agent(firm.id, asset, cb_id, agent_ids).unwrap();
            }
        }

        for liability in &config.initial_liabilities {
            self.create_liability_for_agent(firm.id, liability, cb_id, agent_ids).unwrap();
        }
        self.state.agents.firms.insert(firm.id, firm.clone());
        firm
    }

    fn create_asset_for_agent(
        &mut self, agent_id: AgentId, asset: &AssetConfig, cb_id: AgentId, agent_ids: &HashMap<String, AgentId>,
    ) -> Result<(), String> {
        match asset {
            AssetConfig::Cash { amount } => {
                let cash =
                    Instrument::cash(InstrumentId(Uuid::new_v4()), cb_id, CashType::Currency, Currency::USD, dec!(0.0))
                        .build();
                self.create_and_register_instrument(agent_id, cb_id, cash, *amount, 1.0)?;
            }
            AssetConfig::Deposit { bank_id, amount } => {
                self.seed_deposit(agent_id, bank_id, *amount, agent_ids, SeedMode::RtgsFromTreasury)?;
            }
            AssetConfig::Reserves { amount } => {
                let rate = self.state.financial_system.central_bank.policy_rate_bps;
                let reserves = Instrument::cash(
                    InstrumentId(Uuid::new_v4()),
                    cb_id,
                    CashType::CentralBankReserves,
                    Currency::USD,
                    rate,
                )
                .build();
                self.create_and_register_instrument(agent_id, cb_id, reserves, *amount, 1.0)?;
            }
            AssetConfig::Bond { tenor, quantity } => {
                let gov_id = self.state.financial_system.government.id;
                let issue_date = self.state.current_date;
                let maturity_date = parse_tenor_to_date(tenor, issue_date)?;
                let coupon_bps = self.state.financial_system.central_bank.policy_rate_bps;
                let bond_builder = if (maturity_date - issue_date).num_days() <= 365 {
                    Instrument::bond(
                        InstrumentId(Uuid::new_v4()),
                        gov_id,
                        BondType::Government,
                        Money::from(1000),
                        issue_date,
                        maturity_date,
                    )
                    .zero_coupon_rate_bps()
                } else {
                    Instrument::bond(
                        InstrumentId(Uuid::new_v4()),
                        gov_id,
                        BondType::Government,
                        Money::from(1000),
                        issue_date,
                        maturity_date,
                    )
                    .coupon_bps(coupon_bps)
                    .frequency(2)
                };
                let bond_instrument = bond_builder
                    .rating(CreditRating::government_aaa())
                    .auto_market()
                    .build()
                    .map_err(|e| e.to_string())?;
                self.create_and_register_instrument(agent_id, gov_id, bond_instrument, *quantity as f64, 1000.0)?;
            }
            AssetConfig::Inventory { .. } => {}
        }
        Ok(())
    }

    fn create_liability_for_agent(
        &mut self, agent_id: AgentId, liability: &LiabilityConfig, _cb_id: AgentId,
        agent_ids: &HashMap<String, AgentId>,
    ) -> Result<(), String> {
        match liability {
            LiabilityConfig::Deposit { creditor_id, amount, .. } => {
                let creditor_agent_id =
                    *agent_ids.get(creditor_id).ok_or_else(|| format!("Creditor ID {} not found", creditor_id))?;
                let rate = self.state.financial_system.central_bank.policy_rate_bps;
                let deposit = Instrument::cash(
                    InstrumentId(Uuid::new_v4()),
                    agent_id,
                    CashType::DemandDeposit,
                    Currency::USD,
                    rate,
                )
                .build();
                self.create_and_register_instrument(creditor_agent_id, agent_id, deposit, *amount, 1.0)?;
            }
            LiabilityConfig::Loan { creditor_id, amount, rate_bps, maturity_days } => {
                let creditor_agent_id =
                    *agent_ids.get(creditor_id).ok_or_else(|| format!("Creditor ID {} not found", creditor_id))?;
                let issue_date = self.state.current_date;
                let maturity_date = issue_date + Duration::days(maturity_days.unwrap_or(365) as i64);
                let loan_instrument = Instrument::bond(
                    InstrumentId(Uuid::new_v4()),
                    agent_id,
                    BondType::Corporate,
                    Money::from(*amount as u64),
                    issue_date,
                    maturity_date,
                )
                .coupon_bps(Decimal::from_f64(*rate_bps).unwrap_or_default())
                .frequency(12)
                .rating(CreditRating::Corporate(SpCreditRating::BBB))
                .build()
                .map_err(|e| format!("Failed to build loan instrument: {}", e))?;
                self.create_and_register_instrument(creditor_agent_id, agent_id, loan_instrument, 1.0, *amount)?;
            }
        }
        Ok(())
    }

    pub fn initialize_treasury_general_account(&mut self) -> Result<(), String> {
        let gov_id = self.state.financial_system.government.id;
        let cb_id = self.state.financial_system.central_bank.id;
        let initial_balance = 200_000_000_000.0; // 200 Billion

        let tga = Instrument::cash(
            InstrumentId(Uuid::new_v4()),
            cb_id, // Issued by CB
            CashType::TreasuryGeneralAccount,
            Currency::USD,
            dec!(0),
        )
        .build();
        self.state.financial_system.instruments.insert(tga.id, tga.clone());

        let bond_instrument = Instrument::bond(
            InstrumentId(Uuid::new_v4()),
            gov_id, // Issued by Government
            BondType::Government,
            Money::from_f64(initial_balance).unwrap(),
            self.state.current_date,
            self.state.current_date + Duration::days(365 * 30), // 30-year bond
        )
        .coupon_bps(decimal_to_bps(dec!(0.035))) // 3.5% coupon
        .build()
        .map_err(|e| e.to_string())?;
        let bond_id = bond_instrument.id;
        self.state.financial_system.instruments.insert(bond_id, bond_instrument);

        let gov_bs =
            self.state.financial_system.balance_sheets.entry(gov_id).or_insert_with(|| BalanceSheet::new(gov_id));
        gov_bs.assets.insert(
            tga.id,
            Position { quantity: initial_balance, book_value_per_unit: Money::ONE, ..Default::default() },
        );
        gov_bs.liabilities.insert(
            bond_id,
            Position {
                quantity: 1.0,
                book_value_per_unit: Money::from_f64(initial_balance).unwrap(),
                ..Default::default()
            },
        );

        let cb_bs = self.state.financial_system.balance_sheets.entry(cb_id).or_insert_with(|| BalanceSheet::new(cb_id));
        cb_bs.assets.insert(
            bond_id,
            Position {
                quantity: 1.0,
                book_value_per_unit: Money::from_f64(initial_balance).unwrap(),
                ..Default::default()
            },
        );
        cb_bs.liabilities.insert(
            tga.id,
            Position { quantity: initial_balance, book_value_per_unit: Money::ONE, ..Default::default() },
        );

        self.state.financial_system.clearing_house.csd.initialize_government_account(gov_id);
        Ok(())
    }
    fn seed_deposit(
        &mut self, beneficiary_id: AgentId, bank_ref_id: &str, amount: f64, agent_ids: &HashMap<String, AgentId>,
        mode: SeedMode,
    ) -> Result<(), String> {
        match mode {
            SeedMode::RtgsFromTreasury => {
                let bank_id = *agent_ids
                    .get(bank_ref_id)
                    .ok_or_else(|| format!("Bank ID '{}' not found in agent map", bank_ref_id))?;
                let cb_id = self.state.financial_system.central_bank.id;
                let gov_id = self.state.financial_system.government.id;

                let rate = self.state.financial_system.central_bank.policy_rate_bps;
                let deposit_instrument = Instrument::cash(
                    InstrumentId(Uuid::new_v4()),
                    bank_id,
                    CashType::DemandDeposit,
                    Currency::USD,
                    rate,
                )
                .build();
                let deposit_inst_id = deposit_instrument.id;
                self.state.financial_system.instruments.insert(deposit_inst_id, deposit_instrument);

                let reserves_inst_id = self
                    .state
                    .financial_system
                    .find_bank_reserves_account(&bank_id)
                    .ok_or_else(|| format!("Bank {} has no reserves account initialized", bank_id))?;
                let tga_inst_id = self
                    .state
                    .financial_system
                    .find_government_tga_account()
                    .map(|(id, _)| id)
                    .ok_or_else(|| "Government TGA account not found. It must be initialized first.".to_string())?;

                let beneficiary_bs = self.state.financial_system.balance_sheets.get_mut(&beneficiary_id).unwrap();
                beneficiary_bs.assets.insert(
                    deposit_inst_id,
                    Position { quantity: amount, book_value_per_unit: Money::ONE, ..Default::default() },
                );

                let bank_bs = self.state.financial_system.balance_sheets.get_mut(&bank_id).unwrap();
                bank_bs.liabilities.insert(
                    deposit_inst_id,
                    Position { quantity: amount, book_value_per_unit: Money::ONE, ..Default::default() },
                );
                let reserves_pos = bank_bs.assets.entry(reserves_inst_id).or_insert_with(Default::default);
                reserves_pos.quantity += amount;

                let gov_bs = self.state.financial_system.balance_sheets.get_mut(&gov_id).unwrap();
                let tga_pos = gov_bs.assets.get_mut(&tga_inst_id).ok_or("TGA position not found in Gov BS")?;
                tga_pos.quantity -= amount;

                let cb_bs = self.state.financial_system.balance_sheets.get_mut(&cb_id).unwrap();
                let tga_liab_pos = cb_bs.liabilities.get_mut(&tga_inst_id).ok_or("TGA position not found in CB BS")?;
                tga_liab_pos.quantity -= amount;
                let reserves_liab_pos = cb_bs
                    .liabilities
                    .get_mut(&reserves_inst_id)
                    .ok_or_else(|| format!("Reserves for bank {} not found in CB BS", bank_id))?;
                reserves_liab_pos.quantity += amount;

                Ok(())
            }
        }
    }
}

fn parse_tenor_to_date(tenor_str: &str, start_date: NaiveDate) -> Result<NaiveDate, String> {
    let s = tenor_str.strip_prefix('T').ok_or_else(|| format!("Invalid tenor format: {}", tenor_str))?;
    if let Some(years_str) = s.strip_suffix('Y') {
        Ok(TimePeriod::Years(years_str.parse::<u32>().unwrap()).add_to_date(start_date))
    } else if let Some(months_str) = s.strip_suffix('M') {
        Ok(TimePeriod::Months(months_str.parse::<u32>().unwrap()).add_to_date(start_date))
    } else {
        Err(format!("Unknown suffix in tenor: {}", tenor_str))
    }
}
