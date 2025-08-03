use crate::{EffectError, SimState, StateEffect};

pub struct TransactionExecutor;

impl TransactionExecutor {
    pub fn apply(effects: &[StateEffect], state: &mut SimState) -> Result<(), EffectError> {
        for effect in effects {
            effect.apply(state)?;
        }
        Ok(())
    }
}