use crate::scenario::{AssetConfig, BankConfig, ConsumerConfig, FirmConfig, LiabilityConfig};
use rand::prelude::*;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Duration;
use sim_core::prelude::*;

pub struct AgentFactory<'a> {
    pub state: &'a mut SimState,
    pub rng: &'a mut StdRng,
    agent_ids: HashMap<String, AgentId>,
    instrument_ids: HashMap<String, InstrumentId>,
    pending_effects: Vec<StateEffect>, // Collect effects to apply
}

impl<'a> AgentFactory<'a> {
    pub fn new(state: &'a mut SimState, rng: &'a mut StdRng) -> Self {
        Self { 
            state, 
            rng, 
            agent_ids: HashMap::new(), 
            instrument_ids: HashMap::new(),
            pending_effects: Vec::new(),
        }
    }

    pub fn create_agent_entities(
        &mut self,
        banks: &[BankConfig],
        consumers: &[ConsumerConfig],
        firms: &[FirmConfig],
    ) {
        for config in banks {
            let bank = Bank::new(config.name.clone(), dec!(200.0), dec!(-70.0));
            self.agent_ids.insert(config.id.clone(), bank.id);
            self.state.agents.banks.insert(bank.id, bank);
        }

        for (i, config) in consumers.iter().enumerate() {
            let bank_id = *self.agent_ids.get(&config.bank_id).expect("Bank ID not found for consumer");
            let personality = *[
                PersonalityArchetype::Balanced,
                PersonalityArchetype::Saver,
                PersonalityArchetype::Spender,
            ]
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
        &mut self,
        banks: &[BankConfig],
        consumers: &[ConsumerConfig],
        firms: &[FirmConfig],
        good_ids: &HashMap<String, GoodId>,
    ) {
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
                
                let owner_bs = self.state.financial_system.balance_sheets.get_mut(&owner_id).unwrap();
                owner_bs.assets.insert(instrument.id, Position { 
                    quantity: *amount, 
                    book_value_per_unit: Money::ONE, 
                    cost_basis_per_unit: Money::ONE 
                });
                
                let cb_bs = self.state.financial_system.balance_sheets.get_mut(&cb_id).unwrap();
                cb_bs.liabilities.insert(instrument.id, Position {
                    quantity: *amount,
                    book_value_per_unit: Money::ONE,
                    cost_basis_per_unit: Money::ONE
                });
            }
            AssetConfig::Bond { tenor, quantity } => {
                let gov_id = self.state.financial_system.government.id;
                let instrument = self.get_or_create_bond_instrument(tenor, gov_id);
                
                self.state.financial_system.clearing_house.csd.register_security(
                    instrument.id, 
                    &instrument, 
                    self.state.current_date
                ).unwrap();
                
                self.state.financial_system.clearing_house.csd.credit_securities(
                    owner_id, 
                    instrument.id, 
                    *quantity as f64
                ).unwrap();
                
                let gov_bs = self.state.financial_system.balance_sheets.get_mut(&gov_id).unwrap();
                gov_bs.liabilities.insert(instrument.id, Position {
                    quantity: *quantity as f64,
                    book_value_per_unit: Money::from(1000),
                    cost_basis_per_unit: Money::from(1000),
                });
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
                    LoanPurpose::WorkingCapital
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
                    LoanPurpose::RealEstate
                );
            }
            _ => {}
        }
    }

    fn create_loan_via_effects(
        &mut self,
        borrower: AgentId,
        lender: AgentId,
        principal: f64,
        rate_bps: BasisPoints,
        maturity_days: u32,
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
        let loan = Loan { instrument_id: InstrumentId(uuid::Uuid::new_v4()), details: loan_details, status: LoanStatus::Current, servicing_history: vec![] };
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

        let gov_bs = self.state.financial_system.balance_sheets.get_mut(&gov_id).unwrap();
        gov_bs.assets.insert(tga_instrument.id, Position {
            quantity: initial_balance,
            book_value_per_unit: Money::ONE,
            cost_basis_per_unit: Money::ONE,
        });

        let cb_bs = self.state.financial_system.balance_sheets.get_mut(&cb_id).unwrap();
        cb_bs.liabilities.insert(tga_instrument.id, Position {
            quantity: initial_balance,
            book_value_per_unit: Money::ONE,
            cost_basis_per_unit: Money::ONE,
        });
    }

    fn get_or_create_deposit_instrument(&mut self, bank_id: AgentId) -> Instrument {
        let key = format!("DEPOSIT_{}", bank_id);
        if let Some(inst_id) = self.instrument_ids.get(&key) {
            return self.state.financial_system.instruments.get(inst_id).unwrap().clone();
        }
        let deposit = Instrument::cash(
            InstrumentId(Uuid::new_v4()),
            bank_id,
            CashType::DemandDeposit,
            Currency::USD,
            dec!(25.0)
        ).build();
        self.instrument_ids.insert(key, deposit.id);
        self.state.financial_system.instruments.insert(deposit.id, deposit.clone());
        deposit
    }
    // TODO merge with above
    fn get_or_create_reserves_instrument(&mut self) -> Instrument {
        let key = "RESERVES".to_string();
        if let Some(inst_id) = self.instrument_ids.get(&key) {
            return self.state.financial_system.instruments.get(inst_id).unwrap().clone();
        }
        let cb_id = self.state.financial_system.central_bank.id;
        let reserves = Instrument::cash(
            InstrumentId(Uuid::new_v4()),
            cb_id,
            CashType::CentralBankReserves,
            Currency::USD,
            self.state.financial_system.central_bank.policy_rate_bps
        ).build();
        self.instrument_ids.insert(key, reserves.id);
        self.state.financial_system.instruments.insert(reserves.id, reserves.clone());
        reserves
    }

    fn get_or_create_tga_instrument(&mut self) -> Instrument {
        let key = "TGA".to_string();
        if let Some(inst_id) = self.instrument_ids.get(&key) {
            return self.state.financial_system.instruments.get(inst_id).unwrap().clone();
        }
        let cb_id = self.state.financial_system.central_bank.id;
        let tga = Instrument::cash(
            InstrumentId(Uuid::new_v4()),
            cb_id,
            CashType::TreasuryGeneralAccount,
            Currency::USD,
            dec!(0.0)
        ).build();
        self.instrument_ids.insert(key, tga.id);
        self.state.financial_system.instruments.insert(tga.id, tga.clone());
        tga
    }

    fn get_or_create_bond_instrument(&mut self, tenor: &str, issuer_id: AgentId) -> Instrument {
        let key = format!("BOND_{}_{}", tenor, issuer_id);
        if let Some(inst_id) = self.instrument_ids.get(&key) {
            return self.state.financial_system.instruments.get(inst_id).unwrap().clone();
        }
        let issue = self.state.current_date;
        let maturity = issue + Duration::days(365 * 2);
        let bond = Instrument::bond(
            InstrumentId(Uuid::new_v4()),
            issuer_id,
            BondType::Government,
            Money::from(1000),
            issue,
            maturity
        ).coupon_bps(dec!(250.0)).rating(CreditRating::government_aaa()).auto_market().build().unwrap();

        self.instrument_ids.insert(key, bond.id);
        self.state.financial_system.instruments.insert(bond.id, bond.clone());
        bond
    }

    pub fn get_agent_id_map(&self) -> &HashMap<String, AgentId> {
        &self.agent_ids
    }
}