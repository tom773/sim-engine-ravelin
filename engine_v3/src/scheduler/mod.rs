pub mod context;
pub mod dag;
pub mod handlers;
pub mod phase_handler;
pub mod phase_mapping;
pub mod phase_types;
pub mod phases;

pub use context::TickContext;
pub use dag::*;
pub use handlers::*;
pub use phase_handler::*;
pub use phase_mapping::*;
pub use phase_types::*;
pub use phases::*;

#[deprecated(note = "Use TickContext instead")]
pub use context::TickContext as StepContext;

use ahash::AHashMap;
use sim_core::types::core_utils::time::Session;

#[derive(Debug, Default)]
pub struct SchedulerMetrics {
    pub tick_durations: std::collections::VecDeque<std::time::Duration>,
    pub session_durations: AHashMap<Session, std::collections::VecDeque<std::time::Duration>>,
    pub phase_durations: AHashMap<(Session, Phase), std::collections::VecDeque<std::time::Duration>>,
    pub failure_counts: AHashMap<(Session, Phase), u64>,
    pub max_history: usize,
}

impl SchedulerMetrics {
    pub fn new() -> Self {
        Self {
            tick_durations: std::collections::VecDeque::new(),
            session_durations: AHashMap::new(),
            phase_durations: AHashMap::new(),
            failure_counts: AHashMap::new(),
            max_history: 1000,
        }
    }

    pub fn record_tick(&mut self, result: &TickExecutionResult) {
        metrics::histogram!("engine.tick.duration_ms", result.total_duration.as_secs_f64() * 1_000.0);
        self.tick_durations.push_back(result.total_duration);
        if self.tick_durations.len() > self.max_history {
            self.tick_durations.pop_front();
        }

        for session_result in &result.session_results {
            let session = session_result.session;

            self.session_durations.entry(session).or_default().push_back(session_result.duration);
            let session_history = self.session_durations.get_mut(&session).unwrap();
            if session_history.len() > self.max_history {
                session_history.pop_front();
            }

            for (phase, phase_result) in &session_result.phase_results {
                let key = (session, *phase);
                let duration = std::time::Duration::from_millis(phase_result.duration_ms);

                self.phase_durations.entry(key).or_default().push_back(duration);
                let phase_history = self.phase_durations.get_mut(&key).unwrap();
                if phase_history.len() > self.max_history {
                    phase_history.pop_front();
                }

                if !phase_result.success {
                    *self.failure_counts.entry(key).or_insert(0) += 1;
                }

                metrics::histogram!(
                    "engine.phase.duration_ms",
                    phase_result.duration_ms as f64,
                    "session" => format!("{:?}", session),
                    "phase" => format!("{:?}", phase)
                );
            }
        }
    }

    pub fn average_tick_duration(&self) -> std::time::Duration {
        if self.tick_durations.is_empty() {
            return std::time::Duration::ZERO;
        }
        let sum: std::time::Duration = self.tick_durations.iter().sum();
        sum / self.tick_durations.len() as u32
    }

    pub fn print_summary(&self) {
        println!("=== Scheduler Performance Summary ===");
        println!("Average tick duration: {:?}", self.average_tick_duration());
        println!("Total ticks recorded: {}", self.tick_durations.len());

        println!("\nSession Performance:");
        for session in &[Session::AM, Session::PM, Session::EOD] {
            if let Some(durations) = self.session_durations.get(session) {
                if !durations.is_empty() {
                    let sum: std::time::Duration = durations.iter().sum();
                    let avg = sum / durations.len() as u32;
                    println!("  {:?}: avg {:?}, {} runs", session, avg, durations.len());
                }
            }
        }

        println!("\nPhase Performance:");
        let mut phases: Vec<_> = self.phase_durations.keys().collect();
        phases.sort_by_key(|(s, p)| (format!("{:?}", s), format!("{:?}", p)));

        for &(session, phase) in phases {
            let avg_duration = self.average_phase_duration(session, phase);
            let failure_rate = self.failure_rate(session, phase);
            let run_count = self.phase_durations.get(&(session, phase)).map(|d| d.len()).unwrap_or(0);

            println!(
                "  {:?}/{:?}: avg {:?}, {} runs, {:.1}% failure rate",
                session,
                phase,
                avg_duration,
                run_count,
                failure_rate * 100.0
            );
        }
    }

    fn average_phase_duration(&self, session: Session, phase: Phase) -> std::time::Duration {
        if let Some(durations) = self.phase_durations.get(&(session, phase)) {
            if durations.is_empty() {
                return std::time::Duration::ZERO;
            }
            let sum: std::time::Duration = durations.iter().sum();
            sum / durations.len() as u32
        } else {
            std::time::Duration::ZERO
        }
    }

    fn failure_rate(&self, session: Session, phase: Phase) -> f64 {
        let failures = *self.failure_counts.get(&(session, phase)).unwrap_or(&0) as f64;
        let total_runs = self.phase_durations.get(&(session, phase)).map(|d| d.len()).unwrap_or(0) as f64;
        if total_runs > 0.0 { failures / total_runs } else { 0.0 }
    }
}
