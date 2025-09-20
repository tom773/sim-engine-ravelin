use sim_core::prelude::*;

fn main() {
    tracing_subscriber::fmt::init();    
    let catalog = InstrumentCatalog::new();
    tracing::info!("Registry {:#?}", catalog);
}