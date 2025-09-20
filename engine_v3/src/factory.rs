use crate::scenario::{AssetConfig, BankConfig, ConsumerConfig, FirmConfig, LiabilityConfig, ReservesConfig};
use chrono::{Duration, Months, NaiveDate};
use rand::distr::Uniform;
use rand::prelude::*;
use rust_decimal_macros::dec;
use sim_core::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

pub const DEFAULT_TGA_BALANCE: f64 = 15_000_000.0;

pub struct AgentFactory<'a> {
    pub state: &'a mut SimState,
    pub rng: &'a mut StdRng,
    agent_ids: HashMap<String, AgentId>,
    pending_effects: Vec<StateEffect>,
}

impl<'a> AgentFactory<'a> {
    pub fn new(state: &'a mut SimState, rng: &'a mut StdRng) -> Self {
        Self { state, rng, agent_ids: HashMap::new(), pending_effects: Vec::new() }
    }

    pub fn create_agent_entities(&mut self, banks: &[BankConfig], consumers: &[ConsumerConfig], firms: &[FirmConfig]) {
        for config in banks {
            let bank = Bank::new(config.name.clone(), dec!(200.0), dec!(-70.0));
            self.agent_ids.insert(config.id.clone(), bank.id);
            self.state.agents.banks.insert(bank.id, bank);
        }

        for (i, config) in consumers.iter().enumerate() {
            let bank_id = *self.agent_ids.get(&config.bank_id).expect("Bank ID not found for consumer");
            let personality =
                *[PersonalityArchetype::Balanced, PersonalityArchetype::Saver, PersonalityArchetype::Spender]
                    .choose(self.rng)
                    .unwrap();
            let is_golden = i == 0;
            let mut consumer = Consumer::new(self.rng.random_range(25..65), bank_id, personality, is_golden);
            consumer.income = config.income;
            self.agent_ids.insert(config.id.clone(), consumer.id);
            self.state.agents.consumers.insert(consumer.id, consumer);
        }

        for config in firms {
            let bank_id = *self.agent_ids.get(&config.bank_id).expect("Bank ID not found for firm");
            let recipe_id = config.recipe_name.as_deref().map(|s| RecipeId(s.to_string()));
            let mut firm = Firm::new(bank_id, config.name.clone(), recipe_id, 25.0);
            if let Some(m) = config.desired_markup {
                firm.desired_markup = m;
            }
            self.agent_ids.insert(config.id.clone(), firm.id);
            self.state.agents.firms.insert(firm.id, firm);
        }
    }

    pub fn setup_market_infrastructure(&mut self) {
        let current_date = self.state.current_date;
        self.state.financial_system.attach_default_pricing_feeds(current_date);
        self.state.financial_system.exchange.ensure_labour_market(LabourMarketId(Uuid::new_v4()), "General");
    }

    pub fn create_balance_sheet_skeletons(&mut self) {
        for agent_id in self.agent_ids.values() {
            self.state.financial_system.balance_sheets.insert(*agent_id, BalanceSheet::new(*agent_id));
        }
        let gov_id = self.state.financial_system.government.id;
        let cb_id = self.state.financial_system.central_bank.id;
        self.state.financial_system.balance_sheets.insert(gov_id, BalanceSheet::new(gov_id));
        self.state.financial_system.balance_sheets.insert(cb_id, BalanceSheet::new(cb_id));
    }

    pub fn populate_positions(
        &mut self, banks: &[BankConfig], consumers: &[ConsumerConfig], firms: &[FirmConfig],
        good_ids: &HashMap<String, GoodId>,
    ) -> f64 {
        let all_configs: Vec<(&String, &Vec<AssetConfig>, &Vec<LiabilityConfig>)> = banks
            .iter()
            .map(|c| (&c.id, &c.initial_assets, &c.initial_liabilities))
            .chain(consumers.iter().map(|c| (&c.id, &c.initial_assets, &c.initial_liabilities)))
            .chain(firms.iter().map(|c| (&c.id, &c.initial_assets, &c.initial_liabilities)))
            .collect();

        for (id_str, assets, liabilities) in all_configs {
            let agent_id = *self.agent_ids.get(id_str).unwrap();

            for asset in assets {
                self.populate_asset(agent_id, asset, good_ids);
            }
            for liability in liabilities {
                self.populate_liability(agent_id, liability);
            }
        }

        self.apply_pending_effects();

        self.allocate_bank_reserves(banks)
    }

