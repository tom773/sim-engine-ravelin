use thiserror::Error;
use super::state::WorldState;

pub trait Effect: Send + Sync + 'static {
    fn apply(self, world: &mut dyn WorldState) -> Result<(), EffectError>;
}

#[derive(Error, Debug)]
pub enum EffectError {
    #[error("Effect application failed: {0}")]
    ApplicationError(String),
    #[error("Invalid effect state: {0}")]
    InvalidState(String),
}

pub trait AnyEffect: Send + Sync {
    fn apply(self: Box<Self>, world: &mut dyn WorldState) -> Result<(), EffectError>;
}

impl<T> AnyEffect for T
where
    T: Effect,
{
    fn apply(self: Box<Self>, world: &mut dyn WorldState) -> Result<(), EffectError> {
        Effect::apply(*self, world)
    }
}