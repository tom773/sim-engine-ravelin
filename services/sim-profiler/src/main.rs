use engine_v3::scenario::Scenario;
use engine_v3::scheduler::TickStep;
use rand::{SeedableRng, rngs::StdRng};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let opts = CliOptions::from_args(env::args().skip(1));
    info!(
        ?opts.scenario_path,
        ticks = opts.ticks,
        tick_logging = opts.tick_logging,
        "starting profiler run"
    );

    let scenario_src = fs::read_to_string(&opts.scenario_path)?;
    let scenario = Scenario::from_toml_str(&scenario_src)
        .map_err(|err| format!("failed to parse scenario {:?}: {err}", opts.scenario_path))?;

    let mut engine = scenario.initialize_engine();
    engine.set_tick_logging(opts.tick_logging);

    let mut rng = StdRng::seed_from_u64(scenario.config.seed);

    for tick in 0..opts.ticks {
        let (result, _events) = engine.run_tick(&mut rng);
        let total_ms = result.total_duration.as_secs_f64() * 1_000.0;

        let mut phases: Vec<(TickStep, u64)> =
            result.step_results.iter().map(|(step, step_result)| (*step, step_result.duration_ms)).collect();
        phases.sort_by(|a, b| b.1.cmp(&a.1));

        let phase_summary = phases
            .iter()
            .take(8)
            .map(|(step, duration)| format!("{:?}={}ms", step, duration))
            .collect::<Vec<_>>()
            .join(", ");

        info!(
            tick = result.tick_number,
            elapsed_ms = total_ms,
            phases = %phase_summary,
            "tick completed"
        );

        if !result.success {
            warn!(
                tick = result.tick_number,
                ?result.failed_steps,
                "tick reported failures; stopping early"
            );
            break;
        }

        if engine.state.ticknum >= engine.state.config.iterations {
            info!("configured iterations exhausted ({}); stopping", engine.state.config.iterations);
            break;
        }

        if (tick + 1) % opts.report_every == 0 {
            println!("tick {:>6} | total {:>8.2} ms | top phases: {}", result.tick_number, total_ms, phase_summary);
        }
    }

    Ok(())
}

struct CliOptions {
    scenario_path: PathBuf,
    ticks: u32,
    report_every: u32,
    tick_logging: bool,
}

impl CliOptions {
    fn from_args<I>(args: I) -> Self
    where
        I: Iterator<Item = String>,
    {
        let mut scenario_path: Option<PathBuf> = None;
        let mut ticks: Option<u32> = None;
        let mut report_every: Option<u32> = None;
        let mut tick_logging = false;

        let mut iter = args.peekable();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--scenario" | "-s" => {
                    if let Some(path) = iter.next() {
                        scenario_path = Some(PathBuf::from(path));
                    }
                }
                "--ticks" | "-t" => {
                    if let Some(value) = iter.next() {
                        ticks = value.parse().ok();
                    }
                }
                "--report" | "-r" => {
                    if let Some(value) = iter.next() {
                        report_every = value.parse().ok();
                    }
                }
                "--log" => tick_logging = true,
                _ => {
                    // Support positional args for quick usage.
                    if scenario_path.is_none() {
                        scenario_path = Some(PathBuf::from(arg));
                    } else if ticks.is_none() {
                        ticks = arg.parse().ok();
                    }
                }
            }
        }

        Self {
            scenario_path: scenario_path.unwrap_or_else(|| PathBuf::from("config/config.toml")),
            ticks: ticks.unwrap_or(100),
            report_every: report_every.unwrap_or(10),
            tick_logging,
        }
    }
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .try_init();
}
