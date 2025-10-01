
use super::*;
use crate::executor::SimulationEngine;

pub fn register_legacy_handlers_as_phases(scheduler: &mut TickScheduler) {
    register_am_session_handlers(scheduler);

    register_pm_session_handlers(scheduler);

    register_cob_session_handlers(scheduler);
}

fn register_am_session_handlers(scheduler: &mut TickScheduler) {
    use sim_core::types::core_utils::time::Session;

    scheduler.register_phase_handler(
        Session::AM,
        Phase::Ingest,
        CompositePhaseHandler::new(vec![
            Box::new(GovCouponsHandler),
            Box::new(InterbankLoanServicingHandler),
            Box::new(RepoServicingHandler),
        ]),
    );

    scheduler.register_phase_handler(
        Session::AM,
        Phase::Decide,
        GatherIntentionsHandler,
    );

    scheduler.register_phase_handler(
        Session::AM,
        Phase::Submit,
        CompositePhaseHandler::new(vec![
            Box::new(PhaseResolutionHandler {
                phase: domains::ResolutionPhase::Independent,
            }),
            Box::new(PhaseResolutionHandler {
                phase: domains::ResolutionPhase::Market,
            }),
            Box::new(ApplyMarketEffectsHandler),
            Box::new(PhaseResolutionHandler {
                phase: domains::ResolutionPhase::Dependent,
            }),
        ]),
    );

    scheduler.register_phase_handler(
        Session::AM,
        Phase::Match,
        CompositePhaseHandler::new(vec![
            Box::new(DebtAuctionsHandler),
            Box::new(ClearMarketsHandler),
            Box::new(ClearOvernightHandler),
        ]),
    );

    scheduler.register_phase_handler(
        Session::AM,
        Phase::Settle,
        CompositePhaseHandler::new(vec![
            Box::new(SettleTradesHandler),
            Box::new(ApplyPaymentQueuingHandler),
            Box::new(RunRTGSHandler),
        ]),
    );

    scheduler.register_phase_handler(
        Session::AM,
        Phase::MarkRisk,
        NoOpPhaseHandler,
    );

    scheduler.register_phase_handler(
        Session::AM,
        Phase::Report,
        NoOpPhaseHandler,
    );
}

fn register_pm_session_handlers(scheduler: &mut TickScheduler) {
    use sim_core::types::core_utils::time::Session;

    scheduler.register_phase_handler(
        Session::PM,
        Phase::Ingest,
        NoOpPhaseHandler,
    );

    scheduler.register_phase_handler(
        Session::PM,
        Phase::Decide,
        NoOpPhaseHandler,
    );

    scheduler.register_phase_handler(
        Session::PM,
        Phase::Submit,
        NoOpPhaseHandler,
    );
    
    scheduler.register_phase_handler(
        Session::PM,
        Phase::Match,
        NoOpPhaseHandler
    );

    scheduler.register_phase_handler(
        Session::PM,
        Phase::Settle,
        NoOpPhaseHandler
    );

    scheduler.register_phase_handler(
        Session::PM,
        Phase::MarkRisk,
        NoOpPhaseHandler,
    );

    scheduler.register_phase_handler(
        Session::PM,
        Phase::Report,
        NoOpPhaseHandler,
    );
}

fn register_cob_session_handlers(scheduler: &mut TickScheduler) {
    use sim_core::types::core_utils::time::Session;

    scheduler.register_phase_handler(
        Session::EOD,
        Phase::Cutoff,
        NoOpPhaseHandler,
    );

    scheduler.register_phase_handler(
        Session::EOD,
        Phase::PaymentsFinality,
        CompositePhaseHandler::new(vec![
            Box::new(ApplyPaymentQueuingHandler),
            Box::new(RunRTGSHandler),
        ]),
    );

    scheduler.register_phase_handler(
        Session::EOD,
        Phase::ReserveCalc,
        ReserveCalcHandler,
    );

    scheduler.register_phase_handler(
        Session::EOD,
        Phase::CorridorFacilities,
        CorridorFacilitiesHandler,
    );

    scheduler.register_phase_handler(
        Session::EOD,
        Phase::AccrualRoll,
        CompositePhaseHandler::new(vec![
            Box::new(DepositServicingHandler),
            Box::new(CreditServicingHandler),
        ]),
    );

    scheduler.register_phase_handler(
        Session::EOD,
        Phase::Close,
        CompositePhaseHandler::new(vec![
            Box::new(CreditReconciliationHandler),
            Box::new(ApplyAllEffectsHandler),
            Box::new(UpdateHistoryHandler),
            Box::new(BankBalanceSheetSummaryHandler),
            Box::new(UpkeepHandler),
        ]),
    );
}

#[derive(Debug)]
pub struct CompositePhaseHandler {
    handlers: Vec<Box<dyn PhaseHandler + Send + Sync>>,
}

impl CompositePhaseHandler {
    pub fn new(handlers: Vec<Box<dyn PhaseHandler + Send + Sync>>) -> Self {
        Self { handlers }
    }
}

impl PhaseHandler for CompositePhaseHandler {
    fn execute(
        &self,
        engine: &mut SimulationEngine,
        context: &mut PhaseContext,
        rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        let mut total_duration = 0u64;
        let mut combined_telemetry = StepTelemetry::new();

        for handler in &self.handlers {
            let result = handler.execute(engine, context, rng);
            total_duration += result.duration_ms;

            if !result.success {
                return result;
            }

            for metric in result.telemetry.metrics {
                combined_telemetry.metrics.push(metric);
            }
        }

        StepResult::success(total_duration, combined_telemetry)
    }
}

#[derive(Debug)]
pub struct NoOpPhaseHandler;

impl PhaseHandler for NoOpPhaseHandler {
    fn execute(
        &self,
        _engine: &mut SimulationEngine,
        _context: &mut PhaseContext,
        _rng: &mut dyn rand::RngCore,
    ) -> StepResult {
        StepResult::success(0, StepTelemetry::new())
    }
}
