use serde::{Deserialize, Serialize};
use thiserror::Error;
use strum_macros::{Display, EnumString};
use crate::*;

#[derive(Clone, Debug, Serialize, Deserialize, Display, EnumString)]
pub enum StateEffect {
    CreateInstrument(FinancialInstrument),
    UpdateInstrument { id: InstrumentId, new_principal: f64 },
    TransferInstrument { id: InstrumentId, new_creditor: AgentId },
    RemoveInstrument(InstrumentId),
    SwapInstrument { id: InstrumentId, new_debtor: AgentId, new_creditor: AgentId },

    AddInventory { owner: AgentId, good_id: GoodId, quantity: f64, unit_cost: f64 },
    RemoveInventory { owner: AgentId, good_id: GoodId, quantity: f64 },

    RecordTransaction(Transaction),

    UpdateConsumerIncome { id: AgentId, new_income: f64 },
    UpdateFirmRevenue { id: AgentId, revenue: f64 },

    Hire { firm: AgentId, count: u32 },
    Produce { firm: AgentId, good_id: GoodId, amount: f64 },

    PlaceOrderInBook { market_id: MarketId, order: Order },
}

impl StateEffect {
    pub fn apply(&self, state: &mut SimState) -> Result<(), EffectError> {
        match self {
            StateEffect::CreateInstrument(inst) => state
                .financial_system
                .create_or_consolidate_instrument(inst.clone())
                .map(|_| ())
                .map_err(|e| EffectError::FinancialSystemError(e)),

            StateEffect::UpdateInstrument { id, new_principal } => state
                .financial_system
                .update_instrument(id, *new_principal)
                .map_err(|_| EffectError::InstrumentNotFound { id: *id }),

            StateEffect::RemoveInstrument(id) => {
                state.financial_system.remove_instrument(id).map_err(|_| EffectError::InstrumentNotFound { id: *id })
            }

            StateEffect::TransferInstrument { id, new_creditor } => state
                .financial_system
                .transfer_instrument(id, *new_creditor)
                .map_err(|_| EffectError::InstrumentNotFound { id: *id }),

            StateEffect::SwapInstrument { id, new_debtor, new_creditor } => state
                .financial_system
                .swap_instrument(id, new_debtor, new_creditor)
                .map_err(|_| EffectError::InstrumentNotFound { id: *id }),

            StateEffect::AddInventory { owner, good_id, quantity, unit_cost } => {
                state
                    .financial_system
                    .balance_sheets
                    .get_mut(owner)
                    .ok_or_else(|| EffectError::AgentNotFound { id: *owner })?
                    .add_to_inventory(good_id, *quantity, *unit_cost);
                Ok(())
            }

            StateEffect::RemoveInventory { owner, good_id, quantity } => {
                let bs = state
                    .financial_system
                    .balance_sheets
                    .get_mut(owner)
                    .ok_or_else(|| EffectError::AgentNotFound { id: *owner })?;

                let current = bs
                    .real_assets
                    .values()
                    .filter_map(|asset| match &asset.asset_type {
                        RealAssetType::Inventory { goods } => goods.get(good_id),
                        _ => None,
                    })
                    .map(|item| item.quantity)
                    .next()
                    .unwrap_or(0.0);

                if current < *quantity {
                    return Err(EffectError::InsufficientInventory { good: *good_id, have: current, need: *quantity });
                }

                bs.remove_from_inventory(good_id, *quantity).map_err(|e| EffectError::FinancialSystemError(e))
            }

            StateEffect::RecordTransaction(tx) => {
                state.sim_history.record_transaction(tx.clone());
                Ok(())
            }

            StateEffect::UpdateConsumerIncome { id, new_income } => {
                state
                    .consumers
                    .iter_mut()
                    .find(|c| c.id == *id)
                    .ok_or_else(|| EffectError::AgentNotFound { id: *id })?
                    .income = *new_income;
                Ok(())
            }

            StateEffect::UpdateFirmRevenue { id, revenue } => {
                let tx = Transaction::new(
                    TransactionType::Transfer { from: *id, to: *id, amount: *revenue },
                    InstrumentId(uuid::Uuid::new_v4()),
                    *id,
                    *id,
                    *revenue,
                );
                state.sim_history.record_transaction(tx);
                Ok(())
            }

            StateEffect::Hire { firm, count } => {
                state
                    .firms
                    .iter_mut()
                    .find(|f| f.id == *firm)
                    .ok_or_else(|| EffectError::FirmNotFound { id: *firm })?
                    .hire(*count);
                Ok(())
            }

            StateEffect::Produce { firm, amount, good_id } => {
                state
                    .firms
                    .iter()
                    .find(|f| f.id == *firm)
                    .ok_or_else(|| EffectError::FirmNotFound { id: *firm })?
                    .produce(good_id, *amount as u32);
                Ok(())
            }

            StateEffect::PlaceOrderInBook { market_id, order } => match market_id {
                MarketId::Financial(fin_market_id) => {
                    let market =
                        state.financial_system.exchange.financial_market_mut(fin_market_id).ok_or_else(|| {
                            EffectError::MarketNotFound { market: format!("Financial({:?})", fin_market_id) }
                        })?;

                    match order {
                        Order::Bid(bid) => market.post_bid(bid.agent_id.clone(), bid.quantity, bid.price),
                        Order::Ask(ask) => market.post_ask(ask.agent_id.clone(), ask.quantity, ask.price),
                    }
                    Ok(())
                }

                MarketId::Goods(good_id) => {
                    let market = state
                        .financial_system
                        .exchange
                        .goods_market_mut(good_id)
                        .ok_or_else(|| EffectError::MarketNotFound { market: format!("Goods({:?})", good_id) })?;

                    match order {
                        Order::Bid(bid) => market.post_bid(bid.agent_id.clone(), bid.quantity, bid.price),
                        Order::Ask(ask) => market.post_ask(ask.agent_id.clone(), ask.quantity, ask.price),
                    }
                    Ok(())
                }
                MarketId::Labour(_) => {
                    Err(EffectError::UnimplementedAction("Labour market orders not implemented".to_string()))
                }
            },
        }
    }
    pub fn name(&self) -> String {
        self.to_string()
    }
}

#[derive(Error, Debug, Clone)]
pub enum EffectError {
    #[error("Instrument not found: {id:?}")]
    InstrumentNotFound { id: InstrumentId },
    #[error("Agent not found: {id:?}")]
    AgentNotFound { id: AgentId },
    #[error("Firm not found: {id:?}")]
    FirmNotFound { id: AgentId },
    #[error("Market not found: {market:?}")]
    MarketNotFound { market: String },
    #[error("Insufficient inventory for {good:?}: have {have}, need {need}")]
    InsufficientInventory { good: GoodId, have: f64, need: f64 },
    #[error("Financial system error: {0}")]
    FinancialSystemError(String),
    #[error("Invalid state: {0}")]
    InvalidState(String),
    #[error("Invalid recipe: {id:?}")]
    RecipeError{ id: RecipeId },
    #[error("Unimplemented action: {0}")]
    UnimplementedAction(String),
    #[error("Unhandled action: {0}")]
    Unhandled(String),
    #[error("Bank Tx Failed: Action {0}, reason {1}")]
    TransactionFailure(String, String),
}