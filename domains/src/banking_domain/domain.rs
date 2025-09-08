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

    fn resolve_intention(&self, intention: &SimIntention, context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::Banking(BankingIntention::LendExcessReserves { agent_id, amount, target_rate_bps }) => {
                self.resolve_reserve_lending(*agent_id, *amount, *target_rate_bps, context.state)
            }
            SimIntention::Banking(BankingIntention::BorrowReserves { agent_id, amount, target_rate_bps }) => {
                self.resolve_reserve_borrowing(*agent_id, *amount, *target_rate_bps, context.state)
            }
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
            SimIntention::Banking(BankingIntention::LendExcessReserves { .. })
            | SimIntention::Banking(BankingIntention::BorrowReserves { .. }) => Some(ResolutionPhase::Market),
            | SimIntention::Banking(BankingIntention::RequestLoan { .. })
            | SimIntention::Banking(BankingIntention::ApproveLoan { .. })
            | SimIntention::Banking(BankingIntention::RejectLoan { .. }) => Some(ResolutionPhase::Independent),
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
            BankingAction::PostInterbankLendingOffer { .. } | BankingAction::PostInterbankBorrowingRequest { .. } => {
                DomainResult::success(vec![])
            }
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
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl BankingDomain {
    fn resolve_reserve_lending(
        &self, agent_id: AgentId, amount: f64, target_rate_bps: BasisPoints, _state: &SimState,
    ) -> Vec<SimAction> {
        vec![SimAction::Banking(BankingAction::PostInterbankLendingOffer {
            lender_id: agent_id,
            amount,
            rate_bps: target_rate_bps,
        })]
    }

    fn resolve_reserve_borrowing(
        &self, agent_id: AgentId, amount: f64, target_rate_bps: BasisPoints, _state: &SimState,
    ) -> Vec<SimAction> {
        vec![SimAction::Banking(BankingAction::PostInterbankBorrowingRequest {
            borrower_id: agent_id,
            amount,
            rate_bps: target_rate_bps,
        })]
    }

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
    pub fn resolve_loan_approval(
        &self, bank_id: AgentId, borrower_id: AgentId, terms: LoanTerms 
    ) -> Vec<SimAction> {
        let application_id = Uuid::new_v4(); // In real case, would track the actual application ID
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
            borrower_income: None, // Will be filled in by bank during underwriting
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
        let maturity_date = current_date + chrono::Duration::days(1); // Overnight loan

        let loan_instrument = match Instrument::bond(
            InstrumentId(Uuid::new_v4()),
            borrower_id,
            BondType::InterbankLoan,
            Money::from((amount as u64).max(1)),
            current_date,
            maturity_date,
        )
        .coupon_bps(rate_bps)
        .frequency(0) // Zero coupon for overnight
        .rating(CreditRating::A)
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
            priority: PaymentPriority::Urgent, // Interbank is high priority
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
    fn execute_create_loan_application(&self, _bank_id: AgentId, application: LoanApplication) -> DomainResult {
        println!("📋 Loan application received: ${:.0} for {:?}", application.requested_amount, application.purpose);
        DomainResult::empty()
    }

    fn execute_process_loan_application(
        &self, bank_id: AgentId, application_id: Uuid, decision: LoanDecision, state: &SimState,
    ) -> DomainResult {
        match decision {
            LoanDecision::Approve { terms } => {
                let borrower_id = AgentId::default(); // Would look up from application

                self.execute_loan_origination(bank_id, borrower_id, terms, application_id, state)
            }
            LoanDecision::Reject { reason } => {
                println!("❌ Loan application {} rejected: {}", application_id, reason);
                DomainResult::empty()
            }
            LoanDecision::CounterOffer { alternative_terms } => {
                println!("🔄 Loan application {} - counter offer: ${:.0}", application_id, alternative_terms.principal);
                DomainResult::empty()
            }
        }
    }

    fn execute_loan_origination(
        &self, lender_id: AgentId, borrower_id: AgentId, loan_terms: LoanTerms, _application_id: Uuid, state: &SimState,
    ) -> DomainResult {
        let issue_date = state.current_date;
        let maturity_date = issue_date + chrono::Duration::days((loan_terms.term_months * 30) as i64);

        let loan_instrument = match Instrument::bond(
            InstrumentId(Uuid::new_v4()),
            borrower_id,         // Borrower is the issuer of the debt
            BondType::Corporate, // Or Personal for consumer loans
            Money::from_f64(loan_terms.principal).unwrap_or(Money::ZERO),
            issue_date,
            maturity_date,
        )
        .coupon_bps(loan_terms.annual_rate_bps)
        .frequency(match loan_terms.payment_frequency {
            PaymentFrequency::Monthly => 12,
            PaymentFrequency::Quarterly => 4,
            PaymentFrequency::SemiAnnual => 2,
            PaymentFrequency::Annual => 1,
            PaymentFrequency::InterestOnly => 0,
        })
        .rating(CreditRating::BBB) // Would be determined by underwriting
        .build()
        {
            Ok(instrument) => instrument,
            Err(e) => return DomainResult::failure(vec![format!("Failed to build loan: {}", e)]),
        };

        let deposit_instrument = Instrument::cash(
            InstrumentId(Uuid::new_v4()),
            lender_id, // Bank is the issuer of the deposit liability
            CashType::DemandDeposit,
            Currency::USD,
            state.financial_system.central_bank.policy_rate_bps,
        )
        .build();

        let effects = vec![
            StateEffect::Financial(FinancialEffect::CreateInstrument {
                instrument: loan_instrument,
                creditor: lender_id, // Bank owns the loan asset
                debtor: borrower_id, // Borrower has the liability
                quantity: 1.0,       // One loan contract
            }),
            StateEffect::Financial(FinancialEffect::CreateInstrument {
                instrument: deposit_instrument,
                creditor: borrower_id, // Borrower owns the deposit asset
                debtor: lender_id,     // Bank has the deposit liability
                quantity: loan_terms.principal,
            }),
        ];

        println!(
            "💰 CREDIT CREATED: Bank {} originated ${:.0} loan to {}",
            lender_id, loan_terms.principal, borrower_id
        );

        DomainResult::success(effects)
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Banking",
        constructor: || Box::new(BankingDomain::new()),
    }
}
