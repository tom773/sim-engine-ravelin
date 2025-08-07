use serde::{Deserialize, Serialize};
use sim_core::*;
use sim_macros::SimDomain;

#[derive(Clone, Debug, Serialize, Deserialize, Default, SimDomain)]
pub struct SettlementDomain {}

#[derive(Debug, Clone)]
pub struct SettlementResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl SettlementDomain {
    pub fn new() -> Self {
        Self {}
    }

    pub fn can_handle(&self, action: &SettlementAction) -> bool {
        matches!(
            action,
            SettlementAction::AccrueInterest { .. }
                | SettlementAction::PayInterest { .. }
                | SettlementAction::ProcessCouponPayment { .. }
        )
    }

    pub fn validate(&self, action: &SettlementAction, state: &SimState) -> Result<(), String> {
        match action {
            SettlementAction::AccrueInterest { instrument_id } => self.validate_accrue_interest(instrument_id, state),
            SettlementAction::PayInterest { instrument_id } => self.validate_pay_interest(instrument_id, state),
            SettlementAction::ProcessCouponPayment { instrument_id } => {
                self.validate_process_coupon_payment(instrument_id, state)
            }
        }
    }

    fn validate_accrue_interest(&self, instrument_id: &InstrumentId, state: &SimState) -> Result<(), String> {
        if !state.financial_system.instruments.contains_key(instrument_id) {
            return Err(format!("Instrument {:?} not found for accrual.", instrument_id));
        }
        Ok(())
    }

    fn validate_pay_interest(&self, instrument_id: &InstrumentId, state: &SimState) -> Result<(), String> {
        let instrument = state
            .financial_system
            .instruments
            .get(instrument_id)
            .ok_or(format!("Instrument {:?} not found for interest payment.", instrument_id))?;

        let interest_to_pay = instrument.accrued_interest;
        if interest_to_pay <= 1e-6 {
            return Ok(());
        }

        let available_funds = state.financial_system.get_liquid_assets(&instrument.debtor);
        if available_funds < interest_to_pay {
            return Err(format!(
                "Insufficient funds for interest payment: agent {:?} needs ${:.2}, has ${:.2}",
                instrument.debtor, interest_to_pay, available_funds
            ));
        }
        Ok(())
    }

    fn get_coupon_payment_amount(&self, instrument: &FinancialInstrument) -> Option<f64> {
        if let Some(bond) = instrument.details.as_any().downcast_ref::<BondDetails>() {
            let payment = (instrument.principal * bond.coupon_rate) / bond.frequency as f64;
            Some(payment)
        } else {
            None
        }
    }

    fn validate_process_coupon_payment(&self, instrument_id: &InstrumentId, state: &SimState) -> Result<(), String> {
        let instrument = state
            .financial_system
            .instruments
            .get(instrument_id)
            .ok_or(format!("Instrument {:?} not found for coupon payment.", instrument_id))?;

        let payment_amount = self
            .get_coupon_payment_amount(instrument)
            .ok_or(format!("Instrument {:?} is not a bond, no coupon payment.", instrument_id))?;

        let available_funds = state.financial_system.get_liquid_assets(&instrument.debtor);
        if available_funds < payment_amount {
            return Err(format!(
                "Insufficient funds for coupon payment: agent {:?} needs ${:.2}, has ${:.2}",
                instrument.debtor, payment_amount, available_funds
            ));
        }
        Ok(())
    }

