use once_cell::sync::Lazy;
use serde::Serialize;
use tokio::sync::broadcast;

const RING: usize = 1024;
pub static BUS: Lazy<broadcast::Sender<String>> = Lazy::new(|| {
    let (tx, _rx) = broadcast::channel(RING);
    tx
});

pub fn subscribe() -> broadcast::Receiver<String> {
    BUS.subscribe()
}

pub fn publish<T: Serialize>(event: &T) {
    if let Ok(s) = serde_json::to_string(event) {
        let _ = BUS.send(s);
    }
}

#[macro_export]
macro_rules! dbg_evt {
    ($expr:expr) => {{
        $crate::debug_bus::publish(&$expr);
    }};
}