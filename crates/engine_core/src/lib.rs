mod effect;
mod decision;
mod action;
mod state;

use effect::*;
use decision::*;
use state::*;
use action::*;


#[derive(Debug, Clone)]
enum SimEffect {
    NoOp,
}

impl Effect for SimEffect {
    fn apply(self, _world: &mut dyn WorldState) -> Result<(), EffectError> {
        match self {
            SimEffect::NoOp => Ok(()),
        }
    }
}