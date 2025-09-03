use crate::{
    Any, Domain, DomainResult, DomainValidator, ResolutionContext, ResolutionPhase,
    ResolutionResult,
};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use sim_core::*;
use uuid::Uuid;
extern crate inventory;
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

    fn resolve_intention(
        &self,
        intention: &SimIntention,
        context: &ResolutionContext,
    ) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::DepositFunds {
                agent_id,
                bank,
                amount,
            } => {
                vec![SimAction::Banking(BankingAction::Deposit {
                    agent_id: *agent_id,
                    bank: *bank,
                    amount: *amount,
                })]
            }

            SimIntention::WithdrawFunds {
                agent_id,
                bank,
                amount,
            } => {
                vec![SimAction::Banking(BankingAction::Withdraw {
                    agent_id: *agent_id,
                    bank: *bank,
                    amount: *amount,
                })]
            }

            SimIntention::PayWages {
                employer,
                employee,
                amount,
            } => {
                vec![SimAction::Banking(BankingAction::InitiatePayment {
                    from: *employer,
                    to: *employee,
                    amount: *amount,
                    context: TransactionContext::WagePayment,
                })]
            }

            SimIntention::CollectTaxes {
                government_id,
                target,
                amount,
            } => {
                vec![SimAction::Banking(BankingAction::InitiatePayment {
                    from: *target,
                    to: *government_id,
                    amount: *amount,
                    context: TransactionContext::TaxPayment,
                })]
            }

            SimIntention::InjectLiquidity => {
                vec![SimAction::Banking(BankingAction::InjectLiquidity)]
            }

            SimIntention::LendExcessReserves {
                agent_id,
                amount,
                target_rate_bps,
            } => self.resolve_reserve_lending(*agent_id, *amount, *target_rate_bps, context.state),

            SimIntention::BorrowReserves {
                agent_id,
                amount,
                target_rate_bps,
            } => {
                self.resolve_reserve_borrowing(*agent_id, *amount, *target_rate_bps, context.state)
            }

            _ => return None,
        };

        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::DepositFunds { .. }
            | SimIntention::WithdrawFunds { .. }
            | SimIntention::PayWages { .. }
            | SimIntention::CollectTaxes { .. }
            | SimIntention::InjectLiquidity => Some(ResolutionPhase::Independent),

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
            BankingAction::Deposit { .. } => self.execute_deposit(),
            BankingAction::Withdraw { .. } => self.execute_withdraw(),
            BankingAction::InitiatePayment {
                from,
                to,
                amount,
                context,
            } => self.execute_initiate_payment(*from, *to, *amount, context.clone(), state),
            BankingAction::InjectLiquidity => self.execute_inject_liquidity(state),

            BankingAction::ExecuteInterbankLoan {
                lender_id,
                borrower_id,
                amount,
                rate_bps,
            } => self.execute_interbank_loan(*lender_id, *borrower_id, *amount, *rate_bps, state),
            BankingAction::PostInterbankLendingOffer { .. }
            | BankingAction::PostInterbankBorrowingRequest { .. } => DomainResult::success(vec![]),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl BankingDomain {
    fn find_reserves_instrument_id(&self, state: &SimState) -> Option<InstrumentId> {
        let fs = &state.financial_system;
        let cb_id = fs.central_bank.id;
        fs.instruments.iter().find_map(|(id, inst)| {
            if let InstrumentType::Cash(details) = &inst.instrument_type {
                if details.cash_type == CashType::CentralBankReserves && details.issuer == cb_id {
                    return Some(*id);
                }
            }
            None
        })
    }

    fn resolve_reserve_lending(
        &self,
        agent_id: AgentId,
        amount: f64,
        target_rate_bps: BasisPoints,
        state: &SimState,
    ) -> Vec<SimAction> {
        if let Some(_) = self.find_reserves_instrument_id(state) {
            vec![SimAction::Banking(
                BankingAction::PostInterbankLendingOffer {
                    lender_id: agent_id,
                    amount,
                    rate_bps: target_rate_bps,
                },
            )]
        } else {
            vec![]
        }
    }

    fn resolve_reserve_borrowing(
        &self,
        agent_id: AgentId,
        amount: f64,
        target_rate_bps: BasisPoints,
        state: &SimState,
    ) -> Vec<SimAction> {
        if let Some(_) = self.find_reserves_instrument_id(state) {
            vec![SimAction::Banking(
                BankingAction::PostInterbankBorrowingRequest {
                    borrower_id: agent_id,
                    amount,
                    rate_bps: target_rate_bps,
                },
            )]
        } else {
            vec![]
        }
    }
}

impl BankingDomain {
    fn find_instrument_position(
        fs: &FinancialSystem,
        agent_id: &AgentId,
        cash_type: CashType,
        issuer: AgentId,
    ) -> Option<(InstrumentId, f64)> {
        fs.balance_sheets
            .get(agent_id)?
            .assets
            .iter()
            .find_map(|(id, pos)| {
                fs.instruments.get(id).and_then(|inst| {
                    if let InstrumentType::Cash(c) = &inst.instrument_type {
                        if c.cash_type == cash_type && c.issuer == issuer {
                            return Some((*id, pos.quantity));
                        }
                    }
                    None
                })
            })
    }

