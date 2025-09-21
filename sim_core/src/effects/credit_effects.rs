use crate::types::instrument::credit::ConsumerLoanCategory;
use crate::types::instrument::{
    CashType, CreditState, Currency, InstrumentIdentifiers, InstrumentRuntime, Listability, MarketProfile,
};
use crate::*;
use chrono::NaiveDate;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CreditEffect {
    RegisterFacility {
        facility: CreditFacility,
    },

    RegisterLoan {
        loan: Loan,
        is_consumer: bool,
        purpose: LoanPurpose,
    },

    UpdateFacilityUtilization {
        facility_id: Uuid,
        drawn_amount: Money,
        available_amount: Money,
    },
    RecordLoanApplication {
        bank_id: AgentId,
        application: LoanApplication,
    },
    ProcessLoanPayment {
        loan_id: Uuid,
        principal_paid: Money,
        interest_paid: Money,
        fees_paid: Money,
        payment_date: NaiveDate,
    },

    AccrueLoanInterest {
        loan_id: Uuid,
        interest_amount: Money,
        accrual_date: NaiveDate,
    },

    RecordImpairment {
        loan_id: Uuid,
        stage: ImpairmentStage,
        provision: Money,
    },

    RecordLien {
        lien: Lien,
        loan_id: Uuid,
    },

    UpdateLienStatus {
        lien_id: LienId,
        new_status: LienStatus,
    },
}