    fn create_payment_effects(&self, from: AgentId, to: AgentId, amount: f64, state: &SimState) -> Vec<StateEffect> {
        let mut effects = vec![];
        let cb_id = state.financial_system.central_bank.id;
        if let Some(from_bs) = state.financial_system.get_bs_by_id(&from) {
            if let Some((cash_inst_id, cash_inst)) =
                from_bs.assets.iter().find(|(_, inst)| inst.details.as_any().is::<CashDetails>())
            {
                let new_principal = cash_inst.principal - amount;
                if new_principal < 1e-6 {
                    effects.push(StateEffect::Financial(FinancialEffect::RemoveInstrument(*cash_inst_id)));
                } else {
                    effects.push(StateEffect::Financial(FinancialEffect::UpdateInstrument {
                        id: *cash_inst_id,
                        new_principal,
                    }));
                }
                let new_cash_for_to = cash!(to, amount, cb_id, state.current_date);
                effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(new_cash_for_to)));
            }
        }
        effects
    }

    pub fn execute(&self, action: &SettlementAction, state: &SimState) -> SettlementResult {
        if let Err(e) = self.validate(action, state) {
            return SettlementResult { success: false, effects: vec![], errors: vec![e] };
        }

        match action {
            SettlementAction::AccrueInterest { instrument_id } => self.execute_accrue_interest(instrument_id, state),
            SettlementAction::PayInterest { instrument_id } => self.execute_pay_interest(instrument_id, state),
            SettlementAction::ProcessCouponPayment { instrument_id } => {
                self.execute_process_coupon_payment(instrument_id, state)
            }
        }
    }

    fn calculate_daily_interest_accrual(
        &self, instrument: &FinancialInstrument, current_date: chrono::NaiveDate,
    ) -> f64 {
        let days_since_last_accrual = (current_date - instrument.last_accrual_date).num_days();
        if days_since_last_accrual <= 0 {
            return 0.0;
        }

        let annual_rate = if let Some(deposit) = instrument.details.as_any().downcast_ref::<DemandDepositDetails>() {
            deposit.interest_rate
        } else if let Some(bond) = instrument.details.as_any().downcast_ref::<BondDetails>() {
            bond.coupon_rate
        } else {
            return 0.0;
        };

        let daily_rate = annual_rate / 365.0;
        instrument.principal * daily_rate * days_since_last_accrual as f64
    }

    fn execute_accrue_interest(&self, instrument_id: &InstrumentId, state: &SimState) -> SettlementResult {
        if let Some(instrument) = state.financial_system.instruments.get(instrument_id) {
            let accrued_amount = self.calculate_daily_interest_accrual(instrument, state.current_date);
            if accrued_amount > 1e-6 {
                let effect = StateEffect::Financial(FinancialEffect::AccrueInterest {
                    instrument_id: *instrument_id,
                    accrued_amount,
                    accrual_date: state.current_date,
                });
                SettlementResult { success: true, effects: vec![effect], errors: vec![] }
            } else {
                SettlementResult { success: true, effects: vec![], errors: vec![] }
            }
        } else {
            SettlementResult { success: false, effects: vec![], errors: vec!["Instrument not found".to_string()] }
        }
    }

    fn execute_pay_interest(&self, instrument_id: &InstrumentId, state: &SimState) -> SettlementResult {
        if let Some(instrument) = state.financial_system.instruments.get(instrument_id) {
            let interest_amount = instrument.accrued_interest;
            if interest_amount <= 1e-6 {
                return SettlementResult { success: true, effects: vec![], errors: vec![] };
            }
            let mut effects =
                self.create_payment_effects(instrument.debtor, instrument.creditor, interest_amount, state);
            effects
                .push(StateEffect::Financial(FinancialEffect::ResetAccruedInterest { instrument_id: *instrument_id }));
            SettlementResult { success: true, effects, errors: vec![] }
        } else {
            SettlementResult { success: false, effects: vec![], errors: vec!["Instrument not found".to_string()] }
        }
    }

    fn execute_process_coupon_payment(&self, instrument_id: &InstrumentId, state: &SimState) -> SettlementResult {
        if let Some(instrument) = state.financial_system.instruments.get(instrument_id) {
            if let Some(payment_amount) = self.get_coupon_payment_amount(instrument) {
                if payment_amount <= 1e-6 {
                    return SettlementResult { success: true, effects: vec![], errors: vec![] };
                }
                let effects =
                    self.create_payment_effects(instrument.debtor, instrument.creditor, payment_amount, state);
                SettlementResult { success: true, effects, errors: vec![] }
            } else {
                SettlementResult {
                    success: false,
                    effects: vec![],
                    errors: vec!["Instrument is not a bond".to_string()],
                }
            }
        } else {
            SettlementResult { success: false, effects: vec![], errors: vec!["Instrument not found".to_string()] }
        }
    }
}
