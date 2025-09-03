use crate::types::money::Money;
use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FinancialEffect {
    CreateInstrument {
        instrument: Instrument,
        creditor: AgentId,
        debtor: AgentId,
        quantity: f64,
    },
    UpdateInstrument {
        id: InstrumentId,
        quantity_change: f64,
    },
    TransferInstrument {
        id: InstrumentId,
        old_creditor: AgentId,
        new_creditor: AgentId,
    },
    RemoveInstrument(InstrumentId),
    RecordTransaction(Transaction),
    TransferFunds {
        from: AgentId,
        to: AgentId,
        amount: f64,
        context: String,
    },
    PayWages {
        employer: AgentId,
        employee: AgentId,
        amount: f64,
    },
    AdjustPosition {
        owner: AgentId,
        instrument_id: InstrumentId,
        delta_quantity: f64,
        side: PositionSide,
        cost_per_unit: Option<Money>,
    },
    IssueAndOfferDebt {
        instrument: Instrument,
        quantity: u32,
        price: Money,
    },
    DvP {
        trade: Trade,
    },
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
            FinancialEffect::UpdateInstrument { .. } => "UpdateInstrument",
            FinancialEffect::TransferInstrument { .. } => "TransferInstrument",
            FinancialEffect::RemoveInstrument(_) => "RemoveInstrument",
            FinancialEffect::RecordTransaction(_) => "RecordTransaction",
            FinancialEffect::TransferFunds { .. } => "TransferFunds",
            FinancialEffect::PayWages { .. } => "PayWages",
            FinancialEffect::AdjustPosition { .. } => "AdjustPosition",
            FinancialEffect::IssueAndOfferDebt { .. } => "IssueAndOfferDebt",
            FinancialEffect::DvP { .. } => "DvP",
        }
    }
}

impl StateEffectApplicator {
    pub fn apply_financial_effect(
        state: &mut SimState,
        effect: &FinancialEffect,
    ) -> Result<(), EffectError> {
        match effect {
            FinancialEffect::AdjustPosition {
                owner,
                instrument_id,
                delta_quantity,
                side,
                cost_per_unit,
            } => Self::apply_adjust_position(
                state,
                *owner,
                *instrument_id,
                *delta_quantity,
                side,
                *cost_per_unit,
            ),
            FinancialEffect::CreateInstrument {
                instrument: inst,
                creditor,
                debtor,
                quantity,
            } => {
                let book_value = match &inst.instrument_type {
                    InstrumentType::Bond(d) => d.face_value.to_f64(),
                    InstrumentType::Cash(_) => 1.0,
                    _ => 0.0,
                };

                let new_inst_id = state
                    .financial_system
                    .create_or_consolidate_instrument(
                        *creditor,
                        *debtor,
                        inst.clone(),
                        *quantity,
                        book_value,
                    )
                    .map_err(EffectError::FinancialSystemError)?;

                let final_inst = state
                    .financial_system
                    .instruments
                    .get(&new_inst_id)
                    .unwrap();
                if final_inst.should_create_order_book() {
                    state
                        .financial_system
                        .exchange
                        .ensure_listed(new_inst_id, final_inst);
                }
                Ok(())
            }
            FinancialEffect::UpdateInstrument {
                id,
                quantity_change,
            } => {
                let (creditor_id, debtor_id) = state
                    .financial_system
                    .get_parties(id)
                    .ok_or(EffectError::InstrumentNotFound { id: *id })?;
                state
                    .financial_system
                    .create_or_consolidate_position(
                        &creditor_id,
                        &debtor_id,
                        id,
                        *quantity_change,
                        0.0,
                    )
                    .map_err(EffectError::FinancialSystemError)
            }
            FinancialEffect::TransferInstrument {
                id,
                old_creditor,
                new_creditor,
            } => state
                .financial_system
                .transfer_instrument(id, *old_creditor, *new_creditor)
                .map_err(EffectError::FinancialSystemError),
            FinancialEffect::RemoveInstrument(id) => state
                .financial_system
                .remove_instrument(id)
                .map_err(EffectError::FinancialSystemError),
            FinancialEffect::RecordTransaction(tx) => {
                state.history.transactions.push(tx.clone());
                Ok(())
            }
            FinancialEffect::TransferFunds {
                from,
                to,
                amount,
                context,
            } => Self::apply_transfer_funds(state, *from, *to, *amount, context.clone()),
            FinancialEffect::PayWages {
                employer,
                employee,
                amount,
            } => Self::apply_transfer_funds(
                state,
                *employer,
                *employee,
                *amount,
                "WagePayment".to_string(),
            ),
            FinancialEffect::IssueAndOfferDebt {
                instrument,
                quantity,
                price,
            } => {
                let government_id = instrument.get_consolidation_key().issuer;

                let face_value = instrument
                    .face_value()
                    .unwrap_or(Money::from(1000 as i64))
                    .to_f64();
                let final_instrument_id = state
                    .financial_system
                    .create_or_consolidate_instrument(
                        government_id,
                        government_id,
                        instrument.clone(),
                        *quantity as f64,
                        face_value,
                    )
                    .map_err(EffectError::FinancialSystemError)?;

                let final_inst = state
                    .financial_system
                    .instruments
                    .get(&final_instrument_id)
                    .ok_or(EffectError::InstrumentNotFound {
                        id: final_instrument_id,
                    })?;

                if final_inst.should_create_order_book() {
                    state
                        .financial_system
                        .exchange
                        .ensure_listed(final_instrument_id, final_inst);
                }

                let money_price = price;
                let order = Order {
                    id: Uuid::new_v4(),
                    agent_id: government_id,
                    side: Side::Ask,
                    quantity: *quantity as f64,
                    price: Some(*money_price),
                    order_type: OrderType::Limit,
                };

                let market_effect = MarketEffect::PlaceOrderInBook {
                    market_id: MarketId::Financial(final_instrument_id),
                    order,
                };

                Self::apply_market_effect(state, &market_effect)
            }
            FinancialEffect::DvP { trade } => Self::apply_dvp(state, trade),
        }
    }