    fn populate_asset(&mut self, owner_id: AgentId, asset_config: &AssetConfig, good_ids: &HashMap<String, GoodId>) {
        match asset_config {
            AssetConfig::Deposit { bank_id, amount } => {
                let bank_agent_id = *self.agent_ids.get(bank_id).unwrap();
                let instrument = self.get_or_create_deposit_instrument(bank_agent_id);

                self.pending_effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument {
                    instrument: instrument.clone(),
                    creditor: owner_id,
                    debtor: bank_agent_id,
                    quantity: *amount,
                }));
            }
            AssetConfig::Reserves { amount } => {
                let cb_id = self.state.financial_system.central_bank.id;
                let instrument = self.get_or_create_reserves_instrument();

                self.pending_effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument {
                    instrument: instrument.clone(),
                    creditor: owner_id,
                    debtor: cb_id,
                    quantity: *amount,
                }));
            }
            AssetConfig::Bond { tenor, quantity } => {
                let gov_id = self.state.financial_system.government.id;
                let instrument = self.get_or_create_bond_instrument(tenor, gov_id);
                self.pending_effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument {
                    instrument: instrument.clone(),
                    creditor: owner_id,
                    debtor: gov_id,
                    quantity: *quantity as f64,
                }));
            }
            AssetConfig::Inventory { good_slug, quantity, unit_cost } => {
                let good_id = *good_ids.get(good_slug).unwrap();
                self.pending_effects.push(StateEffect::Inventory(InventoryEffect::AddInventory {
                    owner: owner_id,
                    good_id,
                    quantity: *quantity,
                    unit_cost: *unit_cost,
                }));
            }
            _ => {}
        }
    }

    fn populate_liability(&mut self, owner_id: AgentId, liability_config: &LiabilityConfig) {
        match liability_config {
            LiabilityConfig::Loan { creditor_id, principal, rate_bps, maturity_days } => {
                let creditor_agent_id = *self.agent_ids.get(creditor_id).unwrap();
                self.create_loan_via_effects(
                    owner_id,
                    creditor_agent_id,
                    *principal,
                    *rate_bps,
                    *maturity_days,
                    LoanPurpose::WorkingCapital,
                );
            }
            LiabilityConfig::Mortgage { creditor_id, principal, rate_bps, maturity_days } => {
                let creditor_agent_id = *self.agent_ids.get(creditor_id).unwrap();
                let p = principal.sample(self.rng);
                self.create_loan_via_effects(
                    owner_id,
                    creditor_agent_id,
                    p,
                    *rate_bps,
                    *maturity_days,
                    LoanPurpose::RealEstate,
                );
            }
            _ => {}
        }
    }

    fn create_loan_via_effects(
        &mut self, borrower: AgentId, lender: AgentId, principal: f64, rate_bps: BasisPoints, maturity_days: u32,
        purpose: LoanPurpose,
    ) {
        let issue_date = self.state.current_date;
        let maturity_date = issue_date + Duration::days(maturity_days as i64);
        let is_consumer = self.state.agents.consumers.contains_key(&borrower);

        let loan_details = LoanDetails {
            loan_id: Uuid::new_v4(),
            lender,
            borrower,
            loan_type: match purpose {
                LoanPurpose::RealEstate => LoanType::MortgageLoan,
                LoanPurpose::WorkingCapital => LoanType::WorkingCapital,
                LoanPurpose::Equipment => LoanType::AssetFinance,
                _ => LoanType::TermLoan,
            },
            principal: Money::from_f64(principal).unwrap(),
            outstanding_principal: Money::from_f64(principal).unwrap(),
            spread_bps: rate_bps,
            origination_date: issue_date,
            maturity_date,
            next_payment_date: issue_date + Duration::days(30),
            last_accrual_date: issue_date,
            payment_frequency: PaymentFrequency::Monthly,
            rating: Some(CreditRating::consumer_prime()),
            ..Default::default()
        };
        let loan = Loan {
            instrument_id: InstrumentId(uuid::Uuid::new_v4()),
            details: loan_details,
            status: LoanStatus::Current,
            servicing_history: vec![],
        };

        self.pending_effects.push(StateEffect::Credit(CreditEffect::RegisterLoan {
            loan: loan.clone(),
            is_consumer,
            purpose,
        }));
    }

    fn apply_pending_effects(&mut self) {
        let effects = std::mem::take(&mut self.pending_effects);
        for effect in effects {
            if let Err(e) = self.state.apply_effect(&effect) {
                eprintln!("Error applying effect: {:?}", e);
            }
        }
    }

    pub fn initialize_treasury_general_account(&mut self, initial_balance: f64) {
        let cb_id = self.state.financial_system.central_bank.id;
        let gov_id = self.state.financial_system.government.id;

        let tga_instrument = self.get_or_create_tga_instrument();

        self.pending_effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument {
            instrument: tga_instrument.clone(),
            creditor: gov_id,
            debtor: cb_id,
            quantity: initial_balance,
        }));

        self.apply_pending_effects();
    }

    pub fn seed_central_bank_portfolio(&mut self, face_amount: f64) {
        if face_amount <= 0.0 {
            return;
        }

        let cb_id = self.state.financial_system.central_bank.id;
        let gov_id = self.state.financial_system.government.id;
        let bond = self.get_or_create_bond_instrument("CB_BALANCE", gov_id);

        let face_value = bond.face_value().map(|m| m.to_f64()).unwrap_or(1_000.0);
        if face_value <= 0.0 {
            return;
        }

        let quantity = (face_amount / face_value).round();
        if quantity <= 0.0 {
            return;
        }

        self.pending_effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument {
            instrument: bond,
            creditor: cb_id,
            debtor: gov_id,
            quantity,
        }));

        self.apply_pending_effects();
    }

    fn get_or_create_deposit_instrument(&mut self, bank_id: AgentId) -> Instrument {
        let deposit =
            Instrument::cash(InstrumentId(Uuid::new_v4()), bank_id, CashType::DemandDeposit, Currency::USD, dec!(25.0))
                .build();
        deposit
    }

    fn get_or_create_reserves_instrument(&mut self) -> Instrument {
        let cb_id = self.state.financial_system.central_bank.id;
        let reserves = Instrument::cash(
            InstrumentId(Uuid::new_v4()),
            cb_id,
            CashType::CentralBankReserves,
            Currency::USD,
            self.state.financial_system.central_bank.policy_rate_bps,
        )
        .build();
        reserves
    }

    fn get_or_create_tga_instrument(&mut self) -> Instrument {
        let cb_id = self.state.financial_system.central_bank.id;
        let tga = Instrument::cash(
            InstrumentId(Uuid::new_v4()),
            cb_id,
            CashType::TreasuryGeneralAccount,
            Currency::USD,
            dec!(0.0),
        )
        .build();
        tga
    }

    fn get_or_create_bond_instrument(&mut self, tenor: &str, issuer_id: AgentId) -> Instrument {
        let issue = self.state.current_date;
        let maturity = Self::tenor_to_maturity(issue, tenor).unwrap_or_else(|| issue + Duration::days(365 * 2));
        let bond = Instrument::bond(
            InstrumentId(Uuid::new_v4()),
            issuer_id,
            BondType::Government,
            Money::from(1000),
            issue,
            maturity,
        )
        .coupon_bps(dec!(250.0))
        .rating(CreditRating::government_aaa())
        .auto_market()
        .build()
        .unwrap();

        bond
    }

    fn tenor_to_maturity(issue: NaiveDate, tenor: &str) -> Option<NaiveDate> {
        let months = Self::parse_tenor_to_months(tenor)?;
        issue.checked_add_months(Months::new(months)).or_else(|| {
            let approx_years = months as f64 / 12.0;
            let days = (approx_years * 365.0).round() as i64;
            if days > 0 { Some(issue + Duration::days(days)) } else { None }
        })
    }

    fn parse_tenor_to_months(tenor: &str) -> Option<u32> {
        let trimmed = tenor.trim();
        let trimmed = trimmed.strip_prefix('T').or_else(|| trimmed.strip_prefix('t'))?;
        if trimmed.is_empty() {
            return None;
        }

        let trimmed = trimmed.to_ascii_uppercase();
        let digit_count = trimmed.chars().take_while(|c| c.is_ascii_digit()).count();
        if digit_count == 0 || digit_count == trimmed.len() {
            return None;
        }

        let (value_str, unit_str) = trimmed.split_at(digit_count);
        let value: u32 = value_str.parse().ok()?;
        if value == 0 {
            return None;
        }

        match unit_str {
            "M" => Some(value),
            "Y" => value.checked_mul(12),
            _ => None,
        }
    }

    fn allocate_bank_reserves(&mut self, banks: &[BankConfig]) -> f64 {
        let mut total_reserves = 0.0;
        let cb_id = self.state.financial_system.central_bank.id;

        for bank_cfg in banks {
            let bank_id = match self.agent_ids.get(&bank_cfg.id) {
                Some(id) => *id,
                None => continue,
            };

            let target_amount = match &bank_cfg.reserves {
                Some(ReservesConfig::RatioOfLiabilities { min_ratio, max_ratio }) => {
                    let min = (*min_ratio).max(0.0);
                    let max = (*max_ratio).max(min);
                    let liabilities = self.total_liabilities_for(bank_id);
                    if liabilities <= 0.0 {
                        continue;
                    }
                    let ratio = if (max - min).abs() <= f64::EPSILON {
                        min
                    } else {
                        let dist = Uniform::new_inclusive(min, max).unwrap();
                        self.rng.sample(dist)
                    };
                    liabilities * ratio
                }
                Some(ReservesConfig::RatioOfDeposits { ratio, .. }) => {
                    let liabilities = self.total_deposit_liabilities_for(bank_id);
                    if liabilities <= 0.0 {
                        continue;
                    }
                    liabilities * ratio
                }
                None => continue,
            };

            if target_amount <= 0.0 {
                continue;
            }

            total_reserves += target_amount;
            let instrument = self.get_or_create_reserves_instrument();
            self.pending_effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument {
                instrument,
                creditor: bank_id,
                debtor: cb_id,
                quantity: target_amount,
            }));
        }

        self.apply_pending_effects();
        total_reserves
    }

    fn total_liabilities_for(&self, bank_id: AgentId) -> f64 {
        let mut total = 0.0;
        if let Some(bs) = self.state.financial_system.balance_sheets.get(&bank_id) {
            for (_inst_id, pos) in &bs.liabilities {
                let per_unit = pos.book_value_per_unit.to_f64();
                if per_unit.is_finite() {
                    total += pos.quantity * per_unit;
                }
            }
        }
        total
    }

    fn total_deposit_liabilities_for(&self, bank_id: AgentId) -> f64 {
        let mut total = 0.0;
        let system = &self.state.financial_system;
        if let Some(bs) = system.balance_sheets.get(&bank_id) {
            for (inst_id, pos) in &bs.liabilities {
                if let Some(inst) = system.instruments.instruments.get(inst_id) {
                    if let InstrumentType::Cash(cash) = &inst.instrument_type {
                        if matches!(cash.cash_type, CashType::DemandDeposit | CashType::SavingsDeposit) {
                            let per_unit = pos.book_value_per_unit.to_f64();
                            if per_unit.is_finite() {
                                total += pos.quantity * per_unit;
                            }
                        }
                    }
                }
            }
        }
        total
    }

    pub fn get_agent_id_map(&self) -> &HashMap<String, AgentId> {
        &self.agent_ids
    }
}
