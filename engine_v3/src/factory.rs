use crate::scenario::{AssetConfig, BankConfig, ConsumerConfig, FirmConfig, LiabilityConfig};
use rand::prelude::*;
use rust_decimal_macros::dec;
use uuid::Uuid;

use std::collections::HashMap;

use chrono::{Duration, NaiveDate};
use sim_core::prelude::*;

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
    #[allow(dead_code)]
    ReservesFromCB,
}

pub struct BondIssuance {
    pub instrument: Instrument,
    pub total_quantity: f64,
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
        None
    }

    fn ensure_reserves_account(&mut self, bank_id: AgentId) -> InstrumentId {
        if let Some(inst_id) = self.state.financial_system.find_bank_reserves_account(&bank_id) {
            return inst_id;
        }
        let rate = self.state.financial_system.central_bank.policy_rate_bps;
        let cb_id = self.state.financial_system.central_bank.id;

        let reserves =
            Instrument::cash(InstrumentId(Uuid::new_v4()), cb_id, CashType::CentralBankReserves, Currency::USD, rate)
                .build();
        let inst_id = reserves.id;
        self.state.financial_system.instruments.insert(inst_id, reserves);

        let bank_bs =
            self.state.financial_system.balance_sheets.entry(bank_id).or_insert_with(|| BalanceSheet::new(bank_id));
        bank_bs.assets.entry(inst_id).or_insert_with(Default::default);

        let cb_bs = self.state.financial_system.balance_sheets.entry(cb_id).or_insert_with(|| BalanceSheet::new(cb_id));
        cb_bs.liabilities.entry(inst_id).or_insert_with(Default::default);

        inst_id
    }

    pub fn initialize_treasury_general_account(&mut self) -> Result<(), String> {
        let cb_id = self.state.financial_system.central_bank.id;
        let gov_id = self.state.financial_system.government.id;

        let tga = Instrument::cash(
            InstrumentId(Uuid::new_v4()),
            cb_id,
            CashType::TreasuryGeneralAccount,
            Currency::USD,
            self.state.financial_system.central_bank.policy_rate_bps,
        )
        .build();
        let tga_id = tga.id;
        self.state.financial_system.instruments.insert(tga_id, tga);

        self.state
            .financial_system
            .balance_sheets
            .entry(gov_id)
            .or_insert_with(|| BalanceSheet::new(gov_id))
            .assets
            .insert(tga_id, Position::par(1.0));
        self.state
            .financial_system
            .balance_sheets
            .entry(cb_id)
            .or_insert_with(|| BalanceSheet::new(cb_id))
            .liabilities
            .insert(tga_id, Position::par(1.0));

        let initial_balance = 30_000_000.0;
        let one = Money::ONE;
        {
            let gbs = self.state.financial_system.balance_sheets.get_mut(&gov_id).unwrap();
            if let Some(p) = gbs.assets.get_mut(&tga_id) {
                p.quantity = initial_balance;
                p.book_value_per_unit = one;
            }

            let cbs = self.state.financial_system.balance_sheets.get_mut(&cb_id).unwrap();
            if let Some(p) = cbs.liabilities.get_mut(&tga_id) {
                p.quantity = initial_balance;
                p.book_value_per_unit = one;
            }
        }

        let issue_date = self.state.current_date;
        let maturity_date = parse_tenor_to_date("T30Y", issue_date)?;
        let bond = Instrument::bond(
            InstrumentId(Uuid::new_v4()),
            gov_id,
            BondType::Government,
            Money::from(1000),
            issue_date,
            maturity_date,
        )
        .coupon_bps(self.state.financial_system.central_bank.policy_rate_bps)
        .build()
        .unwrap();

        let bond_id = bond.id;
        self.state.financial_system.instruments.insert(bond_id, bond.clone());
        self.state
            .financial_system
            .clearing_house
            .csd
            .register_security(bond_id, &bond, self.state.current_date)
            .map_err(|e| e.to_string())?;
        self.state
            .financial_system
            .clearing_house
            .csd
            .credit_securities(cb_id, bond_id, initial_balance / 1000.0)
            .map_err(|e| e.to_string())?;

        let gbs = self.state.financial_system.balance_sheets.get_mut(&gov_id).unwrap();
        gbs.liabilities.insert(
            bond_id,
            Position {
                quantity: (initial_balance / 1000.0),
                book_value_per_unit: Money::from(1000),
                cost_basis_per_unit: Money::from(1000),
            },
        );

        Ok(())
    }

    pub fn create_bank(
        &mut self, config: &BankConfig, cb_id: AgentId,
        bond_instruments: &mut std::collections::HashMap<String, InstrumentId>,
    ) -> Bank {
        let bank = Bank::new(config.name.clone(), dec!(200.0), dec!(-70.0));
        self.state.financial_system.balance_sheets.insert(bank.id, BalanceSheet::new(bank.id));

        self.state.agents.banks.insert(bank.id, bank.clone());

        for a in &config.initial_assets {
            self.create_asset_for_agent(bank.id, a, cb_id, &std::collections::HashMap::new(), bond_instruments)
                .expect("Failed to create initial asset for bank");
        }

        bank
    }

    pub fn create_consumer(
        &mut self, config: &ConsumerConfig, bank_id: AgentId, cb_id: AgentId, agent_ids: &HashMap<String, AgentId>,
        count: usize, bond_instruments: &mut HashMap<String, InstrumentId>,
    ) -> Consumer {
        let personality = *[PersonalityArchetype::Balanced, PersonalityArchetype::Saver, PersonalityArchetype::Spender]
            .choose(self.rng)
            .unwrap();
        let is_golden = count == 0;
        let mut consumer = Consumer::new(self.rng.random_range(25..65), bank_id, personality, is_golden);
        consumer.income = config.income;

        self.state.financial_system.balance_sheets.insert(consumer.id, BalanceSheet::new(consumer.id));

        for a in &config.initial_assets {
            self.create_asset_for_agent(consumer.id, a, cb_id, agent_ids, bond_instruments)
                .expect("Failed to create initial asset for consumer");
        }
        self.state.agents.consumers.insert(consumer.id, consumer.clone());
        consumer
    }

    pub fn create_firm(
        &mut self, config: &FirmConfig, bank_id: AgentId, cb_id: AgentId, agent_ids: &HashMap<String, AgentId>,
        bond_instruments: &mut HashMap<String, InstrumentId>,
    ) -> Firm {
        let recipe_id = config.recipe_name.as_deref().and_then(|s| self.resolve_recipe_id(s));
        let mut firm = Firm::new(bank_id, config.name.clone(), recipe_id, 25.0);
        if let Some(m) = config.desired_markup {
            firm.desired_markup = m;
        }

        self.state.financial_system.balance_sheets.insert(firm.id, BalanceSheet::new(firm.id));

        for a in &config.initial_assets {
            if !matches!(a, AssetConfig::Inventory { .. }) {
                self.create_asset_for_agent(firm.id, a, cb_id, agent_ids, bond_instruments)
                    .expect("Failed to create initial asset for firm");
            }
        }

        self.state.agents.firms.insert(firm.id, firm.clone());
        firm
    }

    fn create_and_register_instrument(
        &mut self, owner: AgentId, issuer: AgentId, instrument: Instrument, quantity: f64, book_value_per_unit: f64,
    ) -> Result<(), String> {
        let inst_id = instrument.id;
        self.state.financial_system.instruments.insert(inst_id, instrument.clone());

        if is_security(&instrument) {
            self.state
                .financial_system
                .clearing_house
                .csd
                .register_security(inst_id, &instrument, self.state.current_date)
                .map_err(|e| e.to_string())?;

            self.state
                .financial_system
                .clearing_house
                .csd
                .credit_securities(owner, inst_id, quantity)
                .map_err(|e| e.to_string())?;

            let book = instrument.face_value().unwrap_or(Money::from_f64(book_value_per_unit).unwrap());
            let bs_issuer =
                self.state.financial_system.balance_sheets.entry(issuer).or_insert_with(|| BalanceSheet::new(issuer));
            bs_issuer
                .liabilities
                .insert(inst_id, Position { quantity, book_value_per_unit: book, cost_basis_per_unit: book });
        } else {
            self.state.financial_system.create_or_consolidate_position(
                &owner,
                &issuer,
                &inst_id,
                quantity,
                book_value_per_unit,
            )?;
        }
        Ok(())
    }

    pub fn seed_reserves_to_bank_from_tga(&mut self, bank_id: AgentId, amount: f64) -> Result<(), String> {
        let cb_id = self.state.financial_system.central_bank.id;
        let gov_id = self.state.financial_system.government.id;

        let tga_id = self
            .state
            .financial_system
            .find_government_tga_account()
            .ok_or_else(|| "TGA not initialized".to_string())?;

        let reserves_inst_id = self.ensure_reserves_account(bank_id);
        let bank_bs = self.state.financial_system.balance_sheets.get_mut(&bank_id).unwrap();
        let rpos = bank_bs.assets.entry(reserves_inst_id).or_insert_with(Default::default);
        rpos.quantity += amount;
        rpos.book_value_per_unit = Money::ONE;

        let cb_bs = self.state.financial_system.balance_sheets.get_mut(&cb_id).unwrap();
        let tga_pos = cb_bs.liabilities.get_mut(&tga_id.0).ok_or("CB missing TGA liability")?;
        tga_pos.quantity -= amount;

        let r_liab = cb_bs.liabilities.entry(reserves_inst_id).or_insert_with(Default::default);
        r_liab.quantity += amount;
        r_liab.book_value_per_unit = Money::ONE;

        let gov_bs = self.state.financial_system.balance_sheets.get_mut(&gov_id).unwrap();
        let g_tga = gov_bs.assets.get_mut(&tga_id.0).ok_or("Gov missing TGA asset")?;
        g_tga.quantity -= amount;
        Ok(())
    }

    pub fn seed_reserves_via_cb_creation(&mut self, bank_id: AgentId, amount: f64) -> Result<(), String> {
        let cb_id = self.state.financial_system.central_bank.id;
        let reserves_inst_id = self.ensure_reserves_account(bank_id);
        let bank_bs = self.state.financial_system.balance_sheets.get_mut(&bank_id).unwrap();
        let rpos = bank_bs.assets.entry(reserves_inst_id).or_insert_with(Default::default);
        rpos.quantity += amount;
        rpos.book_value_per_unit = Money::ONE;

        let cb_bs = self.state.financial_system.balance_sheets.get_mut(&cb_id).unwrap();
        let r_liab = cb_bs.liabilities.entry(reserves_inst_id).or_insert_with(Default::default);
        r_liab.quantity += amount;
        r_liab.book_value_per_unit = Money::ONE;
        Ok(())
    }

    fn seed_deposit(
        &mut self, beneficiary_id: AgentId, bank_key: &str, amount: f64, agent_ids: &HashMap<String, AgentId>,
        _mode: SeedMode,
    ) -> Result<(), String> {
        let bank_id = *agent_ids.get(bank_key).ok_or_else(|| format!("Unknown bank id: {}", bank_key))?;

        let deposit = Instrument::cash(
            InstrumentId(Uuid::new_v4()),
            bank_id,
            CashType::DemandDeposit,
            Currency::USD,
            self.state.financial_system.central_bank.policy_rate_bps,
        )
        .build();
        let dep_id = deposit.id;
        self.ensure_reserves_account(bank_id);

        self.state.financial_system.instruments.insert(dep_id, deposit);

        {
            let bbs = self.state.financial_system.balance_sheets.get_mut(&beneficiary_id).unwrap();
            bbs.assets
                .insert(dep_id, Position { quantity: amount, book_value_per_unit: Money::ONE, ..Default::default() });
        }

        {
            let bank_bs = self.state.financial_system.balance_sheets.get_mut(&bank_id).unwrap();
            bank_bs
                .liabilities
                .insert(dep_id, Position { quantity: amount, book_value_per_unit: Money::ONE, ..Default::default() });
        }

        Ok(())
    }

    // FIX: Changed return type to Result<Option<StateEffect>, String>
    pub fn create_liability_for_agent(
        &mut self, agent_id: AgentId, liability: &LiabilityConfig, _cb_id: AgentId,
        agent_ids: &HashMap<String, AgentId>,
    ) -> Result<Option<StateEffect>, String> {
        match liability {
            LiabilityConfig::Deposit { creditor_id, amount, .. } => {
                if self.state.agents.banks.contains_key(&agent_id) {
                    let depositor_id =
                        *agent_ids.get(creditor_id).ok_or_else(|| format!("Unknown agent: {}", creditor_id))?;
                    let bank_key = agent_ids
                        .iter()
                        .find_map(|(k, v)| if *v == agent_id { Some(k.clone()) } else { None })
                        .ok_or_else(|| "Bank agent not present in id map".to_string())?;
                    self.seed_deposit(depositor_id, &bank_key, *amount, agent_ids, SeedMode::RtgsFromTreasury)?;
                    Ok(None)
                } else {
                    Err("Deposit liability configured for non-bank agent".to_string())
                }
            }
            LiabilityConfig::Loan { creditor_id, principal, rate_bps, maturity_days } => {
                let creditor_agent_id =
                    *agent_ids.get(creditor_id).ok_or_else(|| format!("Creditor ID {} not found", creditor_id))?;
                let issue_date = self.state.current_date;
                let maturity_date = issue_date + Duration::days(*maturity_days as i64);

                let instrument_id = InstrumentId(Uuid::new_v4());
                let details = LoanDetails {
                    loan_id: Uuid::new_v4(),
                    lender: creditor_agent_id,
                    borrower: agent_id,
                    loan_type: LoanType::TermLoan,
                    principal: Money::from_f64(*principal).unwrap(),
                    outstanding_principal: Money::from_f64(*principal).unwrap(),
                    spread_bps: *rate_bps,
                    origination_date: issue_date,
                    maturity_date,
                    ..Default::default()
                };

                let loan_instrument = Instrument::loan(instrument_id, details.clone());
                self.state.financial_system.instruments.insert(instrument_id, loan_instrument);

                let loan = Loan { instrument_id, details, status: LoanStatus::Current, servicing_history: Vec::new() };

                Ok(Some(StateEffect::Credit(CreditEffect::RegisterLoan {
                    loan,
                    is_consumer: false,
                    purpose: LoanPurpose::BusinessExpansion,
                })))
            }
            LiabilityConfig::Mortgage { creditor_id, principal, rate_bps, maturity_days } => {
                let creditor_agent_id =
                    *agent_ids.get(creditor_id).ok_or_else(|| format!("Creditor ID {} not found", creditor_id))?;
                let issue_date = self.state.current_date;
                let maturity_date = issue_date + Duration::days(*maturity_days as i64);

                let principal_amount = principal.sample(self.rng);

                let instrument_id = InstrumentId(Uuid::new_v4());
                let details = LoanDetails {
                    loan_id: Uuid::new_v4(),
                    lender: creditor_agent_id,
                    borrower: agent_id,
                    loan_type: LoanType::MortgageLoan,
                    principal: Money::from_f64(principal_amount).unwrap(),
                    outstanding_principal: Money::from_f64(principal_amount).unwrap(),
                    spread_bps: *rate_bps,
                    origination_date: issue_date,
                    maturity_date,
                    ..Default::default()
                };

                let loan_instrument = Instrument::loan(instrument_id, details.clone());
                self.state.financial_system.instruments.insert(instrument_id, loan_instrument);

                let loan = Loan { instrument_id, details, status: LoanStatus::Current, servicing_history: Vec::new() };

                Ok(Some(StateEffect::Credit(CreditEffect::RegisterLoan {
                    loan,
                    is_consumer: true,
                    purpose: LoanPurpose::RealEstate,
                })))
            }
            LiabilityConfig::CreditCard { creditor_id, principal, rate_bps, maturity_days } => {
                let creditor_agent_id =
                    *agent_ids.get(creditor_id).ok_or_else(|| format!("Creditor ID {} not found", creditor_id))?;
                let issue_date = self.state.current_date;
                let maturity_date = issue_date + Duration::days(*maturity_days as i64);

                let limit_amount = principal.sample(self.rng);
                let commitment_money = Money::from_f64(limit_amount).unwrap();
                let instrument_id = InstrumentId(Uuid::new_v4());

                let details = CreditLineDetails {
                    lender: creditor_agent_id,
                    borrower: agent_id,
                    commitment_amount: commitment_money,
                    available_amount: commitment_money,
                    drawn_amount: Money::ZERO,
                    spread_bps: *rate_bps,
                    expiry_date: maturity_date,
                    commitment_date: issue_date,
                    borrower_type: BorrowerType::Individual,
                    ..Default::default()
                };

                let credit_card_instrument = Instrument::consumer_credit_card(instrument_id, details.clone());
                self.state.financial_system.instruments.insert(instrument_id, credit_card_instrument);

                let facility = CreditFacility { instrument_id, details, status: FacilityStatus::Active };
                self.state.financial_system.credit_registry.register_facility(facility)?;

                Ok(None)
            }
        }
    }
    pub fn create_asset_for_agent(
        &mut self, agent_id: AgentId, asset: &AssetConfig, cb_id: AgentId, agent_ids: &HashMap<String, AgentId>,
        bond_instruments: &mut HashMap<String, InstrumentId>,
    ) -> Result<(), String> {
        match asset {
            AssetConfig::Cash { amount } => {
                let cash =
                    Instrument::cash(InstrumentId(Uuid::new_v4()), cb_id, CashType::Currency, Currency::USD, dec!(0.0))
                        .build();
                self.create_and_register_instrument(agent_id, cb_id, cash, *amount, 1.0)?;
                Ok(())
            }
            AssetConfig::Deposit { bank_id, amount } => {
                self.seed_deposit(agent_id, bank_id, *amount, agent_ids, SeedMode::RtgsFromTreasury)?;
                Ok(())
            }
            AssetConfig::Reserves { amount } => {
                if self.state.agents.banks.contains_key(&agent_id) {
                    self.seed_reserves_to_bank_from_tga(agent_id, *amount)
                        .or_else(|_| self.seed_reserves_via_cb_creation(agent_id, *amount))?;
                    Ok(())
                } else {
                    Err("Non-bank cannot hold CB reserves".to_string())
                }
            }
            AssetConfig::Bond { tenor, quantity } => {
                let gov_id = self.state.financial_system.government.id;
                let (_instrument_id, instrument) = if let Some(id) = bond_instruments.get(tenor) {
                    (*id, self.state.financial_system.instruments.get(id).unwrap().clone())
                } else {
                    let issue = self.state.current_date;
                    let maturity = parse_tenor_to_date(tenor, issue)?;
                    let coupon_bps = self.state.financial_system.central_bank.policy_rate_bps;

                    let builder = if (maturity - issue).num_days() <= 365 {
                        Instrument::bond(
                            InstrumentId(Uuid::new_v4()),
                            gov_id,
                            BondType::Government,
                            Money::from(1000),
                            issue,
                            maturity,
                        )
                        .zero_coupon_rate_bps()
                        .rating(CreditRating::Government(SpCreditRating::AAA))
                    } else {
                        Instrument::bond(
                            InstrumentId(Uuid::new_v4()),
                            gov_id,
                            BondType::Government,
                            Money::from(1000),
                            issue,
                            maturity,
                        )
                        .coupon_bps(coupon_bps)
                        .rating(CreditRating::Government(SpCreditRating::AAA))
                    };

                    let inst = builder.build().unwrap();
                    let iid = inst.id;
                    self.state.financial_system.instruments.insert(iid, inst.clone());
                    bond_instruments.insert(tenor.clone(), iid);
                    (iid, inst)
                };

                let book_value = instrument.face_value().unwrap_or(Money::from(1000));
                self.create_and_register_instrument(
                    agent_id,
                    gov_id,
                    instrument,
                    *quantity as f64,
                    book_value.to_f64(),
                )?;
                Ok(())
            }
            AssetConfig::Inventory { .. } => Ok(()),
        }
    }
}

fn parse_tenor_to_date(tenor: &str, start_date: NaiveDate) -> Result<NaiveDate, String> {
    let s = tenor.strip_prefix('T').ok_or_else(|| format!("Invalid tenor: {}", tenor))?;
    if let Some(y) = s.strip_suffix('Y') {
        Ok(TimePeriod::Years(y.parse::<u32>().map_err(|_| "year parse")?).add_to_date(start_date))
    } else if let Some(m) = s.strip_suffix('M') {
        Ok(TimePeriod::Months(m.parse::<u32>().map_err(|_| "month parse")?).add_to_date(start_date))
    } else {
        Err(format!("Unknown tenor suffix in {}", tenor))
    }
}
