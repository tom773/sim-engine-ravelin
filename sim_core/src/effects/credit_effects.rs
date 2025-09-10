use crate::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CreditEffect {
    RegisterFacility {
        facility: CreditFacility,
    },

    RegisterLoan {
        loan: Loan,
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
            CreditEffect::RegisterFacility { facility } => {
                state
                    .financial_system
                    .credit_registry
                    .register_facility(facility.clone())
                    .map_err(EffectError::FinancialSystemError)?;

                let inst = Instrument {
                    id: facility.instrument_id,
                    instrument_type: InstrumentType::Debt(DebtInstrument::CreditLine(facility.details.clone())),
                    instrument_market: InstrumentMarket::MoneyMarket(MoneyMarketSegment::CorporateShortTerm),
                    listability: Listability::Unlisted,
                };

                state.financial_system.instruments.insert(facility.instrument_id, inst);
                Ok(())
            }

            CreditEffect::RegisterLoan { loan } => {
                state
                    .financial_system
                    .credit_registry
                    .register_loan(loan.clone())
                    .map_err(EffectError::FinancialSystemError)?;

                let inst = Instrument {
                    id: loan.instrument_id,
                    instrument_type: InstrumentType::Debt(DebtInstrument::Loan(loan.details.clone())),
                    instrument_market: InstrumentMarket::MoneyMarket(MoneyMarketSegment::CorporateShortTerm),
                    listability: Listability::Unlisted,
                };

                state.financial_system.instruments.insert(loan.instrument_id, inst);

                let lender_bs = state
                    .financial_system
                    .balance_sheets
                    .entry(loan.details.lender)
                    .or_insert_with(|| BalanceSheet::new(loan.details.lender));
                // TODO Book value should be principal, quantity 1
                lender_bs.assets.insert(
                    loan.instrument_id,
                    Position {
                        quantity: loan.details.outstanding_principal.to_f64(),
                        book_value_per_unit: Money::ONE,
                        cost_basis_per_unit: Money::ONE,
                    },
                );

                let borrower_bs = state
                    .financial_system
                    .balance_sheets
                    .entry(loan.details.borrower)
                    .or_insert_with(|| BalanceSheet::new(loan.details.borrower));

                borrower_bs.liabilities.insert(
                    loan.instrument_id,
                    Position {
                        quantity: loan.details.outstanding_principal.to_f64(),
                        book_value_per_unit: Money::ONE,
                        cost_basis_per_unit: Money::ONE,
                    },
                );

                Ok(())
            }

            CreditEffect::UpdateFacilityUtilization { facility_id, drawn_amount, available_amount } => {
                if let Some(facility) = state.financial_system.credit_registry.facilities.get_mut(facility_id) {
                    facility.details.drawn_amount = *drawn_amount;
                    facility.details.available_amount = *available_amount;
                }
                Ok(())
            }

            CreditEffect::ProcessLoanPayment { loan_id, principal_paid, interest_paid, fees_paid, payment_date } => {
                if let Some(loan) = state.financial_system.credit_registry.loans.get_mut(loan_id) {
                    loan.details.outstanding_principal -= *principal_paid;

                    loan.servicing_history.push(PaymentRecord {
                        payment_date: *payment_date,
                        due_date: loan.details.next_payment_date,
                        amount: (*principal_paid + *interest_paid + *fees_paid).to_f64(),
                        principal_paid: principal_paid.to_f64(),
                        interest_paid: interest_paid.to_f64(),
                    });

                    loan.details.next_payment_date = match loan.details.payment_frequency {
                        PaymentFrequency::Monthly => add_months(loan.details.next_payment_date, 1),
                        PaymentFrequency::Quarterly => add_months(loan.details.next_payment_date, 3),
                        PaymentFrequency::SemiAnnual => add_months(loan.details.next_payment_date, 6),
                        PaymentFrequency::Annual => add_months(loan.details.next_payment_date, 12),
                        PaymentFrequency::InterestOnly => add_months(loan.details.next_payment_date, 1),
                    };

                    if let Some(lender_bs) = state.financial_system.balance_sheets.get_mut(&loan.details.lender) {
                        if let Some(pos) = lender_bs.assets.get_mut(&loan.instrument_id) {
                            pos.quantity = loan.details.outstanding_principal.to_f64();
                        }
                        lender_bs.income_statement.interest_income += *interest_paid;
                    }

                    if let Some(borrower_bs) = state.financial_system.balance_sheets.get_mut(&loan.details.borrower) {
                        if let Some(pos) = borrower_bs.liabilities.get_mut(&loan.instrument_id) {
                            pos.quantity = loan.details.outstanding_principal.to_f64();
                        }
                        borrower_bs.income_statement.interest_expense += *interest_paid;
                    }
                }
                Ok(())
            }

            CreditEffect::AccrueLoanInterest { loan_id, interest_amount, accrual_date } => {
                if let Some(loan) = state.financial_system.credit_registry.loans.get_mut(loan_id) {
                    loan.details.accrued_interest += *interest_amount;
                    loan.details.last_accrual_date = *accrual_date;
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
                    if let Some(lender_bs) = state.financial_system.balance_sheets.get_mut(&loan.details.lender) {
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
                            .get_mut(&loan.details.borrower)
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
                    let _old_status = lien.status;
                    lien.status = *new_status;

                    if *new_status == LienStatus::Released {
                        if let CollateralType::Securities { instrument_id, quantity, .. } = &lien.collateral_type {
                            if let Some(loan) = state.financial_system.instruments.get(&lien.secured_obligation) {
                                if let InstrumentType::Debt(DebtInstrument::Loan(details)) = &loan.instrument_type {
                                    if let Some(holding) = state
                                        .financial_system
                                        .clearing_house
                                        .csd
                                        .custody_accounts
                                        .get_mut(&details.borrower)
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
                state.financial_system.credit_registry.applications_by_borrower.entry(borrower_id).or_default().push(app_id);
                Ok(())
            }
        }
    }

}
