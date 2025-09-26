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

#[cfg(feature = "server")]
use std::net::SocketAddr;

#[cfg(feature = "server")]
pub fn init_prometheus_metrics(addr: SocketAddr) -> Result<(), String> {
    let result = metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()
        .map_err(|e| e.to_string());

    if result.is_ok() {
        metrics::gauge!("sim_api.prometheus_up", 1.0);
    }

    result
}

#[cfg(not(feature = "server"))]
pub fn init_prometheus_metrics(_addr: std::net::SocketAddr) -> Result<(), String> {
    Err("prometheus metrics disabled".into())
}
