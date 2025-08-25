use crate::{Any, Domain, DomainResult, inventory};
use serde::{Deserialize, Serialize};
use sim_core::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettlementDomain {}

impl SettlementDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for SettlementDomain {
    fn name(&self) -> &'static str {
        "Settlement"
    }


    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let settlement_action = match action {
            SimAction::Settlement(action) => action,
            _ => return DomainResult::failure(vec!["Not a settlement action".to_string()]),
        };

        if let Err(error) = self.validate(settlement_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match settlement_action {
            SettlementAction::AccrueInterest { instrument_id } => self.execute_accrue_interest(instrument_id, state),
            SettlementAction::PayInterest { instrument_id } => self.execute_pay_interest(instrument_id, state),
            SettlementAction::ProcessCouponPayment { instrument_id } => {
                self.execute_process_coupon_payment(instrument_id, state)
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl SettlementDomain {
    fn validate(&self, action: &SettlementAction, state: &SimState) -> Result<(), String> {
        match action {
            SettlementAction::AccrueInterest { instrument_id }
            | SettlementAction::PayInterest { instrument_id }
            | SettlementAction::ProcessCouponPayment { instrument_id } => {
                if !state.financial_system.instruments.contains_key(instrument_id) {
                    Err(format!("Instrument {} not found", instrument_id.0))
                } else {
                    Ok(())
                }
            }
        }
    }

    fn execute_accrue_interest(&self, instrument_id: &InstrumentId, state: &SimState) -> DomainResult {
        if let Some(instrument) = state.financial_system.instruments.get(instrument_id) {
            let daily_accrual = self.calculate_daily_interest_accrual(instrument, state.current_date);

            if daily_accrual > 1e-6 {
                let effect = StateEffect::Financial(FinancialEffect::AccrueInterest {
                    instrument_id: *instrument_id,
                    accrued_amount: daily_accrual,
                    accrual_date: state.current_date,
                });
                DomainResult::success(vec![effect])
            } else {
                DomainResult::empty()
            }
        } else {
            DomainResult::failure(vec!["Instrument not found".to_string()])
        }
    }

    fn execute_pay_interest(&self, instrument_id: &InstrumentId, state: &SimState) -> DomainResult {
        if let Some(instrument) = state.financial_system.instruments.get(instrument_id) {
            let interest_amount = instrument.accrued_interest;

            if interest_amount <= 1e-6 {
                return DomainResult::empty();
            }

            let payment_effects =
                self.create_payment_effects(instrument.debtor, instrument.creditor, interest_amount, state);

            let mut effects = payment_effects;
            effects
                .push(StateEffect::Financial(FinancialEffect::ResetAccruedInterest { instrument_id: *instrument_id }));

            DomainResult::success(effects)
        } else {
            DomainResult::failure(vec!["Instrument not found".to_string()])
        }
    }

    fn execute_process_coupon_payment(&self, instrument_id: &InstrumentId, state: &SimState) -> DomainResult {
        if let Some(instrument) = state.financial_system.instruments.get(instrument_id) {
            if let Some(payment_amount) = self.get_coupon_payment_amount(instrument) {
                if payment_amount <= 1e-6 {
                    return DomainResult::empty();
                }

                let effects =
                    self.create_payment_effects(instrument.debtor, instrument.creditor, payment_amount, state);

                DomainResult::success(effects)
            } else {
                DomainResult::failure(vec!["Instrument is not a bond".to_string()])
            }
        } else {
            DomainResult::failure(vec!["Instrument not found".to_string()])
        }
    }

    fn calculate_daily_interest_accrual(
        &self, instrument: &FinancialInstrument, current_date: chrono::NaiveDate,
    ) -> f64 {
        if current_date <= instrument.last_accrual_date {
            return 0.0;
        }

        let (annual_rate_bps, day_count) =
            if let Some(deposit) = instrument.details.as_any().downcast_ref::<DemandDepositDetails>() {
                (deposit.interest_rate_bps, deposit.day_count)
            } else if let Some(deposit) = instrument.details.as_any().downcast_ref::<SavingsDepositDetails>() {
                (deposit.interest_rate_bps, deposit.day_count)
            } else if let Some(bond) = instrument.details.as_any().downcast_ref::<BondDetails>() {
                (bond.coupon_rate_bps, bond.day_count)
            } else {
                return 0.0;
            };

        day_count.calculate_accrued_interest(
            instrument.principal,
            annual_rate_bps,
            instrument.last_accrual_date,
            current_date,
        )
    }

    fn get_coupon_payment_amount(&self, instrument: &FinancialInstrument) -> Option<f64> {
        if let Some(bond) = instrument.details.as_any().downcast_ref::<BondDetails>() {
            Some(instrument.principal * bps_to_decimal(bond.coupon_rate_bps) / 2.0)
        } else {
            None
        }
    }

    fn create_payment_effects(&self, from: AgentId, to: AgentId, amount: f64, state: &SimState) -> Vec<StateEffect> {
        vec![
            StateEffect::Financial(FinancialEffect::TransferFunds { from, to, amount }),
            StateEffect::Financial(FinancialEffect::RecordTransaction(Transaction {
                id: uuid::Uuid::new_v4(),
                date: state.ticknum,
                qty: amount,
                from,
                to,
                tx_type: TransactionType::InterestPayment { payer: from, receiver: to, amount: amount },
                instrument_id: None,
            })),
        ]
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Settlement",
        constructor: || Box::new(SettlementDomain::new()),
    }
}
