use super::*;
use crate::executor::SimulationEngine;
use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use sim_core::types::core_utils::time::Session;
use web_time::Instant;

#[derive(Debug)]
pub struct TickScheduler {
    phase_handlers: AHashMap<(Session, Phase), Box<dyn PhaseHandler + Send + Sync>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TickExecutionResult {
    pub tick_number: u32,
    pub total_duration: std::time::Duration,
    pub session_results: Vec<SessionExecutionResult>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExecutionResult {
    pub session: Session,
    pub phase_results: Vec<(Phase, StepResult)>,
    pub duration: std::time::Duration,
}

impl TickScheduler {
    pub fn new() -> Self {
        Self {
            phase_handlers: AHashMap::new(),
        }
    }

    pub fn register_phase_handler<H: PhaseHandler + Send + Sync + 'static>(
        &mut self,
        session: Session,
        phase: Phase,
        handler: H,
    ) {
        self.phase_handlers.insert((session, phase), Box::new(handler));
    }

    pub fn execute_tick(&self, engine: &mut SimulationEngine, rng: &mut dyn rand::RngCore) -> TickExecutionResult {
        self.execute_session_based_tick(engine, rng)
    }

    pub fn execute_session(
        &self,
        session: Session,
        engine: &mut SimulationEngine,
        tick_context: &mut TickContext,
        rng: &mut dyn rand::RngCore,
    ) -> Vec<(Phase, StepResult)> {
        let session_plan = SessionPlan::new(session);
        let mut phase_results = Vec::new();

        for phase in session_plan.phases {
            if let Some(handler) = self.phase_handlers.get(&(session, phase)) {
                let mut phase_context = PhaseContext::new(session, phase, engine.state.ticknum, tick_context);
                let result = handler.execute(engine, &mut phase_context, rng);
                phase_results.push((phase, result));
            }
        }

        phase_results
    }

    fn execute_session_based_tick(
        &self,
        engine: &mut SimulationEngine,
        rng: &mut dyn rand::RngCore,
    ) -> TickExecutionResult {
        let start_time = Instant::now();
        let mut tick_context = TickContext::new(engine.state.ticknum);
        let mut session_results = Vec::new();
        let mut overall_success = true;

        engine.state.financial_system.rtgs.settled.clear();
        engine.state.financial_system.rtgs.rejected.clear();

        for session_plan in SessionPlan::full_day() {
            let session = session_plan.session;
            let session_start = Instant::now();
            engine.state.current_session = session;

            let mut phase_results = Vec::new();

            for phase in session_plan.phases {
                if let Some(handler) = self.phase_handlers.get(&(session, phase)) {
                    let mut phase_context = PhaseContext::new(session, phase, engine.state.ticknum, &mut tick_context);
                    let result = handler.execute(engine, &mut phase_context, rng);

                    if !result.success {
                        let error = result.error.clone().unwrap_or("Unknown error".to_string());
                        tracing::warn!(
                            session = ?session,
                            phase = ?phase,
                            error = %error,
                            "phase failed"
                        );
                        overall_success = false;
                    }

                    phase_results.push((phase, result.clone()));

                    if !result.success {
                        break;
                    }
                }
            }

            let session_duration = session_start.elapsed();
            session_results.push(SessionExecutionResult {
                session,
                phase_results,
                duration: session_duration,
            });

            if !overall_success {
                break;
            }
        }

        let total_duration = start_time.elapsed();
        tracing::info!("{}", engine.state.current_date);
        TickExecutionResult {
            tick_number: tick_context.tick_number,
            total_duration,
            session_results,
            success: overall_success,
        }
    }
}

impl Default for TickScheduler {
    fn default() -> Self {
        Self::new()
    }
}
