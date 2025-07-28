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
                // Financial System Effects
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
                // TODO: Record Transactions
                StateEffect::RecordTransaction(tx) => {
                    state.sim_history.record_transaction(tx.clone());
                }
                // TODO: Firm Effects
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
                // TODO: Consumer Effects

                _ => {
                    return Err(format!("Effect {:?} is not implemented", effect.name()));
                }
            }
        }
        Ok(())
    }
}