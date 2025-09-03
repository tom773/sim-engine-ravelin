use crate::prelude::*;
use crate::types::money::Money;
use chrono::{Months, NaiveDate};
use rust_decimal::prelude::*; // Import ToPrimitive trait
use rust_decimal_macros::dec;
use std::collections::HashSet;
use uuid::Uuid;

impl FinancialSystem {
    pub fn get_parties(&self, instrument_id: &InstrumentId) -> Option<(AgentId, AgentId)> {
        let instrument = self.instruments.get(instrument_id)?;
        let debtor_id = instrument.get_consolidation_key().issuer;

        let creditor_id = self
            .balance_sheets
            .iter()
            .find(|(_agent_id, bs)| bs.assets.contains_key(instrument_id))
            .map(|(agent_id, _bs)| *agent_id)?;

        Some((creditor_id, debtor_id))
    }
}

impl FinancialSystem {
    pub fn create_loan(
        &mut self,
        lender: AgentId,
        borrower: AgentId,
        amount: u64,
        interest_bps: f64,
        term_months: u32,
        date: NaiveDate,
    ) -> Result<InstrumentId, String> {
        let loan_instrument = Instrument::bond(
            InstrumentId(Uuid::new_v4()),
            borrower, // The borrower is the issuer of the debt
            BondType::Corporate,
            Money::from(amount), // Convert u64 to Money
            date,
            date.checked_add_months(Months::new(term_months)).unwrap(),
        )
        .coupon_bps(Decimal::from_f64(interest_bps).unwrap_or(dec!(0))) // Convert f64 to Decimal
        .frequency(12)
        .rating(CreditRating::B) // Default rating for a simple loan
        .build()
        .map_err(|e| e.to_string())?;

        let loan_id = loan_instrument.id;

        self.create_instrument(lender, borrower, loan_instrument, 1.0, amount as f64)?;


        Ok(loan_id)
    }

    pub fn pay_interest(
        &mut self,
        instrument_id: &InstrumentId,
        payment_date: NaiveDate,
    ) -> Result<(), String> {
        let instrument = self.instruments.get_mut(instrument_id).ok_or("Instrument not found")?;

        let (_debtor, _accrued_interest_payment) = {
            let bond_details = match &mut instrument.instrument_type {
                InstrumentType::Bond(d) => d,
                _ => return Err("Instrument is not a bond/loan.".to_string()),
            };

            let annual_rate = bps_to_decimal(bond_details.coupon_rate_bps);
            let face_value_decimal =
                Decimal::from_f64(bond_details.face_value.to_f64()).unwrap_or(dec!(0));
            let frequency_decimal = Decimal::from(bond_details.frequency);

            let payment_decimal = annual_rate * face_value_decimal / frequency_decimal;
            let payment = payment_decimal.to_f64().unwrap_or(0.0);

            bond_details.last_accrual_date = Some(payment_date);
            (bond_details.issuer, payment)
        };

        let _creditor = self
            .balance_sheets
            .values()
            .find(|bs| bs.assets.contains_key(instrument_id))
            .map(|bs| bs.agent_id)
            .ok_or("Could not find a creditor for the instrument.")?;

        Ok(())
    }


