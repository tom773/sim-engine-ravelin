#[allow(unused)]
pub mod execution;
pub use execution::*;
pub mod state;
mod test;
use rand::prelude::*;
pub use shared::*;
pub use state::*;
use chrono::Duration;

pub fn tick(sim_state: &mut SimState) -> (&mut SimState, Vec<SimAction>, Vec<StateEffect>) {
    sim_state.ticknum += 1;
    sim_state.current_date += Duration::days(1);

    let mut rng = StdRng::from_os_rng();

    let mut all_sim_actions = Vec::new();

    let banks: Vec<Bank> = sim_state.financial_system.commercial_banks.values().cloned().collect();
    for bank in &banks {
        let decisions = bank.decide(&sim_state.financial_system, &mut rng);
        let actions = bank.act(&decisions);
        all_sim_actions.extend(actions);
    }
    for firm in &sim_state.firms {
        let decisions = firm.decide(&sim_state.financial_system, &mut rng);
        let actions = firm.act(&decisions);
        all_sim_actions.extend(actions);
    }

    for consumer in &sim_state.consumers {
        let decisions = consumer.decide(&sim_state.financial_system, &mut rng);
        let actions = consumer.act(&decisions);
        all_sim_actions.extend(actions);
    }

    let mut all_effects = Vec::new();
    for action in &all_sim_actions {
        let result = TransactionExecutor::execute(action, sim_state);
        if result.success {
            all_effects.extend(result.effects);
        } else {
            for error in result.errors {
                println!("Action failed: {} - {}", action.name(), error);
            }
        }
    }

    if let Err(e) = TransactionExecutor::apply(&all_effects, sim_state) {
        println!("Error applying initial effects: {}", e);
    }

    let trades = sim_state.financial_system.exchange.clear_markets();
    let mut settlement_effects = Vec::new();

    for trade in &trades {
        let total_value = trade.quantity * trade.price;

        let payment_action = SimAction::Transfer {
            agent_id: trade.buyer.clone(),
            from: trade.buyer.clone(),
            to: trade.seller.clone(),
            amount: total_value,
        };

        let payment_result = TransactionExecutor::execute(&payment_action, sim_state);

        if payment_result.success {
            settlement_effects.extend(payment_result.effects);

            match &trade.market_id {
                MarketId::Goods(good_id) => {
                    settlement_effects.push(StateEffect::RemoveInventory {
                        owner: trade.seller.clone(),
                        good_id: *good_id,
                        quantity: trade.quantity,
                    });
                    settlement_effects.push(StateEffect::AddInventory {
                        owner: trade.buyer.clone(),
                        good_id: *good_id,
                        quantity: trade.quantity,
                        unit_cost: trade.price,
                    });
                }
                MarketId::Financial(_) => {}
                MarketId::Labour(_) => {}
            }
        } else {
            println!("Trade settlement failed for trade: {:?}", trade);
        }
    }

    if let Err(e) = TransactionExecutor::apply(&settlement_effects, sim_state) {
        println!("Error applying settlement effects: {}", e);
    }

    all_effects.extend(settlement_effects);

    (sim_state, all_sim_actions.clone(), all_effects.clone())
}

pub fn inject_liquidity(ss: &mut SimState) -> &mut SimState {
    let action = SimAction::InjectLiquidity;
    let ns = TransactionExecutor::execute(&action, ss);
    if let Err(e) = TransactionExecutor::apply(&ns.effects, ss) {
        println!("Error applying effects: {}", e);
    }
    ss
}
