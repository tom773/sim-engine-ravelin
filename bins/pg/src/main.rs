use engine_v3::DomainRegistry;
use rust_decimal_macros::dec;
use sim_core::prelude::*;
use uuid::Uuid;

mod log;
use log::*;

fn main() {
    tracing_subscriber::fmt::init();
    println!("--- playground binary starting ---");

    let mut state = setup();

    let before_state = state.clone();

    let consumer_ids: Vec<AgentId> = state.agents.consumers.keys().cloned().collect();
    let consumer_1_id = consumer_ids[0];
    let consumer_2_id = consumer_ids[1];

    test_transfer(&mut state, consumer_1_id, consumer_2_id, 750.0);

    log_balance_sheet_changes(&before_state, &state);

    println!("\n--- playground finished ---");
}

fn test_transfer(state: &mut SimState, from: AgentId, to: AgentId, amount: f64) {
    println!();
    println!("{:=<80}", "");
    println!("{:.^80}", format!(" TESTING TRANSFER: ${amount} from {from} to {to} "));
    println!("{:=<80}\n", "");

    let domain_registry = DomainRegistry::new();
    let action = SimAction::Banking(BankingAction::InitiatePayment {
        from,
        to,
        amount,
        context: TransactionContext::GenericTransfer,
    });

    let effects = domain_registry.execute_action(&action, state).expect("Action execution failed");

    state.apply_effects(&effects).expect("Failed to apply effects");
    let _ = run_rtgs(state).map_err(|e| format!("RTGS execution failed: {:?}", e));

}