    pub fn enter_repo_agreement(
        &mut self,
        lender: AgentId,
        borrower: AgentId,
        collateral_id: InstrumentId,
        collateral_quantity: f64,
        interest_bps: BasisPoints,
        start_date: NaiveDate,
        end_date: NaiveDate,
        haircut: f64,
    ) -> Result<InstrumentId, String> {
        if haircut < 0.0 || haircut > 1.0 {
            return Err("Haircut must be between 0.0 and 1.0".to_string());
        }

        let borrower_bs = self.balance_sheets.get(&borrower).ok_or("Borrower not found")?;
        let collateral_pos = borrower_bs
            .assets
            .get(&collateral_id)
            .ok_or("Collateral not found on borrower's BS")?;
        if collateral_pos.quantity < collateral_quantity {
            return Err("Insufficient collateral".to_string());
        }

        let collateral_value = collateral_pos.book_value_per_unit * collateral_quantity;
        let haircut_decimal = Decimal::from_f64(1.0 - haircut).unwrap_or(dec!(1));
        let loan_amount = collateral_value * haircut_decimal;

        let haircut_rate = Decimal::from_f64(haircut).unwrap_or(dec!(0));
        let repo_details = RepoDetails {
            lender,
            borrower,
            collateral_id,
            collateral_quantity,
            loan_amount,
            interest_bps,
            start_date,
            end_date,
            haircut: haircut_rate,
        };

        let repo_instrument = Instrument::new(
            InstrumentId(Uuid::new_v4()),
            InstrumentType::Repo(repo_details),
            InstrumentMarket::MoneyMarket(MoneyMarketSegment::Repo),
        );
        let repo_id = repo_instrument.id;

        self.instruments.insert(repo_id, repo_instrument);


        let lender_bs = self.balance_sheets.get_mut(&lender).ok_or("Lender not found")?;
        lender_bs.assets.insert(
            repo_id,
            Position {
                quantity: 1.0,
                book_value_per_unit: loan_amount,
                cost_basis_per_unit: loan_amount, // <-- Add this line
            },
        );

        let borrower_bs = self.balance_sheets.get_mut(&borrower).ok_or("Borrower not found")?;
        borrower_bs.liabilities.insert(
            repo_id,
            Position {
                quantity: 1.0,
                book_value_per_unit: loan_amount,
                cost_basis_per_unit: loan_amount, // <-- Add this line
            },
        );
        Ok(repo_id)
    }

    pub fn unwind_repo(&mut self, repo_id: InstrumentId) -> Result<(), String> {
        let details = if let Some(instrument) = self.instruments.get(&repo_id) {
            if let InstrumentType::Repo(d) = &instrument.instrument_type {
                Ok(d.clone())
            } else {
                Err("Instrument is not a Repo".to_string())
            }
        } else {
            Err("Repo instrument not found".to_string())
        }?;

        let duration_days = (details.end_date - details.start_date).num_days() as f64;
        let duration_years = duration_days / 360.0;

        let annual_rate = bps_to_decimal(details.interest_bps);
        let duration_rate = annual_rate * Decimal::from_f64(duration_years).unwrap_or(dec!(0));
        let interest = details.loan_amount * duration_rate;
        let _repayment_amount = details.loan_amount + interest;


        self.instruments.remove(&repo_id);
        self.balance_sheets
            .get_mut(&details.lender)
            .ok_or("Lender not found")?
            .assets
            .remove(&repo_id);
        self.balance_sheets
            .get_mut(&details.borrower)
            .ok_or("Borrower not found")?
            .liabilities
            .remove(&repo_id);

        Ok(())
    }
}

impl FinancialSystem {
    pub fn create_instrument(
        &mut self,
        creditor_id: AgentId,
        debtor_id: AgentId,
        instrument: Instrument,
        quantity: f64,
        book_value_per_unit: f64,
    ) -> Result<(), String> {
        let inst_id = instrument.id;

        if self.instruments.contains_key(&inst_id) {
            return Err("Instrument with this ID already exists.".to_string());
        }
        self.instruments.insert(inst_id, instrument);

        let book_value_money = Money::from_f64(book_value_per_unit).unwrap_or(Money::ZERO);
        let creditor_bs =
            self.balance_sheets.get_mut(&creditor_id).ok_or("Creditor not found")?;
        creditor_bs.assets.insert(
            inst_id,
            Position {
                quantity,
                book_value_per_unit: book_value_money,
                cost_basis_per_unit: book_value_money, // <-- Add this line
            },
        );

        let debtor_bs = self.balance_sheets.get_mut(&debtor_id).ok_or("Debtor not found")?;
        debtor_bs.liabilities.insert(
            inst_id,
            Position {
                quantity,
                book_value_per_unit: book_value_money,
                cost_basis_per_unit: book_value_money, // <-- Add this line
            },
        );

        Ok(())
    }

