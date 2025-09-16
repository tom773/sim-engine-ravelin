use crate::scenario::{AssetConfig, BankConfig, ConsumerConfig, FirmConfig, LiabilityConfig};

use rand::prelude::*;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use uuid::Uuid;

use std::collections::HashMap;

use chrono::{Duration, NaiveDate};
use sim_core::prelude::*;

fn is_security(inst: &Instrument) -> bool {
    matches!(
        inst.instrument_type,
        InstrumentType::Debt(_)
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

pub struct SeedSale {
    pub buyer: AgentId,
    pub instrument_id: InstrumentId,
    pub quantity: f64,
    pub price_per_unit: Money,
}

pub struct BondIssuance {
    pub instrument: Instrument,
    pub total_quantity: f64,
}

pub fn issue_government_bonds(
    state: &mut SimState, bond_issuances: &[BondIssuance],
) -> Result<Vec<StateEffect>, EffectError> {
    let gov_id = state.financial_system.government.id;

    for issuance in bond_issuances {
        let inst = issuance.instrument.clone();
        let inst_id = inst.id;
        state.financial_system.instruments.insert(inst_id, inst.clone());

        state
            .financial_system
            .clearing_house
            .csd
            .register_security(inst_id, &inst, state.current_date)
            .map_err(|e| EffectError::FinancialSystemError(e.to_string()))?;

        state
            .financial_system
            .clearing_house
            .csd
            .credit_securities(gov_id, inst_id, issuance.total_quantity)
            .map_err(|e| EffectError::FinancialSystemError(e.to_string()))?;

        let fv = inst.face_value().unwrap_or(Money::from(1000));
        let gbs = state.financial_system.balance_sheets.entry(gov_id).or_insert_with(|| BalanceSheet::new(gov_id));
        gbs.liabilities.insert(
            inst_id,
            Position { quantity: issuance.total_quantity, book_value_per_unit: fv, cost_basis_per_unit: fv },
        );
    }

    Ok(vec![])
}

pub fn settle_seeded_primary_sales(state: &mut SimState, sales: &[SeedSale]) -> Result<(), EffectError> {
    if sales.is_empty() {
        return Ok(());
    }

    let gov_id = state.financial_system.government.id;
    let cb_id = state.financial_system.central_bank.id;

    let mut pending: HashMap<Uuid, SettlementInstruction> = HashMap::new();

    for s in sales {
        let instr = SettlementInstruction {
            instruction_id: Uuid::new_v4(),
            trade_id: Uuid::new_v4(),
            seller: gov_id,
            buyer: s.buyer,
            instrument_id: s.instrument_id,
            quantity: s.quantity,
            cash_amount: (s.price_per_unit * s.quantity).to_f64(),
            settlement_date: state.current_date,
            status: SettlementStatus::Pending,
        };

        let symbol = state
            .financial_system
            .exchange
            .inst_to_symbol
            .get(&s.instrument_id)
            .map(|x| x.0.clone())
            .unwrap_or_else(|| "UNKNOWN".to_string());

        state
            .financial_system
            .clearing_house
            .csd
            .reserve_securities_for_dvp(instr.clone(), gov_id, &symbol, state.current_date)
            .map_err(|e| EffectError::FinancialSystemError(e.to_string()))?;

        let (_buyer_liquid, buyer_bank) = state
            .financial_system
            .find_agent_liquid_account(&s.buyer)
            .ok_or_else(|| EffectError::InvalidState("Buyer has no liquid account".into()))?;
        let pi = PaymentInstruction {
            id: Uuid::new_v4(),
            payer: s.buyer,
            payee: gov_id,
            amount: (s.price_per_unit * s.quantity).to_f64(),
            from_bank: buyer_bank,
            to_bank: cb_id,
            context: TransactionContext::TradeSettlement { trade_id: instr.trade_id },
            priority: PaymentPriority::Normal,
            earliest_release_tick: state.ticknum,
            deadline_tick: state.ticknum + 10,
        };
        state.financial_system.rtgs.pending.push(pi);
        pending.insert(instr.trade_id, instr);
    }

    let finalization_effects = run_rtgs(state)?;
    for eff in finalization_effects {
        match eff {
            StateEffect::Financial(FinancialEffect::DvPFinalize { trade_id }) => {
                state
                    .financial_system
                    .clearing_house
                    .csd
                    .finalize_book_entry_transfer(&trade_id)
                    .map_err(|e| EffectError::FinancialSystemError(e.to_string()))?;
                pending.remove(&trade_id);
            }
            StateEffect::Financial(FinancialEffect::DvPCancel { trade_id }) => {
                state
                    .financial_system
                    .clearing_house
                    .csd
                    .cancel_security_reservation(&trade_id)
                    .map_err(|e| EffectError::FinancialSystemError(e.to_string()))?;
                pending.remove(&trade_id);
            }
            _ => {}
        }
    }

    for trade_id in pending.keys() {
        state
            .financial_system
            .clearing_house
            .csd
            .cancel_security_reservation(trade_id)
            .map_err(|e| EffectError::FinancialSystemError(e.to_string()))?;
    }
    Ok(())
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

        let initial_balance = Money::from_f64(30_000_000.0).unwrap();
        {
            let gbs = self.state.financial_system.balance_sheets.get_mut(&gov_id).unwrap();
            if let Some(p) = gbs.assets.get_mut(&tga_id) {
                p.book_value_per_unit = initial_balance;
            }

            let cbs = self.state.financial_system.balance_sheets.get_mut(&cb_id).unwrap();
            if let Some(p) = cbs.liabilities.get_mut(&tga_id) {
                p.book_value_per_unit = initial_balance;
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
            .credit_securities(cb_id, bond_id, initial_balance.to_f64() / 1000.0)
            .map_err(|e| e.to_string())?;

        let gbs = self.state.financial_system.balance_sheets.get_mut(&gov_id).unwrap();
        gbs.liabilities.insert(
            bond_id,
            Position {
                quantity: (initial_balance / Money::from(1000)).to_f64().unwrap(),
                book_value_per_unit: Money::from(1000),
                cost_basis_per_unit: Money::from(1000),
            },
        );

        Ok(())
    }

    pub fn create_bank(&mut self, config: &BankConfig, cb_id: AgentId) -> Bank {
        let bank = Bank::new(config.name.clone(), dec!(200.0), dec!(-70.0));
        self.state.financial_system.balance_sheets.insert(bank.id, BalanceSheet::new(bank.id));
        self.ensure_reserves_account(bank.id); // always present

        for a in &config.initial_assets {
            match a {
                AssetConfig::Reserves { amount } if *amount > 0.0 => {
                    self.seed_reserves_to_bank_from_tga(bank.id, *amount).unwrap_or_else(|e| {
                        tracing::warn!("TGA->bank reserves seed failed: {} (falling back to CB creation)", e);
                        self.seed_reserves_via_cb_creation(bank.id, *amount).expect("CB reserve creation failed");
                    });
                }
                _ => {
                    let _ = self.create_asset_for_agent(bank.id, a, cb_id, &HashMap::new(), &mut HashMap::new());
                }
            }
        }
        for l in &config.initial_liabilities {
            self.create_liability_for_agent(bank.id, l, cb_id, &HashMap::new()).unwrap();
        }

        self.state.agents.banks.insert(bank.id, bank.clone());
        bank
    }

    pub fn create_consumer(
        &mut self, config: &ConsumerConfig, bank_id: AgentId, cb_id: AgentId, agent_ids: &HashMap<String, AgentId>,
        count: usize, bond_instruments: &mut HashMap<String, InstrumentId>,
    ) -> (Consumer, Vec<SeedSale>) {
        let personality = *[PersonalityArchetype::Balanced, PersonalityArchetype::Saver, PersonalityArchetype::Spender]
            .choose(self.rng)
            .unwrap();
        let is_golden = count == 0;
        let mut consumer = Consumer::new(self.rng.random_range(25..65), bank_id, personality, is_golden);
        consumer.income = config.income;

        self.state.financial_system.balance_sheets.insert(consumer.id, BalanceSheet::new(consumer.id));

        let mut sales = Vec::new();
        for a in &config.initial_assets {
            if let Ok(Some(sale)) = self.create_asset_for_agent(consumer.id, a, cb_id, agent_ids, bond_instruments) {
                sales.push(sale);
            }
        }
        for l in &config.initial_liabilities {
            self.create_liability_for_agent(consumer.id, l, cb_id, agent_ids).unwrap();
        }
        self.state.agents.consumers.insert(consumer.id, consumer.clone());
        (consumer, sales)
    }

    pub fn create_firm(
        &mut self, config: &FirmConfig, bank_id: AgentId, cb_id: AgentId, agent_ids: &HashMap<String, AgentId>,
        bond_instruments: &mut HashMap<String, InstrumentId>,
    ) -> (Firm, Vec<SeedSale>) {
        let recipe_id = config.recipe_name.as_deref().and_then(|s| self.resolve_recipe_id(s));
        let mut firm = Firm::new(bank_id, config.name.clone(), recipe_id, 25.0);
        if let Some(m) = config.desired_markup {
            firm.desired_markup = m;
        }

        self.state.financial_system.balance_sheets.insert(firm.id, BalanceSheet::new(firm.id));

        let mut sales = Vec::new();
        for a in &config.initial_assets {
            if !matches!(a, AssetConfig::Inventory { .. }) {
                if let Ok(Some(sale)) = self.create_asset_for_agent(firm.id, a, cb_id, agent_ids, bond_instruments) {
                    sales.push(sale);
                }
            }
        }
        for l in &config.initial_liabilities {
            self.create_liability_for_agent(firm.id, l, cb_id, agent_ids).unwrap();
        }

        self.state.agents.firms.insert(firm.id, firm.clone());
        (firm, sales)
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

    fn seed_reserves_to_bank_from_tga(&mut self, bank_id: AgentId, amount: f64) -> Result<(), String> {
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
        tga_pos.book_value_per_unit -= Money::from_f64(amount).unwrap();
        let r_liab = cb_bs.liabilities.entry(reserves_inst_id).or_insert_with(Default::default);
        r_liab.quantity += amount;
        r_liab.book_value_per_unit = Money::ONE;

        let gov_bs = self.state.financial_system.balance_sheets.get_mut(&gov_id).unwrap();
        let g_tga = gov_bs.assets.get_mut(&tga_id.0).ok_or("Gov missing TGA asset")?;
        g_tga.book_value_per_unit -= Money::from_f64(amount).unwrap();
        Ok(())
    }

    fn seed_reserves_via_cb_creation(&mut self, bank_id: AgentId, amount: f64) -> Result<(), String> {
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
        mode: SeedMode,
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
        let reserves_inst_id = self.ensure_reserves_account(bank_id);

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

            let rpos = bank_bs.assets.entry(reserves_inst_id).or_insert_with(Default::default);
            rpos.quantity += amount;
            rpos.book_value_per_unit = Money::ONE;
        }

        match mode {
            SeedMode::RtgsFromTreasury => {
                let tga_id = self
                    .state
                    .financial_system
                    .find_government_tga_account()
                    .ok_or_else(|| "TGA not initialized".to_string())?;

                let cb_id = self.state.financial_system.central_bank.id;
                let cb_bs = self.state.financial_system.balance_sheets.get_mut(&cb_id).unwrap();
                let tga = cb_bs.liabilities.get_mut(&tga_id.0).ok_or("CB missing TGA liability")?;
                tga.book_value_per_unit -= Money::from_f64(amount).unwrap();
                let r_liab = cb_bs.liabilities.entry(reserves_inst_id).or_insert_with(Default::default);
                r_liab.quantity += amount;
                r_liab.book_value_per_unit = Money::ONE;

                let gov_id = self.state.financial_system.government.id;
                let gov_bs = self.state.financial_system.balance_sheets.get_mut(&gov_id).unwrap();
                let g_tga = gov_bs.assets.get_mut(&tga_id.0).ok_or("Gov missing TGA asset")?;
                g_tga.book_value_per_unit -= Money::from_f64(amount).unwrap();
            }
            SeedMode::ReservesFromCB => {
                let cb_id = self.state.financial_system.central_bank.id;
                let cb_bs = self.state.financial_system.balance_sheets.get_mut(&cb_id).unwrap();
                let r_liab = cb_bs.liabilities.entry(reserves_inst_id).or_insert_with(Default::default);
                r_liab.quantity += amount;
                r_liab.book_value_per_unit = Money::ONE;
            }
        }

        Ok(())
    }

    pub fn create_liability_for_agent(
        &mut self, agent_id: AgentId, liability: &LiabilityConfig, _cb_id: AgentId,
        agent_ids: &HashMap<String, AgentId>,
    ) -> Result<(), String> {
        match liability {
            LiabilityConfig::Deposit { creditor_id, amount, .. } => {
                if self.state.agents.banks.contains_key(&agent_id) {
                    let depositor_id =
                        *agent_ids.get(creditor_id).ok_or_else(|| format!("Unknown agent: {}", creditor_id))?;
                    let bank_key = agent_ids
                        .iter()
                        .find_map(|(k, v)| if *v == agent_id { Some(k.clone()) } else { None })
                        .ok_or_else(|| "Bank agent not present in id map".to_string())?;
                    self.seed_deposit(depositor_id, &bank_key, *amount, agent_ids, SeedMode::RtgsFromTreasury)
                } else {
                    Err("Deposit liability configured for non-bank agent".to_string())
                }
            }
            LiabilityConfig::Loan { creditor_id, principal, rate_bps, maturity_days } => {
                let creditor_agent_id =
                    *agent_ids.get(creditor_id).ok_or_else(|| format!("Creditor ID {} not found", creditor_id))?;
                let issue_date = self.state.current_date;
                let maturity_date = issue_date + Duration::days(*maturity_days as i64);
                let loan_instrument = Instrument::bond(
                    InstrumentId(Uuid::new_v4()),
                    agent_id,
                    BondType::Corporate,
                    Money::from(*principal as u64),
                    issue_date,
                    maturity_date,
                )
                .coupon_bps(*rate_bps)
                .frequency(12)
                .rating(CreditRating::Corporate(SpCreditRating::BBB))
                .build()
                .map_err(|e| format!("Failed to build loan instrument: {}", e))?;
                self.create_and_register_instrument(creditor_agent_id, agent_id, loan_instrument, 1.0, *principal)
            }
        }
    }

    pub fn create_asset_for_agent(
        &mut self, agent_id: AgentId, asset: &AssetConfig, cb_id: AgentId, agent_ids: &HashMap<String, AgentId>,
        bond_instruments: &mut HashMap<String, InstrumentId>,
    ) -> Result<Option<SeedSale>, String> {
        match asset {
            AssetConfig::Cash { amount } => {
                let cash =
                    Instrument::cash(InstrumentId(Uuid::new_v4()), cb_id, CashType::Currency, Currency::USD, dec!(0.0))
                        .build();
                self.create_and_register_instrument(agent_id, cb_id, cash, *amount, 1.0)?;
                Ok(None)
            }
            AssetConfig::Deposit { bank_id, amount } => {
                self.seed_deposit(agent_id, bank_id, *amount, agent_ids, SeedMode::RtgsFromTreasury)?;
                Ok(None)
            }
            AssetConfig::Reserves { amount } => {
                if self.state.agents.banks.contains_key(&agent_id) {
                    self.seed_reserves_to_bank_from_tga(agent_id, *amount)
                        .or_else(|_| self.seed_reserves_via_cb_creation(agent_id, *amount))?;
                    Ok(None)
                } else {
                    Err("Non-bank cannot hold CB reserves".to_string())
                }
            }
            AssetConfig::Bond { tenor, quantity } => {
                let instrument_id = if let Some(id) = bond_instruments.get(tenor) {
                    *id
                } else {
                    let issue = self.state.current_date;
                    let maturity = parse_tenor_to_date(tenor, issue)?;
                    let coupon_bps = self.state.financial_system.central_bank.policy_rate_bps;

                    let builder = if (maturity - issue).num_days() <= 365 {
                        Instrument::bond(
                            InstrumentId(Uuid::new_v4()),
                            self.state.financial_system.government.id,
                            BondType::Government,
                            Money::from(1000),
                            issue,
                            maturity,
                        )
                        .zero_coupon_rate_bps()
                    } else {
                        Instrument::bond(
                            InstrumentId(Uuid::new_v4()),
                            self.state.financial_system.government.id,
                            BondType::Government,
                            Money::from(1000),
                            issue,
                            maturity,
                        )
                        .coupon_bps(coupon_bps)
                    };

                    let inst = builder.build();
                    let iid = inst.as_ref().unwrap().clone().id;
                    self.state.financial_system.instruments.insert(iid, inst.unwrap());
                    bond_instruments.insert(tenor.clone(), iid);
                    iid
                };

                let price = Money::from(1000);
                Ok(Some(SeedSale { buyer: agent_id, instrument_id, quantity: *quantity as f64, price_per_unit: price }))
            }
            AssetConfig::Inventory { .. } => Ok(None), // handled by scenario after firm exists
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
