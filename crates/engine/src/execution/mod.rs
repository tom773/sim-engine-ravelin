use shared::*;
use crate::state::SimState;
use uuid::Uuid;
use std::collections::HashMap;

pub mod executor;
pub mod actions;
pub mod effects;
pub mod scheduler;

pub use scheduler::*;
pub use executor::*;
pub use actions::*;
pub use effects::*;

// Lifecycle of a tick:
/* 
    1. Call <agent>.decide() to get their decisions.
    2. Collect all actions via <agent>.act().
    3. Convert each action to sim actions.
    4. Run TransactionExecutor::execute_action() to collect effects of actions.
    5. Run TransactionExecutor::apply_effects() to apply the effects of the actions. 
    6. Update the SimState with the new tick number.
*/