    fn apply_adjust_position(
        state: &mut SimState,
        owner: AgentId,
        instrument_id: InstrumentId,
        delta_quantity: f64,
        side: &PositionSide,
        cost_per_unit: Option<Money>,
    ) -> Result<(), EffectError> {
        let balance_sheet = state
            .financial_system
            .balance_sheets
            .get_mut(&owner)
            .ok_or(EffectError::AgentNotFound { id: owner })?;

        let position_map = match side {
            PositionSide::Asset => &mut balance_sheet.assets,
            PositionSide::Liability => &mut balance_sheet.liabilities,
        };

        let position = position_map.entry(instrument_id).or_insert_with(|| {
            let book_value = state
                .financial_system
                .instruments
                .get(&instrument_id)
                .and_then(|inst| inst.face_value()) // .face_value() returns Option<Money>
                .unwrap_or(Money::from(1)); // Default to 1.00 for cash or other par-value assets

            Position {
                quantity: 0.0,
                book_value_per_unit: book_value,
                cost_basis_per_unit: cost_per_unit.unwrap_or(book_value),
            }
        });

        position.quantity += delta_quantity;

        if position.quantity <= 1e-9 {
            position_map.remove(&instrument_id);
        }

        Ok(())
    }

    fn apply_dvp(state: &mut SimState, trade: &Trade) -> Result<(), EffectError> {
        let total_payment = (trade.price * trade.quantity).to_f64();

        Self::apply_transfer_funds(
            state,
            trade.buyer,
            trade.seller,
            total_payment,
            format!("DvP settlement for trade in market {}", trade.market_id),
        )?;

        match &trade.market_id {
            MarketId::Goods(good_id) => {
                Self::apply_inventory_effect(
                    state,
                    &InventoryEffect::RemoveInventory {
                        owner: trade.seller,
                        good_id: *good_id,
                        quantity: trade.quantity,
                    },
                )?;
                Self::apply_inventory_effect(
                    state,
                    &InventoryEffect::AddInventory {
                        owner: trade.buyer,
                        good_id: *good_id,
                        quantity: trade.quantity,
                        unit_cost: trade.price.to_f64(),
                    },
                )?;
            }
            MarketId::Financial(instrument_id) => {
                Self::apply_adjust_position(
                    state,
                    trade.seller,
                    *instrument_id,
                    -trade.quantity,
                    &PositionSide::Asset,
                    None,
                )?;
                Self::apply_adjust_position(
                    state,
                    trade.buyer,
                    *instrument_id,
                    trade.quantity,
                    &PositionSide::Asset,
                    Some(trade.price),
                )?;
            }
            MarketId::Labour(_) => { /* No asset leg for labour trades */ }
        }

        Ok(())
    }

