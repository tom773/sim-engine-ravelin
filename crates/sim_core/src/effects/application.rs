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
        let mut errors = Vec::new();
        
        for (index, effect) in effects.iter().enumerate() {
            match self.apply_effect(effect) {
                Ok(()) => {
                }
                Err(e) => {
                    errors.push(format!("Effect {}: {}", index, e));
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(EffectError::InvalidState(errors.join("; ")))
        }
    }
}


pub struct StateEffectApplicator;

impl StateEffectApplicator {
    pub fn apply_to_state(state: &mut SimState, effect: &StateEffect) -> Result<(), EffectError> {
        match effect {
            StateEffect::Financial(financial_effect) => { Self::apply_financial_effect(state, financial_effect) }
            StateEffect::Inventory(inventory_effect) => Self::apply_inventory_effect(state, inventory_effect),
            StateEffect::Market(market_effect) => Self::apply_market_effect(state, market_effect),
            StateEffect::Agent(agent_effect) => Self::apply_agent_effect(state, agent_effect),
        }
    }

    fn apply_inventory_effect(state: &mut SimState, effect: &InventoryEffect) -> Result<(), EffectError> {
        match effect {
            InventoryEffect::AddInventory { owner, good_id, quantity, unit_cost } => {
                let bs = state
                    .financial_system
                    .balance_sheets
                    .get_mut(owner)
                    .ok_or(EffectError::AgentNotFound { id: *owner })?;
                bs.add_to_inventory(good_id, *quantity, *unit_cost);
                Ok(())
            }
            InventoryEffect::RemoveInventory { owner, good_id, quantity } => {
                let bs = state
                    .financial_system
                    .balance_sheets
                    .get_mut(owner)
                    .ok_or(EffectError::AgentNotFound { id: *owner })?;
                bs.remove_from_inventory(good_id, *quantity).map_err(EffectError::FinancialSystemError)
            }
        }
    }

    fn apply_market_effect(state: &mut SimState, effect: &MarketEffect) -> Result<(), EffectError> {
        match effect {
            MarketEffect::PlaceOrderInBook { market_id, order } => {
                let order_book = match market_id {
                    MarketId::Goods(id) => {
                        state.financial_system.exchange.goods_market_mut(id).map(|m| &mut m.order_book)
                    }
                    MarketId::Financial(id) => {
                        state.financial_system.exchange.financial_market_mut(id).map(|m| &mut m.order_book)
                    }
                    MarketId::Labour(_) => {
                        return Err(EffectError::InvalidState(
                            "Cannot place direct orders in a labour market.".to_string(),
                        ));
                    }
                }
                .ok_or_else(|| EffectError::MarketNotFound { market: format!("{:?}", market_id) })?;

                match order {
                    Order::Bid(bid) => order_book.bids.push(bid.clone()),
                    Order::Ask(ask) => order_book.asks.push(ask.clone()),
                }
                Ok(())
            }
            MarketEffect::ExecuteTrade(_trade) => {
                Ok(())
            }
            MarketEffect::UpdatePrice { market_id, new_price } => {
                if let MarketId::Financial(fin_market_id) = market_id {
                    let daily_rate = fin_market_id.price_to_daily_rate(*new_price);
                    let annual_rate = (1.0 + daily_rate).powf(365.0) - 1.0;
                    println!(
                        "[EFFECT] Market {:?} price updated. New Price: {:.2}, Daily Rate: {:.6}, Annual Rate: {:.4}%",
                        market_id,
                        new_price,
                        daily_rate,
                        annual_rate * 100.0
                    );
                    Ok(())
                } else {
                    Err(EffectError::InvalidState(
                        "UpdatePrice effect is only valid for non-financial markets.".to_string(),
                    ))
                }
            }
            MarketEffect::ClearMarket { market_id } => {
                let order_book = match market_id {
                    MarketId::Goods(id) => {
                        state.financial_system.exchange.goods_market_mut(id).map(|m| &mut m.order_book)
                    }
                    MarketId::Financial(id) => {
                        state.financial_system.exchange.financial_market_mut(id).map(|m| &mut m.order_book)
                    }
                    MarketId::Labour(_) => {
                        return Err(EffectError::InvalidState(
                            "ClearMarket is not applicable to labour markets.".to_string(),
                        ));
                    }
                }
                .ok_or_else(|| EffectError::MarketNotFound { market: format!("{:?}", market_id) })?;

                order_book.bids.clear();
                order_book.asks.clear();
                println!("[EFFECT] Cleared order book for market: {:?}", market_id);
                Ok(())
            }
            MarketEffect::UpdateLabourMarket { market_id, update } => {
                let market = state
                    .financial_system
                    .exchange
                    .labour_market_mut(market_id)
                    .ok_or_else(|| EffectError::MarketNotFound { market: format!("{:?}", market_id) })?;
                match update {
                    LabourMarketUpdate::AddApplication(app) => market.job_applications.push(app.clone()),
                    LabourMarketUpdate::AddOffer(offer) => market.job_offers.push(offer.clone()),
                }
                Ok(())
            }
            MarketEffect::ClearLabourMarketOrders { market_id, filled_applications } => {
                let market = state
                    .financial_system
                    .exchange
                    .labour_market_mut(market_id)
                    .ok_or_else(|| EffectError::MarketNotFound { market: format!("{:?}", market_id) })?;
                let filled_ids: std::collections::HashSet<_> = filled_applications.iter().collect();
                market.job_applications.retain(|app| !filled_ids.contains(&&app.application_id));
                Ok(())
            }
        }
    }

