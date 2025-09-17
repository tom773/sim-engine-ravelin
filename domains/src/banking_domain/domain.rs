use crate::{Any, Domain, DomainResult, ResolutionContext, ResolutionPhase, ResolutionResult, inventory};
use serde::{Deserialize, Serialize};
use sim_core::*;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankingDomain {}

impl BankingDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for BankingDomain {
    fn name(&self) -> &'static str {
        "Banking"
    }

    fn resolve_intention(&self, intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::Banking(BankingIntention::RequestLoan { agent_id, bank_id, amount, purpose, collateral }) => {
                self.resolve_loan_request(*agent_id, *bank_id, *amount, purpose.clone(), collateral.clone())
            }
            SimIntention::Banking(BankingIntention::ApproveLoan { bank_id, borrower_id, amount: _, terms }) => {
                self.resolve_loan_approval(*bank_id, *borrower_id, terms.clone())
            }

            SimIntention::Banking(BankingIntention::RejectLoan { bank_id, borrower_id, application_id, reason }) => {
                self.resolve_loan_rejection(*bank_id, *borrower_id, *application_id, reason.clone())
            }
            _ => return None,
        };

        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::Banking(BankingIntention::RequestLoan { .. })
            | SimIntention::Banking(BankingIntention::ApproveLoan { .. })
            | SimIntention::Banking(BankingIntention::RejectLoan { .. }) => Some(ResolutionPhase::Independent),
            SimIntention::Banking(BankingIntention::PostOvernightFundingQuote { .. }) => Some(ResolutionPhase::Market),
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let banking_action = match action {
            SimAction::Banking(action) => action,
            _ => return DomainResult::failure(vec!["Not a banking action".to_string()]),
        };

        if let Err(error) = self.validate(banking_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match banking_action {
            BankingAction::ExecuteInterbankLoan { lender_id, borrower_id, amount, rate_bps } => {
                self.execute_interbank_loan(*lender_id, *borrower_id, *amount, *rate_bps, state)
            }
            BankingAction::CreateLoanApplication { bank_id, application } => {
                self.execute_create_loan_application(*bank_id, application.clone())
            }

            BankingAction::ProcessLoanApplication { bank_id, application_id, decision } => {
                self.execute_process_loan_application(*bank_id, *application_id, decision.clone(), state)
            }

            BankingAction::OriginateLoan { lender_id, borrower_id, loan_terms, application_id } => {
                self.execute_loan_origination(*lender_id, *borrower_id, loan_terms.clone(), *application_id, state)
            }
            _ => DomainResult::empty(),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl BankingDomain {
    fn validate(&self, action: &BankingAction, state: &SimState) -> Result<(), String> {
        match action {
            BankingAction::PostInterbankLendingOffer { lender_id, amount, .. } => {
                Validator::positive_amount(*amount)?;
                Validator::bank_exists(*lender_id, state)
            }
            BankingAction::PostInterbankBorrowingRequest { borrower_id, amount, .. } => {
                Validator::positive_amount(*amount)?;
                Validator::bank_exists(*borrower_id, state)
            }
            BankingAction::ExecuteInterbankLoan { lender_id, borrower_id, amount, .. } => {
                Validator::positive_amount(*amount)?;
                Validator::bank_exists(*lender_id, state)?;
                Validator::bank_exists(*borrower_id, state)
            }
            BankingAction::OriginateLoan { lender_id, borrower_id, loan_terms, .. } => {
                Validator::bank_exists(*lender_id, state)?;
                Validator::agent_exists(*borrower_id, state)?;
                Validator::positive_amount(loan_terms.principal)?;
                if loan_terms.term_months == 0 {
                    return Err("Loan term must be greater than zero".to_string());
                }
                Ok(())
            }
            BankingAction::CreateLoanApplication { bank_id, application } => {
                Validator::bank_exists(*bank_id, state)?;
                Validator::positive_amount(application.requested_amount)
            }
            BankingAction::ProcessLoanApplication { bank_id, application_id: _, .. } => {
                Validator::bank_exists(*bank_id, state)
            }
        }
    }
    pub fn resolve_loan_approval(&self, bank_id: AgentId, borrower_id: AgentId, terms: LoanTerms) -> Vec<SimAction> {
        let application_id = Uuid::new_v4();
        vec![SimAction::Banking(BankingAction::OriginateLoan {
            lender_id: bank_id,
            borrower_id,
            loan_terms: terms,
            application_id,
        })]
    }
    pub fn resolve_loan_request(
        &self, borrower_id: AgentId, bank_id: AgentId, amount: f64, purpose: LoanPurpose,
        collateral: Option<Vec<InstrumentId>>,
    ) -> Vec<SimAction> {
        let application = LoanApplication {
            application_id: Uuid::new_v4(),
            borrower_id,
            requested_amount: amount,
            purpose,
            proposed_collateral: collateral,
            borrower_income: None,
            debt_to_income_ratio: None,
            application_date: chrono::Utc::now().date_naive(),
            status: ApplicationStatus::Pending,
        };

        vec![SimAction::Banking(BankingAction::CreateLoanApplication { bank_id, application })]
    }
    pub fn resolve_loan_rejection(
        &self, bank_id: AgentId, _borrower_id: AgentId, application_id: Uuid, reason: String,
    ) -> Vec<SimAction> {
        vec![SimAction::Banking(BankingAction::ProcessLoanApplication {
            bank_id,
            application_id,
            decision: LoanDecision::Reject { reason },
        })]
    }
    fn execute_interbank_loan(
        &self, lender_id: AgentId, borrower_id: AgentId, amount: f64, rate_bps: BasisPoints, state: &SimState,
    ) -> DomainResult {
        let current_date = state.current_date;
        let maturity_date = current_date + chrono::Duration::days(1);

        let loan_instrument = match Instrument::bond(
            InstrumentId(Uuid::new_v4()),
            borrower_id,
            BondType::InterbankLoan,
            Money::from((amount as u64).max(1)),
            current_date,
            maturity_date,
        )
        .coupon_bps(rate_bps)
        .frequency(0)
        .rating(CreditRating::Corporate(SpCreditRating::A))
        .build()
        {
            Ok(inst) => inst,
            Err(e) => {
                return DomainResult::failure(vec![format!("Failed to create interbank loan: {}", e)]);
            }
        };

        let loan_id = loan_instrument.id;

        let mut effects = vec![StateEffect::Financial(FinancialEffect::CreateInstrument {
            instrument: loan_instrument,
            creditor: lender_id,
            debtor: borrower_id,
            quantity: 1.0,
        })];

        let payment_instruction = PaymentInstruction {
            id: Uuid::new_v4(),
            from_bank: lender_id,
            to_bank: borrower_id,
            payer: lender_id,
            payee: borrower_id,
            amount,
            context: TransactionContext::GenericTransfer { from: lender_id, to: borrower_id, amount },
            priority: PaymentPriority::Urgent,
            earliest_release_tick: state.ticknum,
            deadline_tick: state.ticknum + 1,
        };

        effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(payment_instruction)));

        effects.push(StateEffect::Financial(FinancialEffect::RecordTransaction(Transaction {
            id: Uuid::new_v4(),
            from_agent: lender_id,
            to_agent: borrower_id,
            amount,
            transaction_type: "InterbankLoan".to_string(),
            timestamp: current_date,
            instrument_id: Some(loan_id),
            ref_id: None,
        })));

        DomainResult::success(effects)
    }
    fn execute_create_loan_application(&self, bank_id: AgentId, application: LoanApplication) -> DomainResult {
        DomainResult::success(vec![StateEffect::Credit(CreditEffect::RecordLoanApplication { bank_id, application })])
    }

    fn execute_process_loan_application(
        &self, bank_id: AgentId, application_id: Uuid, decision: LoanDecision, state: &SimState,
    ) -> DomainResult {
        match decision {
            LoanDecision::Approve { terms } => {
                let borrower_id = state
                    .financial_system
                    .credit_registry
                    .applications
                    .get(&application_id)
                    .map(|a| a.borrower_id)
                    .unwrap_or_default();

                self.execute_loan_origination(bank_id, borrower_id, terms, application_id, state)
            }
            LoanDecision::Reject { reason: _ } => DomainResult::empty(),
            LoanDecision::CounterOffer { alternative_terms } => {
                println!("🔄 Loan application {} - counter offer: ${:.0}", application_id, alternative_terms.principal);
                DomainResult::empty()
            }
        }
    }

    // domains/src/banking_domain/domain.rs

    fn execute_loan_origination(
        &self, lender_id: AgentId, borrower_id: AgentId, loan_terms: LoanTerms, application_id: Uuid, state: &SimState,
    ) -> DomainResult {
        let issue_date = state.current_date;
        let maturity_date = issue_date + chrono::Duration::days((loan_terms.term_months * 30) as i64);

        let principal = Money::from_f64(loan_terms.principal).unwrap_or(Money::ZERO);
        let next_payment_date = match loan_terms.payment_frequency {
            PaymentFrequency::Monthly => issue_date + chrono::Duration::days(30),
            PaymentFrequency::Quarterly => issue_date + chrono::Duration::days(90),
            PaymentFrequency::SemiAnnual => issue_date + chrono::Duration::days(180),
            PaymentFrequency::Annual => issue_date + chrono::Duration::days(365),
            PaymentFrequency::InterestOnly => issue_date + chrono::Duration::days(30),
        };

        let amortization = match loan_terms.payment_frequency {
            PaymentFrequency::InterestOnly => Amortization::InterestOnly,
            _ => Amortization::Annuity,
        };

        // Determine if the borrower is a consumer from the agent registry
        let is_consumer = state.agents.consumers.contains_key(&borrower_id);

        let credit_rating = if is_consumer {
            Some(CreditRating::Consumer(ConsumerCreditRating::Subprime))
        } else {
            Some(CreditRating::Corporate(SpCreditRating::BBB))
        };

        let loan_details = LoanDetails {
            loan_id: Uuid::new_v4(),
            lender: lender_id,
            borrower: borrower_id,
            // borrower_type field is removed
            loan_type: LoanType::TermLoan,
            facility_id: None,

            principal,
            outstanding_principal: principal,
            reference_rate: Some(RateIndex::Fixed),
            spread_bps: loan_terms.annual_rate_bps,
            rate_floor_bps: None,
            rate_cap_bps: None,

            day_count: DayCount::ActAct,
            compounding: Compounding::Simple,
            payment_frequency: loan_terms.payment_frequency,

            origination_date: issue_date,
            maturity_date,
            next_payment_date,
            last_accrual_date: issue_date,

            amortization,
            prepayment_terms: PrepaymentTerms {
                allowed: true,
                penalty_type: PrepaymentPenalty::None,
                lockout_period_months: None,
            },

            collateral: vec![],
            covenants: vec![],
            credit_rating,
            impairment: ImpairmentState {
                stage: ImpairmentStage::Stage1Performing,
                provision_amount: Money::ZERO,
                days_past_due: 0,
                probability_of_default: 0.0,
                loss_given_default: 0.0,
                exposure_at_default: principal,
            },

            accrued_interest: Money::ZERO,
            unamortized_fees: Money::ZERO,
        };

        let loan = Loan {
            instrument_id: InstrumentId(Uuid::new_v4()),
            details: loan_details,
            status: LoanStatus::Current,
            servicing_history: vec![],
        };

        let deposit_instrument = Instrument::cash(
            InstrumentId(Uuid::new_v4()),
            lender_id, // bank issues the deposit liability
            CashType::DemandDeposit,
            Currency::USD,
            state.financial_system.central_bank.policy_rate_bps,
        )
        .build();

        let mut effects = vec![
            StateEffect::Credit(CreditEffect::RegisterLoan {
                loan: loan.clone(),
                is_consumer: is_consumer, // Pass the borrower type to the effect
                purpose: loan_terms.purpose.clone()
            }),
            StateEffect::Financial(FinancialEffect::CreateInstrument {
                instrument: deposit_instrument,
                creditor: borrower_id, // borrower has the deposit asset
                debtor: lender_id,     // bank has the deposit liability
                quantity: principal.to_f64(),
            }),
        ];

        effects.push(StateEffect::Financial(FinancialEffect::RecordTransaction(Transaction {
            id: Uuid::new_v4(),
            from_agent: lender_id,
            to_agent: borrower_id,
            amount: principal.to_f64(),
            transaction_type: "LoanOrigination".to_string(),
            timestamp: issue_date,
            instrument_id: Some(loan.instrument_id),
            ref_id: Some(application_id),
        })));

        DomainResult::success(effects)
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Banking",
        constructor: || Box::new(BankingDomain::new()),
    }
}