impl CreditEffect {
    pub fn name(&self) -> String {
        match self {
            CreditEffect::RegisterFacility { .. } => "RegisterFacility".to_string(),
            CreditEffect::RegisterLoan { .. } => "RegisterLoan".to_string(),
            CreditEffect::UpdateFacilityUtilization { .. } => "UpdateFacilityUtilization".to_string(),
            CreditEffect::ProcessLoanPayment { .. } => "ProcessLoanPayment".to_string(),
            CreditEffect::AccrueLoanInterest { .. } => "AccrueLoanInterest".to_string(),
            CreditEffect::RecordImpairment { .. } => "RecordImpairment".to_string(),
            CreditEffect::RecordLien { .. } => "RecordLien".to_string(),
            CreditEffect::UpdateLienStatus { .. } => "UpdateLienStatus".to_string(),
            CreditEffect::RecordLoanApplication { .. } => "RecordLoanApplication".to_string(),
        }
    }
}
impl StateEffectApplicator {
    pub fn apply_credit_effect(state: &mut SimState, effect: &CreditEffect) -> Result<(), EffectError> {
        match effect {
            CreditEffect::RegisterLoan { loan, is_consumer, purpose } => {
                let fs = &mut state.financial_system;

                let credit_state = if *is_consumer {
                    let category = match purpose {
                        LoanPurpose::RealEstate => ConsumerLoanCategory::ResidentialMortgage,
                        LoanPurpose::Equipment => ConsumerLoanCategory::AutoLoan,
                        LoanPurpose::PersonalConsumption => ConsumerLoanCategory::PersonalLoan,
                        LoanPurpose::Refinancing => ConsumerLoanCategory::PersonalLoan,
                        _ => ConsumerLoanCategory::PersonalLoan,
                    };
                    CreditState::ConsumerLoan { category, loan: loan.state.clone() }
                } else {
                    CreditState::Loan(loan.state.clone())
                };

                let instrument = Instrument::new(
                    InstrumentIdentifiers::from(loan.instrument_id),
                    MarketProfile::unlisted(),
                    Listability::Unlisted,
                    InstrumentRuntime::Credit(credit_state),
                );

                let instrument_id = loan.instrument_id;
                fs.instruments.insert(instrument_id, instrument.clone());

                let cr = &mut fs.credit_registry;
                cr.register_loan(loan.clone()).map_err(EffectError::FinancialSystemError)?;

                let lender = loan.state.lender;
                let borrower = loan.state.borrower;
                let outstanding = loan.state.outstanding_principal.to_f64();

                let lender_bs = fs.balance_sheets.entry(lender).or_insert_with(|| BalanceSheet::new(lender));
                lender_bs.assets.insert(
                    instrument_id,
                    Position {
                        quantity: outstanding,
                        book_value_per_unit: Money::ONE,
                        cost_basis_per_unit: Money::ONE,
                    },
                );

                let borrower_bs = fs.balance_sheets.entry(borrower).or_insert_with(|| BalanceSheet::new(borrower));
                borrower_bs.liabilities.insert(
                    instrument_id,
                    Position {
                        quantity: outstanding,
                        book_value_per_unit: Money::ONE,
                        cost_basis_per_unit: Money::ONE,
                    },
                );

                let deposit_instrument_id = fs
                    .instruments
                    .instruments
                    .iter()
                    .find_map(|(id, inst)| {
                        if let InstrumentRuntime::Cash(details) = inst.state() {
                            if details.issuer == lender && details.cash_type == CashType::DemandDeposit {
                                return Some(*id);
                            }
                        }
                        None
                    })
                    .unwrap_or_else(|| {
                        let new_deposit = Instrument::cash(
                            InstrumentId(Uuid::new_v4()),
                            lender,
                            CashType::DemandDeposit,
                            Currency::USD,
                            dec!(25.0),
                        )
                        .build();
                        let id = new_deposit.instrument_id();
                        fs.instruments.insert(id, new_deposit.clone());
                        id
                    });

                let borrower_assets = fs.balance_sheets.get_mut(&borrower).unwrap();
                let deposit_pos = borrower_assets.assets.entry(deposit_instrument_id).or_insert_with(|| Position {
                    quantity: 0.0,
                    book_value_per_unit: Money::ONE,
                    cost_basis_per_unit: Money::ONE,
                });
                deposit_pos.quantity += outstanding;

                let lender_liabilities = fs.balance_sheets.get_mut(&lender).unwrap();
                let deposit_liability =
                    lender_liabilities.liabilities.entry(deposit_instrument_id).or_insert_with(|| Position {
                        quantity: 0.0,
                        book_value_per_unit: Money::ONE,
                        cost_basis_per_unit: Money::ONE,
                    });
                deposit_liability.quantity += outstanding;

                if let Some(app_ids) = cr.applications_by_borrower.remove(&borrower) {
                    for app_id in app_ids {
                        cr.applications.remove(&app_id);

                        for ids in cr.applications_by_bank.values_mut() {
                            if let Some(pos) = ids.iter().position(|id| *id == app_id) {
                                ids.swap_remove(pos);
                            }
                        }
                    }
                }

                Ok(())
            }

            CreditEffect::RegisterFacility { facility } => {
                state
                    .financial_system
                    .credit_registry
                    .register_facility(facility.clone())
                    .map_err(EffectError::FinancialSystemError)?;

                let profile =
                    MarketProfile::from_market(InstrumentMarket::MoneyMarket(MoneyMarketSegment::CorporateShortTerm));
                let instrument = Instrument::new(
                    InstrumentIdentifiers::from(facility.instrument_id),
                    profile,
                    Listability::Unlisted,
                    InstrumentRuntime::Credit(CreditState::Facility(facility.state.clone())),
                );

                state.financial_system.instruments.insert(facility.instrument_id, instrument.clone());
                Ok(())
            }

            CreditEffect::UpdateFacilityUtilization { facility_id, drawn_amount, available_amount } => {
                if let Some(facility) = state.financial_system.credit_registry.facilities.get_mut(facility_id) {
                    facility.state.drawn_amount = *drawn_amount;
                    facility.state.available_amount = *available_amount;
                }
                Ok(())
            }

            CreditEffect::ProcessLoanPayment { loan_id, principal_paid, interest_paid, fees_paid, payment_date } => {
                if let Some(loan) = state.financial_system.credit_registry.loans.get_mut(loan_id) {
                    loan.state.outstanding_principal -= *principal_paid;

                    loan.servicing_history.push(PaymentRecord {
                        payment_date: *payment_date,
                        due_date: loan.state.next_payment_date,
                        amount: (*principal_paid + *interest_paid + *fees_paid).to_f64(),
                        principal_paid: principal_paid.to_f64(),
                        interest_paid: interest_paid.to_f64(),
                    });

                    let next_payment = loan.state.next_payment_date;
                    let frequency = loan.state.archetype.repayment_schedule.payment_frequency;
                    loan.state.next_payment_date = match frequency {
                        PaymentFrequency::Monthly => add_months(next_payment, 1),
                        PaymentFrequency::Quarterly => add_months(next_payment, 3),
                        PaymentFrequency::SemiAnnual => add_months(next_payment, 6),
                        PaymentFrequency::Annual => add_months(next_payment, 12),
                        PaymentFrequency::InterestOnly => add_months(next_payment, 1),
                    };

                    if let Some(lender_bs) = state.financial_system.balance_sheets.get_mut(&loan.state.lender) {
                        if let Some(pos) = lender_bs.assets.get_mut(&loan.instrument_id) {
                            pos.quantity = loan.state.outstanding_principal.to_f64();
                            pos.book_value_per_unit = Money::ONE;
                            pos.cost_basis_per_unit = Money::ONE;
                        }
                        lender_bs.income_statement.interest_income += *interest_paid;
                    }

                    if let Some(borrower_bs) = state.financial_system.balance_sheets.get_mut(&loan.state.borrower) {
                        if let Some(pos) = borrower_bs.liabilities.get_mut(&loan.instrument_id) {
                            pos.quantity = loan.state.outstanding_principal.to_f64();
                            pos.book_value_per_unit = Money::ONE;
                            pos.cost_basis_per_unit = Money::ONE;
                        }
                        borrower_bs.income_statement.interest_expense += *interest_paid;
                    }
                }
                Ok(())
            }

            CreditEffect::AccrueLoanInterest { loan_id, interest_amount, accrual_date } => {
                if let Some(loan) = state.financial_system.credit_registry.loans.get_mut(loan_id) {
                    loan.state.accrued_interest += *interest_amount;
                    loan.state.last_accrual_date = *accrual_date;
                }
                Ok(())
            }

            CreditEffect::RecordImpairment { loan_id, stage, provision } => {
                state
                    .financial_system
                    .credit_registry
                    .update_impairment(loan_id, *stage)
                    .map_err(EffectError::FinancialSystemError)?;

                if let Some(loan) = state.financial_system.credit_registry.loans.get(loan_id) {
                    if let Some(lender_bs) = state.financial_system.balance_sheets.get_mut(&loan.state.lender) {
                        lender_bs.income_statement.operating_expenses += *provision;
                    }
                }
                Ok(())
            }

            CreditEffect::RecordLien { lien, loan_id } => {
                if let CollateralType::Securities { instrument_id, quantity, .. } = &lien.collateral_type {
                    if let Some(loan) = state.financial_system.credit_registry.loans.get(loan_id) {
                        if let Some(holding) = state
                            .financial_system
                            .clearing_house
                            .csd
                            .custody_accounts
                            .get_mut(&loan.state.borrower)
                            .and_then(|acc| acc.holdings.get_mut(instrument_id))
                        {
                            if holding.available >= *quantity {
                                holding.available -= quantity;
                                holding.pledged += quantity;
                            }
                        }
                    }
                }

                state
                    .financial_system
                    .credit_registry
                    .create_lien(lien.clone(), *loan_id)
                    .map_err(EffectError::FinancialSystemError)?;
                Ok(())
            }

            CreditEffect::UpdateLienStatus { lien_id, new_status } => {
                if let Some(lien) = state.financial_system.credit_registry.liens.get_mut(lien_id) {
                    lien.status = *new_status;

                    if *new_status == LienStatus::Released {
                        if let CollateralType::Securities { instrument_id, quantity, .. } = &lien.collateral_type {
                            if let Some(instrument) =
                                state.financial_system.instruments.instruments.get(&lien.secured_obligation)
                            {
                                let borrower = match instrument.state() {
                                    InstrumentRuntime::Credit(CreditState::Loan(loan_state)) => {
                                        Some(loan_state.borrower)
                                    }
                                    InstrumentRuntime::Credit(CreditState::ConsumerLoan { loan, .. }) => {
                                        Some(loan.borrower)
                                    }
                                    InstrumentRuntime::Credit(CreditState::Facility(facility)) => {
                                        Some(facility.borrower)
                                    }
                                    InstrumentRuntime::Credit(CreditState::ConsumerCreditCard(facility)) => {
                                        Some(facility.borrower)
                                    }
                                    InstrumentRuntime::Credit(CreditState::TradeCredit(details)) => {
                                        Some(details.debtor)
                                    }
                                    _ => None,
                                };

                                if let Some(borrower_id) = borrower {
                                    if let Some(holding) = state
                                        .financial_system
                                        .clearing_house
                                        .csd
                                        .custody_accounts
                                        .get_mut(&borrower_id)
                                        .and_then(|acc| acc.holdings.get_mut(instrument_id))
                                    {
                                        holding.pledged = (holding.pledged - quantity).max(0.0);
                                        holding.available += quantity;
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(())
            }

            CreditEffect::RecordLoanApplication { bank_id, application } => {
                let app_id = application.application_id;
                let borrower_id = application.borrower_id;
                state.financial_system.credit_registry.applications.insert(app_id, application.clone());
                state.financial_system.credit_registry.applications_by_bank.entry(*bank_id).or_default().push(app_id);
                state
                    .financial_system
                    .credit_registry
                    .applications_by_borrower
                    .entry(borrower_id)
                    .or_default()
                    .push(app_id);
                Ok(())
            }
        }
    }
}