    fn validate(&self, action: &BankingAction, state: &SimState) -> Result<(), String> {
        let fs = &state.financial_system;
        match action {
            BankingAction::Deposit {
                agent_id,
                bank,
                amount,
            } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::agent_exists(*agent_id, state)?;
                DomainValidator::bank_exists(*bank, state)?;

                let cb_id = fs.central_bank.id;
                let currency_pos =
                    Self::find_instrument_position(fs, agent_id, CashType::Currency, cb_id);

                if let Some((_id, available_cash)) = currency_pos {
                    if available_cash < *amount {
                        return Err(format!(
                            "Insufficient physical cash for deposit: agent has ${:.2}, needs ${:.2}",
                            available_cash, amount
                        ));
                    }
                } else {
                    return Err("Agent possesses no physical currency to deposit.".to_string());
                }
                Ok(())
            }
            BankingAction::Withdraw {
                agent_id,
                bank,
                amount,
            } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::agent_exists(*agent_id, state)?;
                DomainValidator::bank_exists(*bank, state)?;

                let deposit_pos =
                    Self::find_instrument_position(fs, agent_id, CashType::DemandDeposit, *bank);

                if let Some((_id, available_deposits)) = deposit_pos {
                    if available_deposits < *amount {
                        return Err(format!(
                            "Insufficient deposits for withdrawal at bank {}: agent has ${:.2}, needs ${:.2}",
                            bank, available_deposits, amount
                        ));
                    }
                } else {
                    return Err(format!(
                        "Agent has no deposit account at bank {} to withdraw from.",
                        bank
                    )
                    .to_string());
                }

                Ok(())
            }
            BankingAction::InitiatePayment {
                from, to, amount, ..
            } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::agent_exists(*from, state)?;
                DomainValidator::agent_exists(*to, state)?;
                Ok(())
            }
            BankingAction::InjectLiquidity => Ok(()),

            BankingAction::PostInterbankLendingOffer {
                lender_id, amount, ..
            } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::bank_exists(*lender_id, state)
            }
            BankingAction::PostInterbankBorrowingRequest {
                borrower_id,
                amount,
                ..
            } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::bank_exists(*borrower_id, state)
            }
            BankingAction::ExecuteInterbankLoan {
                lender_id,
                borrower_id,
                amount,
                ..
            } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::bank_exists(*lender_id, state)?;
                DomainValidator::bank_exists(*borrower_id, state)
            }
        }
    }
}

impl BankingDomain {
    fn execute_deposit(&self) -> DomainResult {
        DomainResult::failure(vec!["Deposit action cannot be executed. Missing core support for partial instrument transfers.".to_string()])
    }

    fn execute_withdraw(&self) -> DomainResult {
        DomainResult::failure(vec!["Withdraw action cannot be executed. Missing core support for partial instrument transfers.".to_string()])
    }

