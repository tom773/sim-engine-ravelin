use super::*;
use ravelin_core::*;

pub struct TransactionExecutor;

impl TransactionExecutor {
    pub fn execute(action: &SimAction, state: &mut SimState) -> ExecutionResult {
        state.domain_registry.execute(&action, state)
    }
    pub fn apply(effects: &[StateEffect], state: &mut SimState) -> Result<(), String> {
        for effect in effects {
            effect.apply(state)
                .map_err(|e| format!("Failed to apply {}: {}", effect.name(), e))?;
        }
        Ok(())
    }
}