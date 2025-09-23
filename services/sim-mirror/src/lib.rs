use arc_swap::ArcSwap;
use crossbeam_channel::{Receiver, Sender, unbounded};
use serde::Serialize;
use std::sync::{Arc, Mutex};

pub mod query;
pub mod runtime;
pub use query::*;
pub use runtime::*;

type Snapshot = Arc<StateDigest>;

/// Shared handle that lets publishers push new digests and subscribers receive them.
/// Mimics the future production flow: the simulation publishes immutable digests,
/// while downstream services read cached snapshots.
#[derive(Clone)]
pub struct MirrorHandle {
    inner: Arc<CacheInner>,
}

struct CacheInner {
    latest: ArcSwap<StateDigest>,
    subscribers: Mutex<Vec<Sender<Snapshot>>>,
}

impl MirrorHandle {
    /// Create a new cache initialised with a bootstrap digest.
    pub fn new() -> Self {
        Self::with_initial(StateDigest::bootstrap())
    }

    /// Create a cache seeded with a caller-provided digest.
    pub fn with_initial(initial: StateDigest) -> Self {
        let latest = ArcSwap::from_pointee(initial);
        let inner = CacheInner { latest, subscribers: Mutex::new(Vec::new()) };
        Self { inner: Arc::new(inner) }
    }

    /// Publish a new digest into the cache and fan it out to subscribers.
    pub fn publish(&self, digest: StateDigest) {
        let snapshot: Snapshot = Arc::new(digest);
        self.inner.latest.store(snapshot.clone());

        let mut subscribers = self.inner.subscribers.lock().expect("subscriber lock");
        subscribers.retain(|sender| sender.send(snapshot.clone()).is_ok());
    }

    /// Read the most recent digest without blocking.
    pub fn latest(&self) -> Snapshot {
        self.inner.latest.load_full()
    }

    /// Subscribe to future digests. The receiver is primed with the current snapshot
    /// so listeners always begin with a consistent view.
    pub fn subscribe(&self) -> Receiver<Snapshot> {
        let (tx, rx) = unbounded();
        let current = self.inner.latest.load_full();
        let _ = tx.send(current);

        self.inner.subscribers.lock().expect("subscriber lock").push(tx);
        rx
    }
}

/// Minimal stand-in for the richer state bundle gathered from the simulation.
#[derive(Debug, Clone, Serialize)]
pub struct StateDigest {
    pub tick: u32,
    pub sim_time: String,
    pub metrics: DigestMetrics,
    pub highlights: Vec<DigestEvent>,
}

impl StateDigest {
    pub fn new(tick: u32, sim_time: impl Into<String>, metrics: DigestMetrics, highlights: Vec<DigestEvent>) -> Self {
        Self { tick, sim_time: sim_time.into(), metrics, highlights }
    }

    pub fn bootstrap() -> Self {
        Self::new(0, "bootstrap", DigestMetrics::default(), vec![DigestEvent::info("mirror", "initialised")])
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DigestMetrics {
    pub total_agents: u32,
    pub total_cash: f64,
    pub total_inventory: f64,
}

impl DigestMetrics {
    pub fn new(total_agents: u32, total_cash: f64, total_inventory: f64) -> Self {
        Self { total_agents, total_cash, total_inventory }
    }
}

impl Default for DigestMetrics {
    fn default() -> Self {
        Self { total_agents: 0, total_cash: 0.0, total_inventory: 0.0 }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DigestEvent {
    pub kind: DigestEventKind,
    pub message: String,
}

impl DigestEvent {
    pub fn info(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self { kind: DigestEventKind::Info(context.into()), message: message.into() }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum DigestEventKind {
    Info(String),
    Warning(String),
    Error(String),
}

pub mod prelude {
    pub use super::{DigestEvent, DigestEventKind, DigestMetrics, MirrorHandle, StateDigest};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_updates_and_streams() {
        let mirror = MirrorHandle::new();
        let rx = mirror.subscribe();

        assert_eq!(rx.recv().unwrap().tick, 0);

        let metrics = DigestMetrics::new(1, 10.0, 5.0);
        mirror.publish(StateDigest::new(1, "t+1", metrics.clone(), vec![]));
        mirror.publish(StateDigest::new(2, "t+2", metrics, vec![]));

        assert_eq!(rx.recv().unwrap().tick, 1);
        assert_eq!(rx.recv().unwrap().tick, 2);
        assert_eq!(mirror.latest().tick, 2);
    }
}
