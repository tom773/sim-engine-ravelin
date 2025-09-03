use crate::{Any, Domain, DomainResult, inventory};
use serde::{Deserialize, Serialize};
use sim_core::*;
use rust_decimal::prelude::*;

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
        let current_date = state.current_date;
        
        if let Some(instrument) = state.financial_system.instruments.get(instrument_id) {
            match &instrument.instrument_type {
                InstrumentType::Cash(details) => {
                    let parties = state.financial_system.get_parties(instrument_id);
                    
                    if let Some((creditor, debtor)) = parties {
                        let principal = state.financial_system.balance_sheets
                            .get(&creditor)
                            .and_then(|bs| bs.assets.get(instrument_id))
                            .map(|pos| pos.quantity)
                            .unwrap_or(0.0);

                        if principal > 0.0 {
                            let principal_money = Money::from_f64(principal).unwrap_or_default();
                            let day_count = DayCount::Act365F;
                            let interest_amount = day_count.calculate_accrued_interest(
                                principal_money,
                                details.interest_bps,
                                current_date - chrono::Duration::days(1),
                                current_date
                            );

                            let proportional_adj = (interest_amount / principal_money).to_f64().unwrap_or_default();

                            let effects = vec![
                                StateEffect::Financial(FinancialEffect::AdjustPosition {
                                    owner: creditor,
                                    instrument_id: *instrument_id,
                                    delta_quantity: proportional_adj,
                                    side: PositionSide::Asset,
                                    cost_per_unit: None,
                                }),
                                StateEffect::Financial(FinancialEffect::AdjustPosition {
                                    owner: debtor,
                                    instrument_id: *instrument_id,
                                    delta_quantity: proportional_adj,
                                    side: PositionSide::Liability,
                                    cost_per_unit: None,
                                }),
                            ];
                            
                            return DomainResult::success(effects);
                        }
                    }
                }
                InstrumentType::Bond(details) => {
                    let parties = state.financial_system.get_parties(instrument_id);
                    
                    if let Some((creditor, debtor)) = parties {
                        let face_value = details.face_value;
                        let last_date = details.last_accrual_date.unwrap_or(details.issue_date);
                        
                        let interest_amount = DayCount::ActAct.calculate_accrued_interest(
                            face_value,
                            details.coupon_rate_bps,
                            last_date,
                            current_date
                        );

                        if interest_amount > Money::from_f64(1e-6).unwrap_or_default() {
                            let effects = vec![
                                StateEffect::Financial(FinancialEffect::RecordTransaction(Transaction {
                                    id: uuid::Uuid::new_v4(),
                                    from_agent: debtor,
                                    to_agent: creditor,
                                    amount: interest_amount.to_f64(),
                                    transaction_type: "AccruedInterest".to_string(),
                                    timestamp: current_date,
                                    instrument_id: Some(*instrument_id),
                                    ref_id: None,
                                }))
                            ];
                            
                            return DomainResult::success(effects);
                        }
                    }
                }
                _ => {}
            }
        }

        DomainResult::empty()
    }

    fn execute_pay_interest(&self, instrument_id: &InstrumentId, state: &SimState) -> DomainResult {
        let fs = &state.financial_system;
        if let Some(instrument) = fs.instruments.get(instrument_id) {
            match &instrument.instrument_type {
                InstrumentType::Cash(details) => {
                    let parties = fs.get_parties(instrument_id);

                    if let Some((creditor, debtor)) = parties {
                        let principal = fs.balance_sheets.get(&creditor)
                            .and_then(|bs| bs.assets.get(instrument_id))
                            .map(|pos| pos.quantity)
                            .unwrap_or(0.0);

                        if principal <= 0.0 { 
                            return DomainResult::empty(); 
                        }

                        let principal_money = Money::from_f64(principal).unwrap_or_default();
                        let day_count = DayCount::Act365F;
                        let interest_amount = day_count.calculate_accrued_interest(
                            principal_money,
                            details.interest_bps,
                            state.current_date - chrono::Duration::days(1),
                            state.current_date
                        );

                        if interest_amount <= Money::from_f64(1e-9).unwrap_or_default() {
                            return DomainResult::empty();
                        }

                        let effects = vec![
                            StateEffect::Financial(FinancialEffect::TransferFunds { 
                                from: debtor, 
                                to: creditor, 
                                amount: interest_amount.to_f64(),
                                context: "InterestPayment".to_string() 
                            })
                        ];
                        
                        return DomainResult::success(effects);
                    }
                }
                _ => return DomainResult::failure(vec!["PayInterest is typically for deposits/loans, not other instruments.".to_string()])
            }
        }
        
        DomainResult::failure(vec!["Instrument not found".to_string()])
    }

    fn execute_process_coupon_payment(&self, instrument_id: &InstrumentId, state: &SimState) -> DomainResult {
        let fs = &state.financial_system;
        if let Some(instrument) = fs.instruments.get(instrument_id) {
            if let InstrumentType::Bond(details) = &instrument.instrument_type {
                
                let payment_per_bond = self.get_coupon_payment_amount(details);

                if payment_per_bond <= Money::from_f64(1e-9).unwrap_or_default() {
                    return DomainResult::empty();
                }

                let parties = fs.get_parties(instrument_id);
                
                if let Some((creditor, debtor)) = parties {
                    if creditor == debtor {
                        return DomainResult::empty();
                    }

                    let quantity_held = fs.balance_sheets.get(&creditor)
                        .and_then(|bs| bs.assets.get(instrument_id))
                        .map(|pos| pos.quantity)
                        .unwrap_or(0.0);
                    
                    let total_payment = payment_per_bond * quantity_held;

                    if total_payment <= Money::from_f64(1e-9).unwrap_or_default() {
                        return DomainResult::empty();
                    }

                    let effects = self.create_payment_effects(debtor, creditor, total_payment.to_f64());
                    return DomainResult::success(effects);
                } else {
                     return DomainResult::failure(vec!["Parties not found for bond.".to_string()]);
                }

            } else {
                DomainResult::failure(vec!["Instrument is not a bond".to_string()])
            }
        } else {
            DomainResult::failure(vec!["Instrument not found".to_string()])
        }
    }

    fn get_coupon_payment_amount(&self, details: &BondDetails) -> Money {
        if details.frequency == 0 { return Money::ZERO; }
        details.face_value * bps_to_decimal(details.coupon_rate_bps) / Decimal::from(details.frequency)
    }

    fn create_payment_effects(&self, from: AgentId, to: AgentId, amount: f64) -> Vec<StateEffect> {
        vec![
            StateEffect::Financial(FinancialEffect::TransferFunds {
                from,
                to,
                amount,
                context: "CouponPayment".to_string(),
            }),
        ]
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Settlement",
        constructor: || Box::new(SettlementDomain::new()),
    }
}