    fn apply_agent_effect(state: &mut SimState, effect: &AgentEffect) -> Result<(), EffectError> {
        match effect {
            AgentEffect::UpdateRevenue { id, revenue } => {
                let tx = Transaction {
                    id: uuid::Uuid::new_v4(),
                    date: state.ticknum,
                    qty: *revenue,
                    from: *id,
                    to: *id,
                    tx_type: TransactionType::Transfer { from: *id, to: *id, amount: *revenue },
                    instrument_id: None,
                };
                state.history.transactions.push(tx);
                Ok(())
            }
            AgentEffect::Produce { firm: _, good_id: _, amount: _ } => {
                Ok(())
            }
            AgentEffect::EstablishEmployment { firm_id, consumer_id, contract } => {
                let firm = state.agents.firms.get_mut(firm_id);
                let consumer = state.agents.consumers.get_mut(consumer_id);

                match (firm, consumer) {
                    (Some(firm), Some(consumer)) => {
                        firm.employees.insert(*consumer_id, contract.clone());
                        consumer.employed_by = Some(*firm_id);
                        consumer.hours_worked = contract.hours;
                        consumer.income = contract.wage_rate * contract.hours;
                        Ok(())
                    }
                    (None, _) => Err(EffectError::AgentNotFound { id: *firm_id }),
                    (_, None) => Err(EffectError::AgentNotFound { id: *consumer_id }),
                }
            }
            AgentEffect::TerminateEmployment { firm_id, consumer_id } => {
                let firm = state.agents.firms.get_mut(firm_id);
                let consumer = state.agents.consumers.get_mut(consumer_id);

                match (firm, consumer) {
                    (Some(firm), Some(consumer)) => {
                        if firm.employees.contains_key(consumer_id) && consumer.employed_by == Some(*firm_id) {
                            firm.employees.remove(consumer_id);
                            consumer.employed_by = None;
                            consumer.income = 0.0;
                            consumer.hours_worked = 0.0;
                            Ok(())
                        } else {
                            Err(EffectError::InvalidState(format!(
                                "Employment relationship mismatch for termination between firm {} and consumer {}.",
                                firm_id, consumer_id
                            )))
                        }
                    }
                    (None, _) => Err(EffectError::AgentNotFound { id: *firm_id }),
                    (_, None) => Err(EffectError::AgentNotFound { id: *consumer_id }),
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
            AgentEffect::RecordDividendIncome { recipient, amount } => {
                if let Some(consumer) = state.agents.get_consumer_mut(recipient) {
                    consumer.income += *amount;
                    Ok(())
                } else if let Some(_firm) = state.agents.get_firm_mut(recipient) {
                    Ok(())
                } else {
                    Err(EffectError::AgentNotFound { id: *recipient })
                }
            }
        }
    }
}

impl EffectApplicator for SimState {
    fn apply_effect(&mut self, effect: &StateEffect) -> Result<(), EffectError> {
        StateEffectApplicator::apply_to_state(self, effect)
    }
}