    fn apply_transfer_funds(
        state: &mut SimState,
        from: AgentId,
        to: AgentId,
        amount: f64,
        context: String,
    ) -> Result<(), EffectError> {
        if amount <= 1e-9 {
            return Ok(());
        }

        let (from_account_id, from_settlement_agent_id) =
            Self::find_agent_liquid_account(state, &from).ok_or_else(|| {
                EffectError::TransactionFailure(
                    "TransferFunds".to_string(),
                    format!(
                        "Sender {} has no valid liquid account (deposit or reserves).",
                        from
                    ),
                )
            })?;

        let from_bs = state.financial_system.balance_sheets.get(&from).unwrap();
        let from_pos = from_bs.assets.get(&from_account_id).unwrap();
        if from_pos.quantity < amount {
            return Err(EffectError::TransactionFailure(
                "TransferFunds".to_string(),
                format!(
                    "Insufficient funds for agent {}: have ${:.2}, need ${:.2}",
                    from, from_pos.quantity, amount
                ),
            ));
        }

        let (to_account_id, to_settlement_agent_id) =
            Self::get_or_create_agent_liquid_account(state, &to)?;

        let mut effects_to_apply = Vec::new();

        if from_settlement_agent_id == to_settlement_agent_id {
            effects_to_apply.extend(vec![
                FinancialEffect::AdjustPosition {
                    owner: from,
                    instrument_id: from_account_id,
                    delta_quantity: -amount,
                    side: PositionSide::Asset,
                    cost_per_unit: None,
                },
                FinancialEffect::AdjustPosition {
                    owner: to,
                    instrument_id: to_account_id,
                    delta_quantity: amount,
                    side: PositionSide::Asset,
                    cost_per_unit: None,
                },
                FinancialEffect::AdjustPosition {
                    owner: from_settlement_agent_id,
                    instrument_id: from_account_id,
                    delta_quantity: -amount,
                    side: PositionSide::Liability,
                    cost_per_unit: None,
                },
                FinancialEffect::AdjustPosition {
                    owner: to_settlement_agent_id,
                    instrument_id: to_account_id,
                    delta_quantity: amount,
                    side: PositionSide::Liability,
                    cost_per_unit: None,
                },
            ]);
        } else {
            let from_bank_reserves_id =
                Self::find_bank_reserves_account(state, &from_settlement_agent_id).ok_or_else(
                    || {
                        EffectError::TransactionFailure(
                            "TransferFunds".to_string(),
                            "Sending bank has no reserves.".to_string(),
                        )
                    },
                )?;
            let to_bank_reserves_id =
                Self::find_bank_reserves_account(state, &to_settlement_agent_id).ok_or_else(
                    || {
                        EffectError::TransactionFailure(
                            "TransferFunds".to_string(),
                            "Receiving bank has no reserves.".to_string(),
                        )
                    },
                )?;

            let from_bank_bs = state
                .financial_system
                .balance_sheets
                .get(&from_settlement_agent_id)
                .unwrap();
            let reserve_pos = from_bank_bs.assets.get(&from_bank_reserves_id).unwrap();
            if reserve_pos.quantity < amount {
                return Err(EffectError::TransactionFailure(
                    "TransferFunds".to_string(),
                    format!(
                        "Bank {} has insufficient reserves for settlement.",
                        from_settlement_agent_id
                    ),
                ));
            }

            effects_to_apply.extend(vec![
                FinancialEffect::AdjustPosition {
                    owner: from,
                    instrument_id: from_account_id,
                    delta_quantity: -amount,
                    side: PositionSide::Asset,
                    cost_per_unit: None,
                },
                FinancialEffect::AdjustPosition {
                    owner: to,
                    instrument_id: to_account_id,
                    delta_quantity: amount,
                    side: PositionSide::Asset,
                    cost_per_unit: None,
                },
                FinancialEffect::AdjustPosition {
                    owner: from_settlement_agent_id,
                    instrument_id: from_account_id,
                    delta_quantity: -amount,
                    side: PositionSide::Liability,
                    cost_per_unit: None,
                },
                FinancialEffect::AdjustPosition {
                    owner: to_settlement_agent_id,
                    instrument_id: to_account_id,
                    delta_quantity: amount,
                    side: PositionSide::Liability,
                    cost_per_unit: None,
                },
                FinancialEffect::AdjustPosition {
                    owner: from_settlement_agent_id,
                    instrument_id: from_bank_reserves_id,
                    delta_quantity: -amount,
                    side: PositionSide::Asset,
                    cost_per_unit: None,
                },
                FinancialEffect::AdjustPosition {
                    owner: to_settlement_agent_id,
                    instrument_id: to_bank_reserves_id,
                    delta_quantity: amount,
                    side: PositionSide::Asset,
                    cost_per_unit: None,
                },
            ]);
        }

        effects_to_apply.push(FinancialEffect::RecordTransaction(Transaction {
            id: Uuid::new_v4(),
            from_agent: from,
            to_agent: to,
            amount,
            transaction_type: context,
            timestamp: state.current_date,
            instrument_id: None,
            ref_id: None,
        }));

        for effect in effects_to_apply {
            Self::apply_financial_effect(state, &effect)?;
        }
        Ok(())
    }

