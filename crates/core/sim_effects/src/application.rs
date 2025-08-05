use sim_types::*;
use crate::*;
use thiserror::Error;

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
    RecipeError { id: RecipeId },
    #[error("Unimplemented action: {0}")]
    UnimplementedAction(String),
    #[error("Unhandled action: {0}")]
    Unhandled(String),
    #[error("Bank transaction failed: Action {0}, reason {1}")]
    TransactionFailure(String, String),
}

pub trait EffectApplicator {
    fn apply_effect(&mut self, effect: &StateEffect) -> Result<(), EffectError>;
    fn apply_effects(&mut self, effects: &[StateEffect]) -> Result<(), EffectError> {
        for effect in effects {
            self.apply_effect(effect)?;
        }
        Ok(())
    }
}

pub struct StateEffectApplicator;

impl StateEffectApplicator {
    pub fn apply_to_state(state: &mut SimState, effect: &StateEffect) -> Result<(), EffectError> {
        match effect {
            StateEffect::Financial(financial_effect) => Self::apply_financial_effect(state, financial_effect),
            StateEffect::Inventory(inventory_effect) => Self::apply_inventory_effect(state, inventory_effect),
            StateEffect::Market(market_effect) => Self::apply_market_effect(state, market_effect),
            StateEffect::Agent(agent_effect) => Self::apply_agent_effect(state, agent_effect),
        }
    }

    fn apply_financial_effect(state: &mut SimState, effect: &FinancialEffect) -> Result<(), EffectError> {
        match effect {
            FinancialEffect::CreateInstrument(inst) => state
                .financial_system
                .create_or_consolidate_instrument(inst.clone())
                .map(|_| ())
                .map_err(EffectError::FinancialSystemError),

            FinancialEffect::UpdateInstrument { id, new_principal } => state
                .financial_system
                .update_instrument(id, *new_principal)
                .map_err(|e| EffectError::FinancialSystemError(e)),

            FinancialEffect::TransferInstrument { id, new_creditor } => state
                .financial_system
                .transfer_instrument(id, *new_creditor)
                .map_err(|e| EffectError::FinancialSystemError(e)),

            FinancialEffect::RemoveInstrument(id) => {
                state.financial_system.remove_instrument(id).map_err(|e| EffectError::FinancialSystemError(e))
            }

            FinancialEffect::SwapInstrument { id, new_debtor, new_creditor } => state
                .financial_system
                .swap_instrument(id, new_debtor, new_creditor)
                .map_err(|e| EffectError::FinancialSystemError(e)),

            FinancialEffect::RecordTransaction(tx) => {
                state.history.transactions.push(tx.clone());
                Ok(())
            }
        }
    }

    fn apply_inventory_effect(state: &mut SimState, effect: &InventoryEffect) -> Result<(), EffectError> {
        match effect {
            InventoryEffect::AddInventory { owner, good_id, quantity, unit_cost } => {
                let bs = state.financial_system.balance_sheets.get_mut(owner).ok_or(EffectError::AgentNotFound { id: *owner })?;
                bs.add_to_inventory(good_id, *quantity, *unit_cost);
                Ok(())
            }
            InventoryEffect::RemoveInventory { owner, good_id, quantity } => {
                let bs = state.financial_system.balance_sheets.get_mut(owner).ok_or(EffectError::AgentNotFound { id: *owner })?;
                bs.remove_from_inventory(good_id, *quantity).map_err(EffectError::FinancialSystemError)
            }
        }
    }

    fn apply_market_effect(state: &mut SimState, effect: &MarketEffect) -> Result<(), EffectError> {
        match effect {
            MarketEffect::PlaceOrderInBook { market_id, order } => {
                match market_id {
                    MarketId::Goods(good_id) => {
                        let market = state
                            .financial_system
                            .exchange
                            .goods_market_mut(good_id)
                            .ok_or_else(|| EffectError::MarketNotFound { market: format!("Goods({})", good_id.0) })?;
                        match order {
                            Order::Bid(bid) => market.order_book.bids.push(bid.clone()),
                            Order::Ask(ask) => market.order_book.asks.push(ask.clone()),
                        }
                    }
                    MarketId::Financial(fin_market_id) => {
                        let market = state
                            .financial_system
                            .exchange
                            .financial_market_mut(fin_market_id)
                            .ok_or_else(|| EffectError::MarketNotFound { market: format!("Financial({:?})", fin_market_id) })?;
                        match order {
                            Order::Bid(bid) => market.order_book.bids.push(bid.clone()),
                            Order::Ask(ask) => market.order_book.asks.push(ask.clone()),
                        }
                    }
                    MarketId::Labour(_) => {
                        return Err(EffectError::UnimplementedAction("Labour market not implemented".to_string()))
                    }
                }
                Ok(())
            }
            MarketEffect::ExecuteTrade(trade) => {
                println!("[EFFECT] Executing Trade: {} buys {} of {:?} from {} @ ${}", trade.buyer, trade.quantity, trade.market_id, trade.seller, trade.price);
                Ok(())
            }
            MarketEffect::UpdatePrice { .. } | MarketEffect::ClearMarket { .. } => {
                Ok(())
            }
        }
    }

    fn apply_agent_effect(state: &mut SimState, effect: &AgentEffect) -> Result<(), EffectError> {
        match effect {
            AgentEffect::Hire { firm, count } => {
                if state.agents.get_firm(firm).is_some() {
                    println!("[EFFECT] Firm {} hiring {} agents", firm, count);
                    Ok(())
                } else {
                    Err(EffectError::FirmNotFound { id: *firm })
                }
            }
            AgentEffect::UpdateIncome { id, new_income } => {
                if let Some(consumer) = state.agents.get_consumer_mut(id) {
                    consumer.income = *new_income;
                    Ok(())
                } else {
                    Err(EffectError::AgentNotFound { id: *id })
                }
            }
            AgentEffect::UpdateRevenue { id, revenue } => {
                 let tx = Transaction {
                    id: uuid::Uuid::new_v4(),
                    date: state.ticknum,
                    qty: *revenue,
                    from: *id, // Source is abstract
                    to: *id,
                    tx_type: TransactionType::Transfer { from: *id, to: *id, amount: *revenue },
                    instrument_id: None,
                };
                state.history.transactions.push(tx);
                Ok(())
            }
            AgentEffect::Produce { firm, good_id, amount } => {
                 println!("[EFFECT] Firm {} producing {} of {:?}", firm, amount, good_id);
                 Ok(())
            }
        }
    }
}

impl EffectApplicator for SimState {
    fn apply_effect(&mut self, effect: &StateEffect) -> Result<(), EffectError> {
        StateEffectApplicator::apply_to_state(self, effect)
    }
}