    pub fn find_consolidatable_instrument_id(
        &self,
        new_inst: &Instrument,
        owner_id: &AgentId,
    ) -> Option<InstrumentId> {
        let new_key = new_inst.get_consolidation_key();
        if let Some(owner_bs) = self.balance_sheets.get(owner_id) {
            for inst_id in owner_bs.assets.keys() {
                if let Some(existing_inst) = self.instruments.get(inst_id) {
                    if existing_inst.get_consolidation_key() == new_key {
                        return Some(*inst_id);
                    }
                }
            }
        }
        None
    }

    pub fn create_or_consolidate_instrument(
        &mut self,
        creditor_id: AgentId,
        debtor_id: AgentId,
        instrument: Instrument,
        quantity_change: f64,
        book_value_change: f64,
    ) -> Result<InstrumentId, String> {
        if let Some(existing_id) =
            self.find_consolidatable_instrument_id(&instrument, &creditor_id)
        {
            let creditor_bs = self.balance_sheets.get_mut(&creditor_id).unwrap();
            let asset_pos = creditor_bs.assets.entry(existing_id).or_default();
            asset_pos.quantity += quantity_change;

            let debtor_bs = self.balance_sheets.get_mut(&debtor_id).unwrap();
            let liability_pos = debtor_bs.liabilities.entry(existing_id).or_default();
            liability_pos.quantity += quantity_change;

            Ok(existing_id)
        } else {
            let id = instrument.id;
            self.create_instrument(
                creditor_id,
                debtor_id,
                instrument,
                quantity_change,
                book_value_change,
            )?;
            Ok(id)
        }
    }

    pub fn remove_instrument(&mut self, id: &InstrumentId) -> Result<(), String> {
        if let Some(instrument) = self.instruments.remove(id) {
            let debtor = instrument.get_consolidation_key().issuer; // Issuer is the debtor

            let creditor = self
                .balance_sheets
                .values()
                .find(|bs| bs.assets.contains_key(id))
                .map(|bs| bs.agent_id);

            if let Some(creditor_id) = creditor {
                self.balance_sheets
                    .get_mut(&creditor_id)
                    .and_then(|bs| bs.assets.remove(id));
            }

            self.balance_sheets.get_mut(&debtor).and_then(|bs| bs.liabilities.remove(id));
            Ok(())
        } else {
            Err("Instrument not found".to_string())
        }
    }

    pub fn transfer_instrument(
        &mut self,
        instrument_id: &InstrumentId,
        old_creditor: AgentId,
        new_creditor: AgentId,
    ) -> Result<(), String> {
        let position = self
            .balance_sheets
            .get_mut(&old_creditor)
            .ok_or("Old creditor not found")?
            .assets
            .remove(instrument_id)
            .ok_or("Instrument not found on old creditor's balance sheet")?;

        self.balance_sheets
            .get_mut(&new_creditor)
            .ok_or("New creditor not found")?
            .assets
            .insert(*instrument_id, position);

        Ok(())
    }
}

impl FinancialSystem {
    pub fn get_bs_by_id(&self, agent_id: &AgentId) -> Option<&BalanceSheet> {
        self.balance_sheets.get(agent_id)
    }
    pub fn get_bs_mut_by_id(&mut self, agent_id: &AgentId) -> Option<&mut BalanceSheet> {
        self.balance_sheets.get_mut(agent_id)
    }
    pub fn get_total_assets(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map_or(0.0, |bs| bs.total_assets(self))
    }
    pub fn get_total_liabilities(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map_or(0.0, |bs| bs.total_liabilities(self))
    }
    pub fn get_liquid_assets(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map_or(0.0, |bs| bs.liquid_assets(self).to_f64())
    }
    pub fn get_total_deposits(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map_or(0.0, |bs| bs.total_deposits(self))
    }

