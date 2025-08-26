use super::*;
use petgraph::Direction;
use petgraph::Graph;
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

pub struct TickScheduler {
    graph: Graph<TickStep, ()>,
    _nodes: HashMap<TickStep, NodeIndex>,
    layers: Vec<Vec<TickStep>>,
    step_handlers: HashMap<TickStep, Box<dyn StepHandler + Send + Sync>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TickExecutionResult {
    pub tick_number: u32,
    pub total_duration: std::time::Duration,
    pub step_results: HashMap<TickStep, StepResult>,
    pub failed_steps: Vec<(TickStep, String)>,
    pub success: bool,
}

impl TickScheduler {
    pub fn new() -> Self {
        let mut graph = Graph::new();
        let mut _nodes = HashMap::new();

        for step in TickStep::all() {
            _nodes.insert(step, graph.add_node(step));
        }

        for step in TickStep::all() {
            let step_node = _nodes[&step];
            for dependency in step.dependencies() {
                let dep_node = _nodes[&dependency];
                graph.add_edge(dep_node, step_node, ());
            }
        }

        let layers = Self::build_execution_layers(&graph);

        Self { graph, _nodes, layers, step_handlers: HashMap::new() }
    }

    fn build_execution_layers(graph: &Graph<TickStep, ()>) -> Vec<Vec<TickStep>> {
        let mut layers = Vec::new();
        let mut remaining: HashSet<NodeIndex> = graph.node_indices().collect();

        while !remaining.is_empty() {
            let ready: Vec<NodeIndex> = remaining
                .iter()
                .filter(|&&node| {
                    graph.neighbors_directed(node, Direction::Incoming).all(|dep| !remaining.contains(&dep))
                })
                .copied()
                .collect();

            if ready.is_empty() {
                panic!("Cyclic dependency detected in tick scheduler");
            }

            let layer: Vec<TickStep> = ready.iter().map(|&idx| graph[idx]).collect();
            layers.push(layer);

            for node in ready {
                remaining.remove(&node);
            }
        }

        layers
    }

    pub fn register_handler<H: StepHandler + Send + Sync + 'static>(&mut self, step: TickStep, handler: H) {
        self.step_handlers.insert(step, Box::new(handler));
    }

    pub fn execute_tick(
        &self, engine: &mut crate::SimulationEngine, rng: &mut dyn rand::RngCore,
    ) -> TickExecutionResult {
        let start_time = Instant::now();
        let mut context = StepContext::new(engine.state.ticknum);
        let mut failed_steps = Vec::new();

        println!("[SCHEDULER] Starting tick {} execution", engine.state.ticknum);

        for (layer_idx, layer) in self.layers.iter().enumerate() {
            println!("[SCHEDULER] Executing layer {}: {:?}", layer_idx, layer);

            for &step in layer {
                if let Some(handler) = self.step_handlers.get(&step) {
                    if let Err(e) = handler.validates_preconditions(&context) {
                        println!("[SCHEDULER] Precondition failed for {:?}: {}", step, e);
                        failed_steps.push((step, e));
                        continue;
                    }

                    let step_start = Instant::now();
                    println!("[SCHEDULER] Starting step {:?}", step);

                    let result = handler.execute(engine, &mut context, rng);
                    let duration = step_start.elapsed();

                    println!("[SCHEDULER] Step {:?} completed in {:?} - success: {}", step, duration, result.success);

                    context.step_data.insert(step, result.clone());

                    if !result.success {
                        let error = result.error.unwrap_or("Unknown error".to_string());
                        failed_steps.push((step, error.clone()));

                        if step.should_abort_on_failure() {
                            println!("[SCHEDULER] Aborting tick due to failure in critical step {:?}: {}", step, error);
                            break;
                        } else {
                            println!(
                                "[SCHEDULER] Continuing despite failure in non-critical step {:?}: {}",
                                step, error
                            );
                        }
                    }
                } else {
                    let error = format!("No handler registered for step {:?}", step);
                    println!("[SCHEDULER] {}", error);
                    failed_steps.push((step, error));

                    if step.should_abort_on_failure() {
                        break;
                    }
                }
            }

            if failed_steps.iter().any(|(step, _)| step.should_abort_on_failure()) {
                break;
            }
        }

        let total_duration = start_time.elapsed();
        let success = failed_steps.is_empty();

        println!(
            "[SCHEDULER] Tick {} completed in {:?} - success: {}, failed steps: {}",
            engine.state.ticknum,
            total_duration,
            success,
            failed_steps.len()
        );

        TickExecutionResult {
            tick_number: context.tick_number,
            total_duration,
            step_results: context.step_data,
            failed_steps,
            success,
        }
    }

    pub fn validate_schedule(&self) -> Result<(), String> {
        if petgraph::algo::is_cyclic_directed(&self.graph) {
            return Err("Scheduler contains cycles".to_string());
        }

        for step in self.graph.node_weights() {
            if !self.step_handlers.contains_key(step) {
                return Err(format!("No handler registered for step {:?}", step));
            }
        }

        Ok(())
    }

