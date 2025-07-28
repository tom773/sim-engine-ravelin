use super::*;
use shared::*;

pub struct TransactionExecutor;

impl TransactionExecutor {
    pub fn execute_action(action: &SimAction, state: &SimState) -> ExecutionResult {
        let registry = DomainRegistry::new();
        registry.execute(action, state)
    }
    pub fn apply_effects(effects: &[StateEffect], state: &mut SimState) -> Result<(), String> {
        for effect in effects {
            match effect {
                StateEffect::CreateInstrument(inst) => {
                    state.financial_system.create_or_consolidate_instrument(inst.clone())?;
                }
                StateEffect::UpdateInstrument { id, new_principal } => {
                    state.financial_system.update_instrument(id, *new_principal)?;
                }
                StateEffect::RemoveInstrument(id) => {
                    state.financial_system.remove_instrument(id)?;
                }
                StateEffect::SwapInstrument { id, new_debtor, new_creditor } => {
                    state.financial_system.swap_instrument(id, new_debtor, new_creditor)?;
                }
                StateEffect::AddInventory { owner, good_id, quantity, unit_cost } => {
                    if let Some(bs) = state.financial_system.balance_sheets.get_mut(owner) {
                        bs.add_to_inventory(good_id, *quantity, *unit_cost);
                    } else {
                        return Err(format!("Owner {} not found for AddInventory", owner.0));
                    }
                }
                StateEffect::RemoveInventory { owner, good_id, quantity } => {
                    if let Some(bs) = state.financial_system.balance_sheets.get_mut(owner) {
                        bs.remove_from_inventory(good_id, *quantity)?;
                    } else {
                        return Err(format!("Owner {} not found for RemoveInventory", owner.0));
                    }
                }
                StateEffect::RecordTransaction(tx) => {
                    state.sim_history.record_transaction(tx.clone());
                }
                StateEffect::Hire { firm, count } => {
                    if let Some(f) = state.firms.iter_mut().find(|f| f.id == *firm) {
                        f.hire(*count);
                    } else {
                        return Err(format!("Firm {} not found", firm.0));
                    }
                }
                StateEffect::Produce { firm, amount, good_id } => {
                    if let Some(f) = state.firms.iter().find(|f| f.id == *firm) {
                        f.produce(good_id, *amount as u32);
                    } else {
                        return Err(format!("Firm {} not found", firm.0));
                    }
                }
                StateEffect::PlaceOrderInBook { market_id, order } => match market_id {
                    MarketId::Financial(fin_market_id) => {
                        if let Some(market) = state.financial_system.exchange.financial_market_mut(fin_market_id) {
                            match order {
                                Order::Bid(bid) => market.post_bid(bid.agent_id.clone(), bid.quantity, bid.price),
                                Order::Ask(ask) => market.post_ask(ask.agent_id.clone(), ask.quantity, ask.price),
                            }
                        } else {
                            return Err(format!("Financial market not found: {:?}", fin_market_id));
                        }
                    }
                    MarketId::Goods(good_id) => {
                        if let Some(market) = state.financial_system.exchange.goods_market_mut(good_id) {
                            match order {
                                Order::Bid(bid) => market.post_bid(bid.agent_id.clone(), bid.quantity, bid.price),
                                Order::Ask(ask) => market.post_ask(ask.agent_id.clone(), ask.quantity, ask.price),
                            }
                        } else {
                            return Err(format!("Goods market not found: {:?}", good_id));
                        }
                    }
                },
                _ => {
                    return Err(format!("Effect {:?} is not implemented", effect.name()));
                }
            }
        }
        Ok(())
    }
}