    pub fn get_cash_assets(&self, agent_id: &AgentId) -> f64 {
        self.get_bs_by_id(agent_id).map_or(0.0, |bs| {
            bs.assets
                .iter()
                .filter_map(|(id, pos)| {
                    self.instruments.get(id).and_then(|inst| {
                        if let InstrumentType::Cash(d) = &inst.instrument_type {
                            if matches!(d.cash_type, CashType::Currency) {
                                return Some(pos.quantity);
                            }
                        }
                        None
                    })
                })
                .sum()
        })
    }

    pub fn get_bank_reserves(&self, agent_id: &AgentId) -> Option<f64> {
        self.get_bs_by_id(agent_id).map(|bs| {
            bs.assets
                .iter()
                .filter_map(|(id, pos)| {
                    self.instruments.get(id).and_then(|inst| {
                        if let InstrumentType::Cash(d) = &inst.instrument_type {
                            if matches!(d.cash_type, CashType::Currency | CashType::CentralBankReserves)
                            {
                                return Some(pos.quantity);
                            }
                        }
                        None
                    })
                })
                .sum()
        })
    }
}

impl FinancialSystem {
    pub fn m0(&self) -> f64 {
        self.balance_sheets.get(&self.central_bank.id).map_or(0.0, |bs| bs.total_liabilities(self))
    }

    pub fn m1(&self, bank_ids: &HashSet<AgentId>) -> f64 {
        self.balance_sheets
            .iter()
            .filter(|(id, _)| !bank_ids.contains(id) && **id != self.central_bank.id)
            .map(|(_, bs)| {
                bs.assets
                    .iter()
                    .filter_map(|(id, pos)| {
                        self.instruments.get(id).and_then(|inst| {
                            if let InstrumentType::Cash(d) = &inst.instrument_type {
                                if matches!(d.cash_type, CashType::Currency | CashType::DemandDeposit)
                                {
                                    return Some(pos.quantity);
                                }
                            }
                            None
                        })
                    })
                    .sum::<f64>()
            })
            .sum()
    }

    pub fn m2(&self, bank_ids: &HashSet<AgentId>) -> f64 {
        let m1 = self.m1(bank_ids);
        let savings: f64 = self
            .balance_sheets
            .iter()
            .filter(|(id, _)| !bank_ids.contains(id) && **id != self.central_bank.id)
            .map(|(_, bs)| {
                bs.assets
                    .iter()
                    .filter_map(|(id, pos)| {
                        self.instruments.get(id).and_then(|inst| {
                            if let InstrumentType::Cash(d) = &inst.instrument_type {
                                if matches!(d.cash_type, CashType::SavingsDeposit) {
                                    return Some(pos.quantity);
                                }
                            }
                            None
                        })
                    })
                    .sum::<f64>()
            })
            .sum();
        m1 + savings
    }

    pub fn all_bank_assets(&self, bank_ids: &HashSet<AgentId>) -> f64 {
        self.balance_sheets
            .iter()
            .filter(|(id, _)| bank_ids.contains(id))
            .map(|(_, bs)| bs.total_assets(self))
            .sum()
    }

    pub fn all_bank_reserves(&self, bank_ids: &HashSet<AgentId>) -> f64 {
        bank_ids.iter().filter_map(|id| self.get_bank_reserves(id)).sum()
    }

    pub fn all_bank_deposits(&self, bank_ids: &HashSet<AgentId>) -> f64 {
        self.balance_sheets
            .iter()
            .filter(|(id, _)| bank_ids.contains(id))
            .map(|(_, bs)| {
                bs.liabilities
                    .iter()
                    .filter_map(|(id, pos)| {
                        self.instruments.get(id).and_then(|inst| {
                            if let InstrumentType::Cash(d) = &inst.instrument_type {
                                if matches!(d.cash_type, CashType::DemandDeposit | CashType::SavingsDeposit)
                                {
                                    return Some(pos.quantity);
                                }
                            }
                            None
                        })
                    })
                    .sum::<f64>()
            })
            .sum()
    }
}