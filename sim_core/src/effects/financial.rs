use crate::types::instrument::archetypes::{InstrumentArchetype, LifecycleRules, MarketProfile, ProductFamily};
use crate::types::instrument::inst_registry::{InstrumentTemplate, LotQuantity, LotType, TemplateId};
use crate::types::money::Money;
use crate::types::system::balance_sheet::Position;
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FinancialEffect {
    CreateInstrument { instrument: Instrument, creditor: AgentId, debtor: AgentId, quantity: f64 },
    RedeemInstrument { instrument_id: InstrumentId },
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
            FinancialEffect::RedeemInstrument { .. } => "RedeemInstrument",
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
        inst.state(),
        InstrumentRuntime::Bond(_)
            | InstrumentRuntime::Equity(_)
            | InstrumentRuntime::Structured(_)
            | InstrumentRuntime::Derivative(_)
    )
}

impl StateEffectApplicator {
    pub fn apply_financial_effect(state: &mut SimState, effect: &FinancialEffect) -> Result<(), EffectError> {
        match effect {
            FinancialEffect::CreateInstrument { instrument: inst, creditor, debtor, quantity } => {
                let instrument_id = inst.instrument_id();
                let par_value = inst.unit_par_value().unwrap_or(Money::ONE);

                if let InstrumentRuntime::Bond(bond_state) = inst.state() {
                    let template_id = state
                        .financial_system
                        .instrument_registry
                        .templates
                        .iter()
                        .find(|(_, t)| matches!(t.archetype, InstrumentArchetype::Bond(_)))
                        .map(|(id, _)| *id)
                        .unwrap_or_else(|| {
                            let template = InstrumentTemplate {
                                id: TemplateId(Uuid::new_v4()),
                                product_family: ProductFamily::FixedIncome,
                                archetype: InstrumentArchetype::Bond(bond_state.archetype.clone()),
                                market_profile: MarketProfile::unlisted(),
                                lifecycle_rules: LifecycleRules {
                                    requires_authorization: false,
                                    supports_partial_redemption: false,
                                    accrual_method: None,
                                    settlement_lag_days: 0,
                                },
                                created_date: state.current_date,
                            };
                            let id = template.id;
                            state.financial_system.instrument_registry.register_template(template).ok();
                            id
                        });

                    let series_id = state
                        .financial_system
                        .instrument_registry
                        .ensure_bond_series(
                            template_id,
                            bond_state.issuer,
                            bond_state.archetype.clone(),
                            bond_state.issue_date,
                            bond_state.maturity_date,
                        )
                        .map_err(|e| EffectError::FinancialSystemError(e))?;

                    if *quantity > 0.0 {
                        state
                            .financial_system
                            .instrument_registry
                            .register_existing_lot(
                                series_id,
                                instrument_id,
                                LotType::Fungible { lot_size: bond_state.archetype.face_value.to_f64() },
                                LotQuantity::Units(*quantity),
                            )
                            .map_err(|e| EffectError::FinancialSystemError(e))?;
                    }

                    tracing::debug!(
                        "Registered bond {} through registry: series {}, template {}",
                        instrument_id,
                        series_id,
                        template_id
                    );
                } else {
                    tracing::debug!(
                        "Creating instrument {} bypassing registry (type: {})",
                        instrument_id,
                        inst.type_as_string()
                    );
                }

                state.financial_system.instruments.instruments.insert(instrument_id, inst.clone());

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

                        let creditor_bs = state
                            .financial_system
                            .balance_sheets
                            .entry(*creditor)
                            .or_insert_with(|| BalanceSheet::new(*creditor));
                        let creditor_pos = creditor_bs.assets.entry(instrument_id).or_insert_with(|| Position {
                            quantity: 0.0,
                            book_value_per_unit: par_value,
                            cost_basis_per_unit: par_value,
                        });
                        creditor_pos.quantity += quantity;
                        creditor_pos.book_value_per_unit = par_value;
                        creditor_pos.cost_basis_per_unit = par_value;
                    }

                    let debtor_bs = state
                        .financial_system
                        .balance_sheets
                        .entry(*debtor)
                        .or_insert_with(|| BalanceSheet::new(*debtor));
                    let pos = debtor_bs.liabilities.entry(instrument_id).or_insert_with(|| Position {
                        quantity: 0.0,
                        book_value_per_unit: par_value,
                        cost_basis_per_unit: par_value,
                    });
                    pos.quantity += quantity;
                    pos.book_value_per_unit = par_value;
                    pos.cost_basis_per_unit = par_value;
                } else {
                    if *quantity > 0.0 {
                        {
                            let creditor_bs = state
                                .financial_system
                                .balance_sheets
                                .entry(*creditor)
                                .or_insert_with(|| BalanceSheet::new(*creditor));
                            let creditor_pos = creditor_bs.assets.entry(instrument_id).or_insert_with(|| Position {
                                quantity: 0.0,
                                book_value_per_unit: par_value,
                                cost_basis_per_unit: par_value,
                            });
                            creditor_pos.quantity += quantity;
                            creditor_pos.book_value_per_unit = par_value;
                            creditor_pos.cost_basis_per_unit = par_value;
                        }

                        {
                            let debtor_bs = state
                                .financial_system
                                .balance_sheets
                                .entry(*debtor)
                                .or_insert_with(|| BalanceSheet::new(*debtor));
                            let debtor_pos = debtor_bs.liabilities.entry(instrument_id).or_insert_with(|| Position {
                                quantity: 0.0,
                                book_value_per_unit: par_value,
                                cost_basis_per_unit: par_value,
                            });
                            debtor_pos.quantity += quantity;
                            debtor_pos.book_value_per_unit = par_value;
                            debtor_pos.cost_basis_per_unit = par_value;
                        }

                        tracing::debug!(
                            "Created instrument {}: creditor {} asset position {:.2}, debtor {} liability position {:.2}",
                            instrument_id,
                            creditor,
                            quantity,
                            debtor,
                            quantity
                        );
                    }
                }

                let final_inst = state.financial_system.instruments.instruments.get(&instrument_id).unwrap();
                if final_inst.listability().should_create_order_book() {
                    state.financial_system.exchange.ensure_listed(instrument_id, final_inst);
                }

                Ok(())
            }

