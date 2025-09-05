use crate::types::money::Money;
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use tracing::{Level, event};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]

pub enum FinancialEffect {
    CreateInstrument { instrument: Instrument, creditor: AgentId, debtor: AgentId, quantity: f64 },
    RecordTransaction(Transaction),
    RecordSettlementInstruction(SettlementInstruction),
    QueuePayment(PaymentInstruction),
    DvPFinalize { trade_id: Uuid },
    DvPCancel { trade_id: Uuid },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PositionSide {
    Asset,
    Liability,
}

impl FinancialEffect {
    pub fn name(&self) -> &'static str {
        match self {
            FinancialEffect::CreateInstrument { .. } => "CreateInstrument",
            FinancialEffect::RecordTransaction(_) => "RecordTransaction",
            FinancialEffect::RecordSettlementInstruction(_) => "RecordSettlementInstruction",
            FinancialEffect::DvPFinalize { .. } => "DvPFinalize",
            FinancialEffect::DvPCancel { .. } => "DvPCancel",
            FinancialEffect::QueuePayment(_) => "QueuePayment",
        }
    }
}

fn is_security(inst: &Instrument) -> bool {
    matches!(
        inst.instrument_type,
        InstrumentType::Bond(_)
            | InstrumentType::Equity(_)
            | InstrumentType::StructuredTranche(_)
            | InstrumentType::Derivative(_)
    )
}

