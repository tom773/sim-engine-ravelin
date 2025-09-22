pub mod context;
pub mod dag;
pub mod handler;
pub mod steps;

pub use context::*;
pub use dag::*;
pub use handler::*;
pub use steps::*;

use ahash::AHashMap;

#[derive(Debug, Default)]
pub struct SchedulerMetrics {
    pub tick_durations: std::collections::VecDeque<std::time::Duration>,
    pub step_durations: AHashMap<TickStep, std::collections::VecDeque<std::time::Duration>>,
    pub failure_counts: AHashMap<TickStep, u64>,
    pub max_history: usize,
}

impl SchedulerMetrics {
    pub fn new() -> Self {
        Self {
            tick_durations: std::collections::VecDeque::new(),
            step_durations: AHashMap::new(),
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

        for (step, step_result) in &result.step_results {
            let duration = std::time::Duration::from_millis(step_result.duration_ms);
            self.step_durations.entry(*step).or_default().push_back(duration);

            let step_history = self.step_durations.get_mut(step).unwrap();
            if step_history.len() > self.max_history {
                step_history.pop_front();
            }

            if !step_result.success {
                *self.failure_counts.entry(*step).or_insert(0) += 1;
            }

            step_result.record_metrics(*step);
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

        println!("\nStep Performance:");
        let mut steps: Vec<_> = self.step_durations.keys().collect();
        steps.sort_by_key(|s| format!("{:?}", s));

        for step in steps {
            let avg_duration = self.average_step_duration(*step);
            let _failure_rate = self.failure_rate(*step);
            let run_count = self.step_durations.get(step).map(|d| d.len()).unwrap_or(0);

            println!(
                "  {:?}: avg {:?}, {} runs, {:.1}% failure rate",
                step,
                avg_duration,
                run_count,
                self.failure_rate(*step) * 100.0
            );
        }
    }

    fn average_step_duration(&self, step: TickStep) -> std::time::Duration {
        if let Some(durations) = self.step_durations.get(&step) {
            if durations.is_empty() {
                return std::time::Duration::ZERO;
            }
            let sum: std::time::Duration = durations.iter().sum();
            sum / durations.len() as u32
        } else {
            std::time::Duration::ZERO
        }
    }

    fn failure_rate(&self, step: TickStep) -> f64 {
        let failures = *self.failure_counts.get(&step).unwrap_or(&0) as f64;
        let total_runs = self.step_durations.get(&step).map(|d| d.len()).unwrap_or(0) as f64;
        if total_runs > 0.0 { failures / total_runs } else { 0.0 }
    }
}