    pub fn dry_run(&self) -> Result<Vec<Vec<TickStep>>, String> {
        self.validate_schedule()?;
        Ok(self.layers.clone())
    }

    pub fn print_execution_plan(&self) {
        println!("=== Tick Execution Plan ===");
        for (i, layer) in self.layers.iter().enumerate() {
            println!("Layer {}: {:?}", i, layer);
            for step in layer {
                let deps = step.dependencies();
                if !deps.is_empty() {
                    println!("  {:?} depends on: {:?}", step, deps);
                } else {
                    println!("  {:?} has no dependencies", step);
                }
            }
        }
        println!("===========================");
    }

    pub fn visualize_dot(&self) -> String {
        use petgraph::dot::{Config, Dot};
        format!("{:?}", Dot::with_config(&self.graph, &[Config::EdgeNoLabel]))
    }

    pub fn get_stats(&self) -> SchedulerStats {
        let total_steps = self.graph.node_count();
        let total_edges = self.graph.edge_count();
        let total_layers = self.layers.len();
        let max_parallelism = self.layers.iter().map(|layer| layer.len()).max().unwrap_or(0);

        let registered_handlers = self.step_handlers.len();
        let unregistered_steps: Vec<TickStep> =
            TickStep::all().into_iter().filter(|step| !self.step_handlers.contains_key(step)).collect();

        let potentially_parallel_steps = TickStep::all().into_iter().filter(|step| step.can_run_parallel()).count();

        SchedulerStats {
            total_steps,
            total_edges,
            total_layers,
            max_parallelism,
            registered_handlers,
            unregistered_steps,
            potentially_parallel_steps,
        }
    }

    pub fn handler_count(&self) -> usize {
        self.step_handlers.len()
    }

    pub fn has_handler(&self, step: TickStep) -> bool {
        self.step_handlers.contains_key(&step)
    }

    pub fn get_layers(&self) -> &[Vec<TickStep>] {
        &self.layers
    }
}

impl Default for TickScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SchedulerStats {
    pub total_steps: usize,
    pub total_edges: usize,
    pub total_layers: usize,
    pub max_parallelism: usize,
    pub registered_handlers: usize,
    pub unregistered_steps: Vec<TickStep>,
    pub potentially_parallel_steps: usize,
}

impl SchedulerStats {
    pub fn new() -> Self {
        Self {
            total_steps: 0,
            total_edges: 0,
            total_layers: 0,
            max_parallelism: 0,
            registered_handlers: 0,
            unregistered_steps: vec![],
            potentially_parallel_steps: 0,
        }
    }

    pub fn print_summary(&self) {
        println!("=== Scheduler Statistics ===");
        println!("Total steps: {}", self.total_steps);
        println!("Total dependencies: {}", self.total_edges);
        println!("Execution layers: {}", self.total_layers);
        println!("Max potential parallelism: {}", self.max_parallelism);
        println!("Registered handlers: {}", self.registered_handlers);
        println!("Potentially parallel steps: {}", self.potentially_parallel_steps);

        if !self.unregistered_steps.is_empty() {
            println!("Unregistered steps: {:?}", self.unregistered_steps);
        }
        println!("============================");
    }

    pub fn is_complete(&self) -> bool {
        self.unregistered_steps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = TickScheduler::new();
        assert!(!scheduler.layers.is_empty());
        assert_eq!(scheduler.graph.node_count(), TickStep::all().len());
    }

    #[test]
    fn test_no_cycles() {
        let scheduler = TickScheduler::new();
        assert!(!petgraph::algo::is_cyclic_directed(&scheduler.graph));
    }

    #[test]
    fn test_layer_ordering() {
        let scheduler = TickScheduler::new();

        let first_layer = &scheduler.layers[0];
        for &step in first_layer {
            assert!(
                step.dependencies().is_empty(),
                "Step {:?} in first layer has dependencies: {:?}",
                step,
                step.dependencies()
            );
        }

        for (layer_idx, layer) in scheduler.layers.iter().enumerate() {
            for &step in layer {
                let step_dependencies = step.dependencies();
                for dependency in step_dependencies {
                    let dep_layer = scheduler
                        .layers
                        .iter()
                        .enumerate()
                        .find(|(_, l)| l.contains(&dependency))
                        .map(|(i, _)| i)
                        .expect(&format!("Dependency {:?} not found in any layer", dependency));

                    assert!(
                        dep_layer < layer_idx,
                        "Step {:?} in layer {} depends on {:?} in layer {}",
                        step,
                        layer_idx,
                        dependency,
                        dep_layer
                    );
                }
            }
        }
    }

    #[test]
    fn test_stats() {
        let scheduler = TickScheduler::new();
        let stats = scheduler.get_stats();

        assert_eq!(stats.total_steps, TickStep::all().len());
        assert!(stats.total_layers > 0);
        assert_eq!(stats.registered_handlers, 0); // No handlers registered yet
        assert_eq!(stats.unregistered_steps.len(), TickStep::all().len());
    }
}