impl StateEffectApplicator {
    pub fn apply_financial_effect(state: &mut SimState, effect: &FinancialEffect) -> Result<(), EffectError> {
        match effect {
            FinancialEffect::CreateInstrument { instrument: inst, creditor, debtor, quantity } => {
                let instrument_id = inst.id;
                state.financial_system.instruments.insert(instrument_id, inst.clone());

                if is_security(inst) {
                    state
                        .financial_system
                        .clearing_house
                        .csd
                        .register_security(instrument_id, inst, state.current_date)
                        .map_err(|e| EffectError::FinancialSystemError(e.to_string()))?;

                    if *quantity > 0.0 {
                        state
                            .financial_system
                            .clearing_house
                            .csd
                            .credit_securities(*creditor, instrument_id, *quantity)
                            .map_err(|e| EffectError::FinancialSystemError(e.to_string()))?;
                    }

                    event!(Level::INFO,
                        instrument_id = %instrument_id,
                        creditor = %creditor,
                        quantity = *quantity,
                        "📈 Security created and credited ONLY in CSD"
                    );
                } else {
                    let book_value = match &inst.instrument_type {
                        InstrumentType::Cash(_) => 1.0,
                        InstrumentType::RealAsset(RealAssetType::Property { market_value, .. }) => {
                            market_value.to_f64()
                        }
                        InstrumentType::RealAsset(RealAssetType::Inventory { goods, .. }) => {
                            goods.values().map(|item| (item.unit_cost * item.quantity).to_f64()).sum()
                        }
                        _ => 0.0,
                    };

                    if *quantity > 0.0 {
                        state
                            .financial_system
                            .create_or_consolidate_position(creditor, debtor, &instrument_id, *quantity, book_value)
                            .map_err(EffectError::FinancialSystemError)?;
                    }

                    event!(Level::INFO,
                        instrument_id = %instrument_id,
                        creditor = %creditor,
                        debtor = %debtor,
                        quantity = *quantity,
                        "💵 Cash/RealAsset created ONLY in balance sheets"
                    );
                }

                let final_inst = state.financial_system.instruments.get(&instrument_id).unwrap();
                if final_inst.should_create_order_book() {
                    state.financial_system.exchange.ensure_listed(instrument_id, final_inst);
                }

                Ok(())
            }
            FinancialEffect::RecordTransaction(tx) => {
                state.history.transactions.push(tx.clone());
                Ok(())
            }
            FinancialEffect::RecordSettlementInstruction(instruction) => {
                // Extract needed data before mutable borrow
                let government_id = state.financial_system.government.id;
                let instrument_name = state
                    .financial_system
                    .instruments
                    .get(&instruction.instrument_id)
                    .map(|inst| inst.type_as_string())
                    .unwrap_or("Unknown");
                let current_date = state.current_date;

                // Now safe to mutably borrow
                state
                    .financial_system
                    .clearing_house
                    .csd
                    .reserve_securities_for_dvp(instruction.clone(), government_id, instrument_name, current_date)
                    .map_err(|e| EffectError::FinancialSystemError(e.to_string()))?;
                Ok(())
            }
            FinancialEffect::QueuePayment(pi) => {
                state.financial_system.rtgs.pending.push(pi.clone());
                Ok(())
            }
            FinancialEffect::DvPCancel { trade_id } => {
                match state.financial_system.clearing_house.csd.cancel_security_reservation(trade_id) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        tracing::error!("CSD cancel_security_reservation error: {:?}", e);
                        return Err(EffectError::FinancialSystemError(e.to_string()));
                    }
                }
            }
            FinancialEffect::DvPFinalize { trade_id } => {
                match state.financial_system.clearing_house.csd.finalize_book_entry_transfer(trade_id) {
                    Ok(_) => {
                        event!(Level::INFO,
                            trade_id = %trade_id.to_string()[..8],
                            "✅ DvP fully settled - securities transferred in CSD"
                        );
                        Ok(())
                    }
                    Err(e) => {
                        tracing::error!("CSD finalize_book_entry_transfer error: {:?}", e);
                        Err(EffectError::FinancialSystemError(e.to_string()))
                    }
                }
            }
        }
    }
    pub fn apply_cash_position_adjustment(
        state: &mut SimState, agent_id: AgentId, instrument_id: InstrumentId, quantity_change: f64,
        side: &PositionSide, book_value: Option<f64>,
    ) -> Result<(), EffectError> {
        let instrument = state
            .financial_system
            .instruments
            .get(&instrument_id)
            .ok_or_else(|| EffectError::InstrumentNotFound { id: instrument_id })?;

        match &instrument.instrument_type {
            InstrumentType::Cash(_) => {}
            InstrumentType::RealAsset(_) => {}
            InstrumentType::Bond(_)
            | InstrumentType::Equity(_)
            | InstrumentType::StructuredTranche(_)
            | InstrumentType::Derivative(_) => {
                return Err(EffectError::FinancialSystemError(format!(
                    "Security {} cannot be adjusted via balance sheet - must use CSD",
                    instrument_id
                )));
            }
            InstrumentType::Repo(_) => {}
        }

        let bs = state.financial_system.balance_sheets.entry(agent_id).or_insert_with(|| BalanceSheet::new(agent_id));

        let positions = match side {
            PositionSide::Asset => &mut bs.assets,
            PositionSide::Liability => &mut bs.liabilities,
        };

        let new_qty: f64 = match positions.entry(instrument_id) {
            Entry::Occupied(mut e) => {
                let pos = e.get_mut();
                pos.quantity += quantity_change;
                let q = pos.quantity;
                if q.abs() < 1e-9 {
                    e.remove();
                    0.0
                } else {
                    q
                }
            }
            Entry::Vacant(e) => {
                if quantity_change.abs() < 1e-9 {
                    0.0
                } else {
                    let book_value_money: Money = book_value.and_then(Money::from_f64).unwrap_or(Money::ONE);
                    e.insert(Position {
                        quantity: quantity_change,
                        book_value_per_unit: book_value_money,
                        cost_basis_per_unit: book_value_money,
                    });
                    quantity_change
                }
            }
        };

        event!(
            Level::DEBUG,
            agent = %agent_id,
            instrument = %instrument_id,
            side = ?side,
            change = quantity_change,
            new_quantity = new_qty,
            "💵 Cash position adjusted"
        );

        Ok(())
    }
}
