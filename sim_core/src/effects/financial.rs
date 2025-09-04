use crate::types::money::Money;
use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::{event, Level};
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
    QueuePayment(PaymentInstruction),
    SettlePayment(PaymentId),
    DvPFinalize {
        trade_id: Uuid,
    },
    DvPCancel {
        trade_id: Uuid,
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
            FinancialEffect::AdjustPosition { .. } => "AdjustPosition",
            FinancialEffect::IssueAndOfferDebt { .. } => "IssueAndOfferDebt",
            FinancialEffect::DvPFinalize { .. } => "DvPFinalize",
            FinancialEffect::DvPCancel { .. } => "DvPCancel",
            FinancialEffect::QueuePayment(_) => "QueuePayment",
            FinancialEffect::SettlePayment(_) => "SettlePayment",
        }
    }
}

impl StateEffectApplicator {
    pub fn apply_financial_effect(state: &mut SimState, effect: &FinancialEffect) -> Result<(), EffectError> {
        match effect {
            FinancialEffect::AdjustPosition { owner, instrument_id, delta_quantity, side, cost_per_unit } => {
                Self::apply_adjust_position(state, *owner, *instrument_id, *delta_quantity, side, *cost_per_unit)
            }
            FinancialEffect::CreateInstrument { instrument: inst, creditor, debtor, quantity } => {
                let book_value = match &inst.instrument_type {
                    InstrumentType::Bond(d) => d.face_value.to_f64(),
                    InstrumentType::Cash(_) => 1.0,
                    _ => 0.0,
                };

                let new_inst_id = state
                    .financial_system
                    .create_or_consolidate_instrument(*creditor, *debtor, inst.clone(), *quantity, book_value)
                    .map_err(EffectError::FinancialSystemError)?;

                let final_inst = state.financial_system.instruments.get(&new_inst_id).unwrap();
                if final_inst.should_create_order_book() {
                    state.financial_system.exchange.ensure_listed(new_inst_id, final_inst);
                }
                Ok(())
            }
            FinancialEffect::UpdateInstrument { id, quantity_change } => {
                let (creditor_id, debtor_id) =
                    state.financial_system.get_parties(id).ok_or(EffectError::InstrumentNotFound { id: *id })?;
                state
                    .financial_system
                    .create_or_consolidate_position(&creditor_id, &debtor_id, id, *quantity_change, 0.0)
                    .map_err(EffectError::FinancialSystemError)
            }
            FinancialEffect::TransferInstrument { id, old_creditor, new_creditor } => state
                .financial_system
                .transfer_instrument(id, *old_creditor, *new_creditor)
                .map_err(EffectError::FinancialSystemError),
            FinancialEffect::RemoveInstrument(id) => {
                state.financial_system.remove_instrument(id).map_err(EffectError::FinancialSystemError)
            }
            FinancialEffect::RecordTransaction(tx) => {
                state.history.transactions.push(tx.clone());
                Ok(())
            }
            FinancialEffect::IssueAndOfferDebt { instrument, quantity, price } => {
                let government_id = instrument.get_consolidation_key().issuer;

                let face_value = instrument.face_value().unwrap_or(Money::from(1000 as i64)).to_f64();
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
                    .ok_or(EffectError::InstrumentNotFound { id: final_instrument_id })?;

                if final_inst.should_create_order_book() {
                    state.financial_system.exchange.ensure_listed(final_instrument_id, final_inst);
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

                let market_effect =
                    MarketEffect::PlaceOrderInBook { market_id: MarketId::Financial(final_instrument_id), order };

                Self::apply_market_effect(state, &market_effect)
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
                tracing::info!("Finalizing DvP for trade_id: {}", trade_id);
                Ok(())
            }
            FinancialEffect::QueuePayment(pi) => {
                state.financial_system.rtgs.pending.push(pi.clone());
                Ok(())
            }
            FinancialEffect::SettlePayment(pid) => settle_one_payment(state, *pid),
        }
    }

    pub fn apply_adjust_position(
        state: &mut SimState, owner: AgentId, instrument_id: InstrumentId, delta_quantity: f64, side: &PositionSide,
        cost_per_unit: Option<Money>,
    ) -> Result<(), EffectError> {
        let owner_type = state.get_agent_type_string(&owner).unwrap();
        let owner_name = format!("{} ({})", owner_type, owner.0.to_string()[..4].to_string());
        let instrument_name = state.financial_system.clone().get_instrument_info(&instrument_id, &state.agents, state.current_date).unwrap().instrument_type;

        let balance_sheet =
            state.financial_system.balance_sheets.get_mut(&owner).ok_or(EffectError::AgentNotFound { id: owner })?;

        let position_map = match side {
            PositionSide::Asset => &mut balance_sheet.assets,
            PositionSide::Liability => &mut balance_sheet.liabilities,
        };

        let old_quantity = position_map.get(&instrument_id).map(|p| p.quantity).unwrap_or(0.0);

        event!(Level::DEBUG,
            agent = %owner_name,
            agent_id = %owner,
            instrument = %instrument_name,
            instrument_id = %instrument_id,
            side = ?side,
            current_quantity = old_quantity,
            delta = delta_quantity,
            "📊 Pre-adjustment position\n"
        );

        let position = position_map.entry(instrument_id).or_insert_with(|| {
            let book_value = state
                .financial_system
                .instruments
                .get(&instrument_id)
                .and_then(|inst| inst.face_value())
                .unwrap_or(Money::from(1));

            Position {
                quantity: 0.0,
                book_value_per_unit: book_value,
                cost_basis_per_unit: cost_per_unit.unwrap_or(book_value),
            }
        });

        position.quantity += delta_quantity;
        let new_quantity = position.quantity;

        let value_change = delta_quantity * position.book_value_per_unit.to_f64();
        let impact = match side {
            PositionSide::Asset => value_change,
            PositionSide::Liability => -value_change,
        };

        event!(Level::INFO,
            agent = %owner_name,
            agent_id = %owner,
            instrument = %instrument_name,
            instrument_id = %instrument_id,
            side = ?side,
            old_quantity = old_quantity,
            new_quantity = new_quantity,
            delta = delta_quantity,
            book_value_per_unit = ?position.book_value_per_unit,
            cost_basis = ?position.cost_basis_per_unit,
            net_worth_impact = impact,
            "\n💰 Balance sheet adjusted"
        );

        if position.quantity <= 1e-9 {
            event!(Level::DEBUG,
                agent = %owner_name,
                instrument = %instrument_name,
                side = ?side,
                "🗑️ Position removed (quantity ~0)"
            );
            position_map.remove(&instrument_id);
        }

        Ok(())
    }

    pub fn apply_dvp(state: &mut SimState, trade: &Trade) -> Result<(), EffectError> {
        let total_payment = (trade.price * trade.quantity).to_f64();

        let (buyer_account_id, buyer_settlement_agent) = state
            .financial_system
            .find_agent_liquid_account(&trade.buyer)
            .ok_or_else(|| EffectError::InvalidState(format!("Buyer {} has no liquid account", trade.buyer)))?;
        let (seller_account_id, seller_settlement_agent) = state
            .financial_system
            .find_agent_liquid_account(&trade.seller)
            .ok_or_else(|| EffectError::InvalidState(format!("Seller {} has no liquid account", trade.seller)))?;

        if buyer_settlement_agent == seller_settlement_agent {
            let bank_id = buyer_settlement_agent;
            Self::apply_adjust_position(
                state,
                trade.buyer,
                buyer_account_id,
                -total_payment,
                &PositionSide::Asset,
                None,
            )?;
            Self::apply_adjust_position(
                state,
                trade.seller,
                seller_account_id,
                total_payment,
                &PositionSide::Asset,
                None,
            )?;
            Self::apply_adjust_position(
                state,
                bank_id,
                buyer_account_id,
                -total_payment,
                &PositionSide::Liability,
                None,
            )?;
            Self::apply_adjust_position(
                state,
                bank_id,
                seller_account_id,
                total_payment,
                &PositionSide::Liability,
                None,
            )?;
        } else {
            let buyer_bank_reserves =
                state.financial_system.find_bank_reserves_account(&buyer_settlement_agent).ok_or_else(|| {
                    EffectError::InvalidState(format!("Sending bank {} has no reserves.", buyer_settlement_agent))
                })?;
            let seller_bank_reserves =
                state.financial_system.find_bank_reserves_account(&seller_settlement_agent).ok_or_else(|| {
                    EffectError::InvalidState(format!("Receiving bank {} has no reserves.", seller_settlement_agent))
                })?;

            Self::apply_adjust_position(
                state,
                trade.buyer,
                buyer_account_id,
                -total_payment,
                &PositionSide::Asset,
                None,
            )?;
            Self::apply_adjust_position(
                state,
                buyer_settlement_agent,
                buyer_account_id,
                -total_payment,
                &PositionSide::Liability,
                None,
            )?;

            Self::apply_adjust_position(
                state,
                buyer_settlement_agent,
                buyer_bank_reserves,
                -total_payment,
                &PositionSide::Asset,
                None,
            )?;
            Self::apply_adjust_position(
                state,
                seller_settlement_agent,
                seller_bank_reserves,
                total_payment,
                &PositionSide::Asset,
                None,
            )?;

            Self::apply_adjust_position(
                state,
                trade.seller,
                seller_account_id,
                total_payment,
                &PositionSide::Asset,
                None,
            )?;
            Self::apply_adjust_position(
                state,
                seller_settlement_agent,
                seller_account_id,
                total_payment,
                &PositionSide::Liability,
                None,
            )?;
        }

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
}
