pub mod executor;
pub mod factory;
pub mod registry;
pub mod scenario;
pub mod scheduler;
// Re-export the core components for easy access.
pub use executor::*;
pub use factory::*;
pub use registry::*;
pub use scenario::*;
pub use scheduler::*;

use std::net::SocketAddr;

pub fn init_prometheus_metrics(addr: SocketAddr) -> Result<(), String> {
    metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(addr)
        .install_recorder()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
