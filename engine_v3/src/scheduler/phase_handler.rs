use super::{Phase, StepResult, TickContext};
use crate::executor::SimulationEngine;
use sim_core::types::core_utils::time::Session;
use std::fmt::Debug;

#[derive(Debug)]
pub struct PhaseContext<'a> {
    pub session: Session,
    pub phase: Phase,
    pub tick_number: u32,
    pub tick_context: &'a mut TickContext,
}

impl<'a> PhaseContext<'a> {
    pub fn new(
        session: Session,
        phase: Phase,
        tick_number: u32,
        tick_context: &'a mut TickContext,
    ) -> Self {
        Self {
            session,
            phase,
            tick_number,
            tick_context,
        }
    }
}

pub trait PhaseHandler: Send + Sync + Debug {
    fn execute(
        &self,
        engine: &mut SimulationEngine,
        context: &mut PhaseContext,
        rng: &mut dyn rand::RngCore,
    ) -> StepResult;
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

