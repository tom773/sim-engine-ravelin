use super::*;
use crate::executor::SimulationEngine; // Points to our new engine struct
use petgraph::Direction;
use petgraph::Graph;
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

#[derive(Debug)]
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

    pub fn handler_count(&self) -> usize {
        self.step_handlers.len()
    }

    pub fn execute_tick(&self, engine: &mut SimulationEngine, rng: &mut dyn rand::RngCore) -> TickExecutionResult {
        let start_time = Instant::now();
        let mut context = StepContext::new(engine.state.ticknum);
        let mut failed_steps = Vec::new();

        for (_layer_idx, layer) in self.layers.iter().enumerate() {
            for &step in layer {
                if let Some(handler) = self.step_handlers.get(&step) {
                    let step_start = Instant::now();
                    let result = handler.execute(engine, &mut context, rng);
                    let _duration = step_start.elapsed();
                    context.step_data.insert(step, result.clone());

                    if !result.success {
                        let error = result.error.unwrap_or("Unknown error".to_string());
                        failed_steps.push((step, error.clone()));

                        if step.should_abort_on_failure() {
                            println!(
                                "[SCHEDULER] Aborting tick {} due to critical failure in {:?}: {}",
                                engine.state.ticknum, step, error
                            );
                            break;
                        }
                    }
                } else {
                    let error = format!("No handler registered for step {:?}", step);
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

    pub fn print_execution_plan(&self) {
        println!("=== Tick Execution Plan ===");
        for (i, layer) in self.layers.iter().enumerate() {
            println!("Layer {}: {:?}", i, layer);
        }
        println!("===========================");
    }
}

impl Default for TickScheduler {
    fn default() -> Self {
        Self::new()
    }
}