    pub fn execute_initiate_payment(
        &self,
        from: AgentId,
        to: AgentId,
        amount: f64,
        context: TransactionContext,
        state: &SimState,
    ) -> DomainResult {
        if amount <= 1e-9 {
            return DomainResult::empty();
        }

        let (from_account_id, from_settlement_agent_id) =
            match state.financial_system.find_agent_liquid_account(&from) {
                Some(acc) => acc,
                None => {
                    return DomainResult::failure(vec![format!(
                        "Sender {} has no liquid account.",
                        from
                    )]);
                }
            };

        let (to_account_id, to_settlement_agent_id) =
            match state.financial_system.find_agent_liquid_account(&to) {
                Some(acc) => acc,
                None => state
                    .financial_system
                    .find_any_bank_account()
                    .ok_or_else(|| "No banks in simulation to open account with.".to_string())
                    .unwrap(),
            };

        let from_bs = state.financial_system.balance_sheets.get(&from).unwrap();
        if let Some(pos) = from_bs.assets.get(&from_account_id) {
            if pos.quantity < amount {
                return DomainResult::failure(vec![format!(
                    "Insufficient funds for agent {}: has {:.2}, needs {:.2}",
                    from, pos.quantity, amount
                )]);
            }
        } else {
            return DomainResult::failure(vec![format!(
                "Sender {} position not found for account {}",
                from, from_account_id
            )]);
        }

        let mut effects = Vec::new();

        if from_settlement_agent_id == to_settlement_agent_id {
            let bank_id = from_settlement_agent_id;

            effects.push(StateEffect::Financial(FinancialEffect::AdjustPosition {
                owner: from,
                instrument_id: from_account_id,
                delta_quantity: -amount,
                side: PositionSide::Asset,
                cost_per_unit: None,
            }));
            effects.push(StateEffect::Financial(FinancialEffect::AdjustPosition {
                owner: to,
                instrument_id: to_account_id,
                delta_quantity: amount,
                side: PositionSide::Asset,
                cost_per_unit: None,
            }));
            effects.push(StateEffect::Financial(FinancialEffect::AdjustPosition {
                owner: bank_id,
                instrument_id: from_account_id,
                delta_quantity: -amount,
                side: PositionSide::Liability,
                cost_per_unit: None,
            }));
            effects.push(StateEffect::Financial(FinancialEffect::AdjustPosition {
                owner: bank_id,
                instrument_id: to_account_id,
                delta_quantity: amount,
                side: PositionSide::Liability,
                cost_per_unit: None,
            }));
        } else {
            let from_bank_reserves_id = match state
                .financial_system
                .find_bank_reserves_account(&from_settlement_agent_id)
            {
                Some(id) => id,
                None => {
                    return DomainResult::failure(vec![format!(
                        "Sending bank {} has no reserves.",
                        from_settlement_agent_id
                    )]);
                }
            };
            let to_bank_reserves_id = match state
                .financial_system
                .find_bank_reserves_account(&to_settlement_agent_id)
            {
                Some(id) => id,
                None => {
                    return DomainResult::failure(vec![format!(
                        "Receiving bank {} has no reserves.",
                        to_settlement_agent_id
                    )]);
                }
            };

            effects.push(StateEffect::Financial(FinancialEffect::AdjustPosition {
                owner: from,
                instrument_id: from_account_id,
                delta_quantity: -amount,
                side: PositionSide::Asset,
                cost_per_unit: None,
            }));
            effects.push(StateEffect::Financial(FinancialEffect::AdjustPosition {
                owner: from_settlement_agent_id,
                instrument_id: from_account_id,
                delta_quantity: -amount,
                side: PositionSide::Liability,
                cost_per_unit: None,
            }));

            effects.push(StateEffect::Financial(FinancialEffect::AdjustPosition {
                owner: from_settlement_agent_id,
                instrument_id: from_bank_reserves_id,
                delta_quantity: -amount,
                side: PositionSide::Asset,
                cost_per_unit: None,
            }));
            effects.push(StateEffect::Financial(FinancialEffect::AdjustPosition {
                owner: to_settlement_agent_id,
                instrument_id: to_bank_reserves_id,
                delta_quantity: amount,
                side: PositionSide::Asset,
                cost_per_unit: None,
            }));

            effects.push(StateEffect::Financial(FinancialEffect::AdjustPosition {
                owner: to,
                instrument_id: to_account_id,
                delta_quantity: amount,
                side: PositionSide::Asset,
                cost_per_unit: None,
            }));
            effects.push(StateEffect::Financial(FinancialEffect::AdjustPosition {
                owner: to_settlement_agent_id,
                instrument_id: to_account_id,
                delta_quantity: amount,
                side: PositionSide::Liability,
                cost_per_unit: None,
            }));
        }

        effects.push(StateEffect::Financial(FinancialEffect::RecordTransaction(
            Transaction {
                id: Uuid::new_v4(),
                from_agent: from,
                to_agent: to,
                amount,
                transaction_type: format!("{:?}", context),
                timestamp: state.current_date,
                instrument_id: None,
                ref_id: None,
            },
        )));

        DomainResult::success(effects)
    }

    fn execute_inject_liquidity(&self, state: &SimState) -> DomainResult {
        let recipients: Vec<AgentId> = state.agents.consumers.keys().cloned().collect();
        let amount_per_recipient = 1000.0;
        let cb_id = state.financial_system.central_bank.id;

        let mut all_effects = Vec::new();
        for recipient in recipients {
            let result = self.execute_initiate_payment(
                cb_id,
                recipient,
                amount_per_recipient,
                TransactionContext::GovTranseferPayment,
                state,
            );
            
            if result.success {
                all_effects.extend(result.effects);
            } else {
                println!("[BankingDomain] Failed to inject liquidity to {}: {:?}", recipient.0, result.errors);
            }
        }

        DomainResult::success(all_effects)
    }

    fn execute_interbank_loan(
        &self,
        lender_id: AgentId,
        borrower_id: AgentId,
        amount: f64,
        rate_bps: BasisPoints,
        state: &SimState,
    ) -> DomainResult {
        let current_date = state.current_date;
        let maturity_date = current_date + Duration::days(1);

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
        .rating(CreditRating::A)
        .build()
        {
            Ok(inst) => inst,
            Err(e) => {
                return DomainResult::failure(vec![format!(
                    "Failed to create interbank loan: {}",
                    e
                )]);
            }
        };

        let loan_id = loan_instrument.id;

        let mut effects = vec![
            StateEffect::Financial(FinancialEffect::CreateInstrument {
                instrument: loan_instrument,
                creditor: lender_id,
                debtor: borrower_id,
                quantity: 1.0,
            }),
        ];

        let payment_result = self.execute_initiate_payment(
            lender_id, 
            borrower_id, 
            amount, 
            TransactionContext::GenericTransfer,
            state
        );

        if payment_result.success {
            effects.extend(payment_result.effects);
        } else {
            return DomainResult::failure(vec![format!("Failed to transfer interbank loan principal: {:?}", payment_result.errors)]);
        }

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