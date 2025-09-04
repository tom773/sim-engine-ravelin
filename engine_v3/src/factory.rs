use crate::scenario::{AssetConfig, BankConfig, ConsumerConfig, FirmConfig, LiabilityConfig};
use chrono::{Duration, NaiveDate};
use rand::prelude::*;
use rust_decimal::prelude::*; // Import ToPrimitive trait
use rust_decimal_macros::dec;
use sim_core::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

pub struct AgentFactory<'a> {
    pub state: &'a mut SimState,
    pub rng: &'a mut ThreadRng,
}

impl<'a> AgentFactory<'a> {
    pub fn new(state: &'a mut SimState, rng: &'a mut ThreadRng) -> Self {
        Self { state, rng }
    }

    fn create_and_register_instrument(
        &mut self, owner_id: AgentId, issuer_id: AgentId, instrument: Instrument, quantity: f64,
        book_value_per_unit: f64,
    ) -> Result<(), String> {
        self.state.financial_system.create_or_consolidate_instrument(
            owner_id,
            issuer_id,
            instrument,
            quantity,
            book_value_per_unit,
        )?;
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
        let recipe_id = config.recipe_name.as_ref().and_then(|name| {
            self.state.financial_system.goods.recipes.iter().find(|(_, r)| &r.name == name).map(|(id, _)| *id)
        });
        let mut firm = Firm::new(bank_id, config.name.clone(), recipe_id, 25.0);
        if let Some(markup) = config.desired_markup {
            firm.desired_markup = markup;
        }
        self.state.financial_system.balance_sheets.insert(firm.id, BalanceSheet::new(firm.id));

        let inventory_instrument = Instrument::new(
            InstrumentId(Uuid::new_v4()),
            InstrumentType::RealAsset(RealAssetType::Inventory { owner: firm.id, goods: HashMap::new() }),
            InstrumentMarket::CapitalMarket(CapitalMarketSegment::StructuredFinance),
        );
        self.create_and_register_instrument(
            firm.id,
            firm.id,
            inventory_instrument,
            1.0, // Represents one 'inventory' asset on the balance sheet
            0.0, // Initial book value is zero, determined by contents
        )
        .unwrap();

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
                let bank_agent_id = *agent_ids.get(bank_id).ok_or_else(|| format!("Bank ID {} not found", bank_id))?;
                let rate = self.state.financial_system.central_bank.policy_rate_bps;
                let deposit = Instrument::cash(
                    InstrumentId(Uuid::new_v4()),
                    bank_agent_id,
                    CashType::DemandDeposit,
                    Currency::USD,
                    rate,
                )
                .build();
                self.create_and_register_instrument(agent_id, bank_agent_id, deposit, *amount, 1.0)?;
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
                let bond_instrument =
                    bond_builder.rating(CreditRating::AAA).auto_market().build().map_err(|e| e.to_string())?;
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
                .rating(CreditRating::B)
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

        let tga = Instrument::cash(
            InstrumentId(Uuid::new_v4()),
            cb_id, // Issuer is central bank
            CashType::TreasuryGeneralAccount,
            Currency::USD,
            dec!(0), // No interest on TGA
        )
        .build();

        self.state.financial_system.instruments.insert(tga.id, tga.clone());

        let initial_balance = 1_000_000.0;

        let gov_bs =
            self.state.financial_system.balance_sheets.entry(gov_id).or_insert_with(|| BalanceSheet::new(gov_id));

        gov_bs.assets.insert(
            tga.id,
            Position {
                quantity: initial_balance,
                book_value_per_unit: Money::from(1),
                cost_basis_per_unit: Money::from(1),
            },
        );

        let cb_bs = self.state.financial_system.balance_sheets.entry(cb_id).or_insert_with(|| BalanceSheet::new(cb_id));

        cb_bs.liabilities.insert(
            tga.id,
            Position {
                quantity: initial_balance,
                book_value_per_unit: Money::from(1),
                cost_basis_per_unit: Money::from(1),
            },
        );

        self.state.financial_system.clearing_house.csd.initialize_government_account(gov_id);

        Ok(())
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
