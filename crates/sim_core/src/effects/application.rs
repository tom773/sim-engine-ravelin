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
            FinancialEffect::SplitAndTransferInstrument { id, buyer, quantity } => {
                state.financial_system
                    .split_and_transfer_instrument(id, *buyer, *quantity)
                    .map(|_| ())
                    .map_err( EffectError::FinancialSystemError)
            }
            FinancialEffect::SwapInstrument { id, new_debtor, new_creditor } => state
                .financial_system
                .swap_instrument(id, new_debtor, new_creditor)
                .map_err(|e| EffectError::FinancialSystemError(e)),

            FinancialEffect::RecordTransaction(tx) => {
                state.history.transactions.push(tx.clone());
                Ok(())
            }
            FinancialEffect::AccrueInterest { instrument_id, accrued_amount, accrual_date } => {
                if let Some(instrument) = state.financial_system.instruments.get_mut(instrument_id) {
                    instrument.accrued_interest += *accrued_amount;
                    instrument.last_accrual_date = *accrual_date;

                    if let Some(creditor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.creditor) {
                        if let Some(asset) = creditor_bs.assets.get_mut(instrument_id) {
                             asset.accrued_interest += *accrued_amount;
                             asset.last_accrual_date = *accrual_date;
                        }
                    }
                     if let Some(debtor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.debtor) {
                        if let Some(liability) = debtor_bs.liabilities.get_mut(instrument_id) {
                             liability.accrued_interest += *accrued_amount;
                             liability.last_accrual_date = *accrual_date;
                        }
                    }

                    Ok(())
                } else {
                     Err(EffectError::InstrumentNotFound { id: *instrument_id })
                }
            }
            FinancialEffect::ResetAccruedInterest { instrument_id } => {
                if let Some(instrument) = state.financial_system.instruments.get_mut(instrument_id) {
                    instrument.accrued_interest = 0.0;
                    if let Some(creditor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.creditor) {
                        if let Some(asset) = creditor_bs.assets.get_mut(instrument_id) {
                             asset.accrued_interest = 0.0;
                        }
                    }
                     if let Some(debtor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.debtor) {
                        if let Some(liability) = debtor_bs.liabilities.get_mut(instrument_id) {
                             liability.accrued_interest = 0.0;
                        }
                    }
                    Ok(())
                } else {
                     Err(EffectError::InstrumentNotFound { id: *instrument_id })
                }
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
            AgentEffect::RecordDividendIncome { recipient, amount } => {
                println!("[EFFECT] Agent {} receiving dividend income of {}", recipient, amount);
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



#[cfg(test)]
mod eff_tests {
    use super::*;
    use uuid::Uuid;

    fn setup_test_state() -> (SimState, AgentId, AgentId, AgentId) {
        let mut state = SimState::default();
        let agent_a = AgentId(Uuid::new_v4());
        let agent_b = AgentId(Uuid::new_v4());
        let agent_c = AgentId(Uuid::new_v4());

        let mut consumer_a = Consumer::new(30, AgentId::default(), PersonalityArchetype::Balanced);
        consumer_a.id = agent_a; // Manually set ID to match our test agent
        consumer_a.income = 50000.0;
        state.agents.consumers.insert(agent_a, consumer_a);

        state.financial_system.balance_sheets.insert(agent_a, BalanceSheet::new(agent_a));
        state.financial_system.balance_sheets.insert(agent_b, BalanceSheet::new(agent_b));
        state.financial_system.balance_sheets.insert(agent_c, BalanceSheet::new(agent_c));

        (state, agent_a, agent_b, agent_c)
    }


    #[test]
    fn test_apply_create_instrument() {
        let (mut state, agent_a, _, _) = setup_test_state();
        let cb_id = state.financial_system.central_bank.id;
        let cash_instrument = cash!(agent_a, 1000.0, cb_id, state.current_date);
        let effect = StateEffect::Financial(FinancialEffect::CreateInstrument(cash_instrument.clone()));

        let result = StateEffectApplicator::apply_to_state(&mut state, &effect);
        assert!(result.is_ok());

        assert!(state.financial_system.instruments.contains_key(&cash_instrument.id));

        let creditor_bs = state.financial_system.get_bs_by_id(&agent_a).unwrap();
        assert_eq!(creditor_bs.assets.get(&cash_instrument.id).unwrap().principal, 1000.0);

        let debtor_bs = state.financial_system.get_bs_by_id(&cb_id).unwrap();
        assert!(debtor_bs.liabilities.contains_key(&cash_instrument.id));
    }

    #[test]
    fn test_apply_update_instrument() {
        let (mut state, agent_a, agent_b, _) = setup_test_state();
        let instrument = deposit!(agent_a, agent_b, 500.0, 0.01, state.current_date);
        state.financial_system.create_instrument(instrument.clone()).unwrap();

        let effect = StateEffect::Financial(FinancialEffect::UpdateInstrument { id: instrument.id, new_principal: 350.0 });
        
        StateEffectApplicator::apply_to_state(&mut state, &effect).unwrap();

        assert_eq!(state.financial_system.instruments.get(&instrument.id).unwrap().principal, 350.0);
        assert_eq!(state.financial_system.get_bs_by_id(&agent_a).unwrap().assets.get(&instrument.id).unwrap().principal, 350.0);
        assert_eq!(state.financial_system.get_bs_by_id(&agent_b).unwrap().liabilities.get(&instrument.id).unwrap().principal, 350.0);
    }

    #[test]
    fn test_apply_transfer_instrument() {
        let (mut state, agent_a, agent_b, agent_c) = setup_test_state();
        let instrument = deposit!(agent_a, agent_b, 500.0, 0.01, state.current_date);
        state.financial_system.create_instrument(instrument.clone()).unwrap();

        let effect = StateEffect::Financial(FinancialEffect::TransferInstrument { id: instrument.id, new_creditor: agent_c });

        StateEffectApplicator::apply_to_state(&mut state, &effect).unwrap();
        
        assert_eq!(state.financial_system.instruments.get(&instrument.id).unwrap().creditor, agent_c);

        assert!(!state.financial_system.get_bs_by_id(&agent_a).unwrap().assets.contains_key(&instrument.id));
        assert!(state.financial_system.get_bs_by_id(&agent_c).unwrap().assets.contains_key(&instrument.id));
        assert!(state.financial_system.get_bs_by_id(&agent_b).unwrap().liabilities.contains_key(&instrument.id));
    }
    #[test]
    fn test_apply_add_and_remove_inventory() {
        let (mut state, agent_a, _, _) = setup_test_state();
        let oil_id = good_id!("oil");

        let add_effect = StateEffect::Inventory(InventoryEffect::AddInventory {
            owner: agent_a, good_id: oil_id, quantity: 100.0, unit_cost: 50.0,
        });
        StateEffectApplicator::apply_to_state(&mut state, &add_effect).unwrap();

        let bs = state.financial_system.get_bs_by_id(&agent_a).unwrap();
        let inventory = bs.get_inventory().unwrap();
        assert_eq!(inventory.get(&oil_id).unwrap().quantity, 100.0);
        
        let inv_asset_value = bs.real_assets.values().find(|a| matches!(a.asset_type, RealAssetType::Inventory{..})).unwrap().market_value;
        assert_eq!(inv_asset_value, 5000.0);

        let remove_effect = StateEffect::Inventory(InventoryEffect::RemoveInventory {
            owner: agent_a, good_id: oil_id, quantity: 30.0,
        });
        StateEffectApplicator::apply_to_state(&mut state, &remove_effect).unwrap();
        
        let bs_after_removal = state.financial_system.get_bs_by_id(&agent_a).unwrap();
        let inventory_after_removal = bs_after_removal.get_inventory().unwrap();
        assert_eq!(inventory_after_removal.get(&oil_id).unwrap().quantity, 70.0);

        let remove_too_much = StateEffect::Inventory(InventoryEffect::RemoveInventory {
            owner: agent_a, good_id: oil_id, quantity: 100.0, // Only 70 left
        });
        let result = StateEffectApplicator::apply_to_state(&mut state, &remove_too_much);
        assert!(result.is_err());
    }


    #[test]
    fn test_apply_place_order_in_book() {
        let (mut state, agent_a, _, _) = setup_test_state();
        let petrol_id = good_id!("petrol");
        let market_id = MarketId::Goods(petrol_id);
        
        state.financial_system.exchange.register_goods_market(petrol_id, &goods::CATALOGUE);

        let bid = Order::Bid(Bid { agent_id: agent_a, price: 10.0, quantity: 5.0 });
        let effect = StateEffect::Market(MarketEffect::PlaceOrderInBook { market_id, order: bid });
        
        StateEffectApplicator::apply_to_state(&mut state, &effect).unwrap();
        
        let market = state.financial_system.exchange.goods_market(&petrol_id).unwrap();
        assert_eq!(market.order_book.bids.len(), 1);
        assert_eq!(market.order_book.bids[0].price, 10.0);
    }
    

    #[test]
    fn test_apply_update_income() {
        let (mut state, agent_a, _, _) = setup_test_state();
        assert_eq!(state.agents.get_consumer(&agent_a).unwrap().income, 50000.0);

        let effect = StateEffect::Agent(AgentEffect::UpdateIncome { id: agent_a, new_income: 95000.0 });
        
        StateEffectApplicator::apply_to_state(&mut state, &effect).unwrap();
        
        let consumer = state.agents.get_consumer(&agent_a).unwrap();
        assert_eq!(consumer.income, 95000.0);
    }
}