    fn find_bank_reserves_account(state: &SimState, bank_id: &AgentId) -> Option<InstrumentId> {
        let bs = state.financial_system.balance_sheets.get(bank_id)?;
        bs.assets.iter().find_map(|(id, _pos)| {
            let inst = state.financial_system.instruments.get(id)?;
            match &inst.instrument_type {
                InstrumentType::Cash(details)
                    if details.cash_type == CashType::CentralBankReserves =>
                {
                    Some(*id)
                }
                _ => None,
            }
        })
    }

    fn find_agent_liquid_account(
        state: &SimState,
        agent_id: &AgentId,
    ) -> Option<(InstrumentId, AgentId)> {
        if state.agents.banks.contains_key(agent_id)
            || *agent_id == state.financial_system.government.id
            || *agent_id == state.financial_system.central_bank.id
        {
            Self::find_bank_reserves_account(state, agent_id)
                .map(|reserves_id| (reserves_id, *agent_id)) // <<< THE FIX IS HERE
        } else {
            let bs = state.financial_system.balance_sheets.get(agent_id)?;
            bs.assets.iter().find_map(|(id, _pos)| {
                let inst = state.financial_system.instruments.get(id)?;
                match &inst.instrument_type {
                    InstrumentType::Cash(details)
                        if details.cash_type == CashType::DemandDeposit =>
                    {
                        Some((*id, details.issuer))
                    }
                    _ => None,
                }
            })
        }
    }

    fn get_or_create_agent_liquid_account(
        state: &mut SimState,
        agent_id: &AgentId,
    ) -> Result<(InstrumentId, AgentId), EffectError> {
        if let Some(account) = Self::find_agent_liquid_account(state, agent_id) {
            return Ok(account);
        }

        let bank_id = state.agents.banks.keys().next().cloned().ok_or_else(|| {
            EffectError::InvalidState(
                "No banks in the simulation to open an account with.".to_string(),
            )
        })?;

        let rate = state.financial_system.central_bank.policy_rate_bps;
        let deposit_inst = Instrument::cash(
            InstrumentId(Uuid::new_v4()),
            bank_id,
            CashType::DemandDeposit,
            Currency::USD,
            rate,
        )
        .build();
        let inst_id = deposit_inst.id;

        state
            .financial_system
            .create_instrument(*agent_id, bank_id, deposit_inst, 0.0, 1.0)
            .map_err(EffectError::FinancialSystemError)?;

        Ok((inst_id, bank_id))
    }
}