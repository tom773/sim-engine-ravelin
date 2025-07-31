use thiserror::Error;
use super::{WorldState, effect::{AnyEffect, Effect}};
use std::any::Any;

pub trait Action: Send + Sync + 'static {
    type Effect: Effect;
    fn validate(&self, world: &dyn WorldState) -> Result<(), ActionError>;
    fn execute(&self, world: &dyn WorldState) -> Vec<Self::Effect>;
}

#[derive(Error, Debug)]
pub enum ActionError {
    #[error("Action validation failed: {0}")]
    ValidationError(String),
    #[error("Action execution failed: {0}")]
    ExecutionError(String),
}

pub trait AnyAction: Send + Sync {
    fn as_any(&self) -> &dyn Any;

    fn validate(&self, world: &dyn WorldState) -> Result<(), ActionError>;
    fn execute(&self, world: &dyn WorldState) -> Vec<Box<dyn AnyEffect>>;
}