            FinancialEffect::RedeemInstrument { instrument_id } => {
                let mut agents_with_holdings = Vec::new();
                for (agent_id, account) in &state.financial_system.clearing_house.csd.custody_accounts {
                    if let Some(holding) = account.holdings.get(instrument_id) {
                        let qty = holding.total_position();
                        if qty > 1e-9 {
                            agents_with_holdings.push((*agent_id, qty));
                        }
                    }
                }

                for (agent_id, qty) in agents_with_holdings {
                    state.financial_system.clearing_house.csd.debit_securities(agent_id, *instrument_id, qty).map_err(
                        |e| EffectError::InvalidState(format!("Failed to debit securities for {}: {}", agent_id, e)),
                    )?;
                    tracing::debug!("Debited {} units of {} from {} custody account", qty, instrument_id, agent_id);
                }

                let mut agents_with_liabilities = Vec::new();
                for (agent_id, bs) in &state.financial_system.balance_sheets {
                    if bs.liabilities.contains_key(instrument_id) {
                        agents_with_liabilities.push(*agent_id);
                    }
                }

                for agent_id in agents_with_liabilities {
                    if let Some(bs) = state.financial_system.balance_sheets.get_mut(&agent_id) {
                        bs.liabilities.remove(instrument_id);
                        tracing::debug!("Removed {} from {} balance sheet liabilities", instrument_id, agent_id);
                    }
                }

                let mut agents_with_assets = Vec::new();
                for (agent_id, bs) in &state.financial_system.balance_sheets {
                    if bs.assets.contains_key(instrument_id) {
                        agents_with_assets.push(*agent_id);
                    }
                }

                for agent_id in agents_with_assets {
                    if let Some(bs) = state.financial_system.balance_sheets.get_mut(&agent_id) {
                        bs.assets.remove(instrument_id);
                        tracing::debug!("Removed {} from {} balance sheet assets", instrument_id, agent_id);
                    }
                }

                if let Some(lot_status) = state.financial_system.instrument_registry.get_lot_status(instrument_id) {
                    if lot_status != crate::types::instrument::inst_registry::LotStatus::Redeemed {
                        state.financial_system.instrument_registry.redeem_lot(*instrument_id).map_err(|e| {
                            EffectError::InvalidState(format!("Failed to redeem lot in registry: {}", e))
                        })?;
                        tracing::debug!("Redeemed lot {} from InstrumentRegistry", instrument_id);
                    }
                }

                state
                    .financial_system
                    .instruments
                    .redeem_instrument(*instrument_id)
                    .map_err(|e| EffectError::InvalidState(format!("Failed to remove from catalog: {}", e)))?;

                tracing::info!("Redeemed instrument: {}", instrument_id);
                Ok(())
            }

            FinancialEffect::RecordTransaction(tx) => {
                state.history.transactions.push(tx.clone());
                Ok(())
            }

            FinancialEffect::RecordSettlementInstruction(instruction) => {
                let government_id = state.financial_system.government.id;
                let instrument_name = state
                    .financial_system
                    .instruments
                    .instruments
                    .get(&instruction.instrument_id)
                    .map(|inst| inst.type_as_string())
                    .unwrap_or("Unknown");
                let current_date = state.current_date;

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
                    Ok(_) => Ok(()),
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
            .instruments
            .get(&instrument_id)
            .ok_or_else(|| EffectError::InstrumentNotFound { id: instrument_id })?;

        match instrument.state() {
            InstrumentRuntime::Cash(_) => {}
            InstrumentRuntime::RealAsset(_) => {}
            InstrumentRuntime::Bond(_) => {
                return Err(EffectError::FinancialSystemError(format!(
                    "Bond {} cannot be adjusted via balance sheet - must use CSD",
                    instrument_id
                )));
            }
            InstrumentRuntime::Equity(_) | InstrumentRuntime::Structured(_) | InstrumentRuntime::Derivative(_) => {
                return Err(EffectError::FinancialSystemError(format!(
                    "Security {} cannot be adjusted via balance sheet - must use CSD",
                    instrument_id
                )));
            }
            InstrumentRuntime::Credit(_) => {}
            InstrumentRuntime::Repo(_) => {}
        }

        let bs = state.financial_system.balance_sheets.entry(agent_id).or_insert_with(|| BalanceSheet::new(agent_id));

        let positions = match side {
            PositionSide::Asset => &mut bs.assets,
            PositionSide::Liability => &mut bs.liabilities,
        };

        let _new_qty: f64 = match positions.entry(instrument_id) {
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

        Ok(())
    }
}
