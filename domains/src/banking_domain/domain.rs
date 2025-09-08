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
            SimIntention::LendExcessReserves { agent_id, amount, target_rate_bps } => {
                self.resolve_reserve_lending(*agent_id, *amount, *target_rate_bps, context.state)
            }
            SimIntention::BorrowReserves { agent_id, amount, target_rate_bps } => {
                self.resolve_reserve_borrowing(*agent_id, *amount, *target_rate_bps, context.state)
            }
            _ => return None,
        };

        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::LendExcessReserves { .. } | SimIntention::BorrowReserves { .. } => {
                Some(ResolutionPhase::Market)
            }
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
            BankingAction::PostInterbankLendingOffer { .. } | 
            BankingAction::PostInterbankBorrowingRequest { .. } => {
                DomainResult::success(vec![])
            }
            BankingAction::ExecuteInterbankLoan { lender_id, borrower_id, amount, rate_bps } => {
                self.execute_interbank_loan(*lender_id, *borrower_id, *amount, *rate_bps, state)
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
        }
    }

    fn execute_interbank_loan(
        &self, lender_id: AgentId, borrower_id: AgentId, amount: f64, rate_bps: BasisPoints, state: &SimState,
    ) -> DomainResult {
        let current_date = state.current_date;
        let maturity_date = current_date + chrono::Duration::days(1); // Overnight loan

        // Create the interbank loan bond
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

        // Create the bond - this will be routed to CSD automatically
        let mut effects = vec![StateEffect::Financial(FinancialEffect::CreateInstrument {
            instrument: loan_instrument,
            creditor: lender_id,
            debtor: borrower_id,
            quantity: 1.0,
        })];

        // Queue the payment through RTGS
        let payment_instruction = PaymentInstruction {
            id: Uuid::new_v4(),
            from_bank: lender_id,
            to_bank: borrower_id,
            payer: lender_id,
            payee: borrower_id,
            amount,
            context: TransactionContext::GenericTransfer {
                from: lender_id,
                to: borrower_id,
                amount,
            },
            priority: PaymentPriority::Urgent, // Interbank is high priority
            earliest_release_tick: state.ticknum,
            deadline_tick: state.ticknum + 1,
        };

        effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(payment_instruction)));

        // Record the transaction
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
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Banking",
        constructor: || Box::new(BankingDomain::new()),
    }
}

