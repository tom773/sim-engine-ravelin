use arc_swap::ArcSwap;
use crossbeam_channel::{unbounded, Receiver, Sender};
use serde::Serialize;
use std::sync::{Arc, Mutex};

type Snapshot = Arc<TickDigest>;

/// Shared handle that lets publishers push new digests and subscribers receive them.
/// This mirrors the long-term pattern where the simulation publishes the latest tick
/// while API workers read from an immutable cache.
#[derive(Clone)]
pub struct MirrorHandle {
    inner: Arc<CacheInner>,
}

struct CacheInner {
    latest: ArcSwap<TickDigest>,
    subscribers: Mutex<Vec<Sender<Snapshot>>>,
}

impl MirrorHandle {
    /// Create a new cache initialised with an empty digest.
    pub fn new() -> Self {
        Self::with_initial(TickDigest::new(0, "bootstrap"))
    }

    /// Create a new cache with a custom starting digest.
    pub fn with_initial(initial: TickDigest) -> Self {
        let latest = ArcSwap::new(Arc::new(initial));
        let inner = CacheInner { latest, subscribers: Mutex::new(Vec::new()) };
        Self { inner: Arc::new(inner) }
    }

    /// Publish a new digest into the cache and fan it out to active subscribers.
    pub fn publish(&self, digest: TickDigest) {
        let snapshot: Snapshot = Arc::new(digest);
        self.inner.latest.store(snapshot.clone());

        let mut subscribers = self.inner.subscribers.lock().expect("subscriber lock");
        subscribers.retain(|sender| sender.send(snapshot.clone()).is_ok());
    }

    /// Get the most recent digest without blocking.
    pub fn latest(&self) -> Snapshot {
        self.inner.latest.load_full()
    }

    /// Subscribe to future digests. The returned receiver is immediately primed
    /// with the current snapshot so listeners always have data to render.
    pub fn subscribe(&self) -> Receiver<Snapshot> {
        let (tx, rx) = unbounded();
        let current = self.inner.latest.load_full();
        let _ = tx.send(current);

        self.inner.subscribers.lock().expect("subscriber lock").push(tx);
        rx
    }
}

/// Extremely small digest type that stands in for the richer state bundle
/// the simulator will emit.
#[derive(Debug, Clone, Serialize)]
pub struct TickDigest {
    pub tick: u32,
    pub summary: String,
}

impl TickDigest {
    pub fn new(tick: u32, summary: impl Into<String>) -> Self {
        Self { tick, summary: summary.into() }
    }
}

pub mod prelude {
    pub use super::{MirrorHandle, TickDigest};
}

pub mod testing {
    use super::{MirrorHandle, TickDigest};
    use std::{thread, time::Duration};

    /// Spawn a background thread that publishes monotonic tick digests at a fixed interval.
    pub fn spawn_counter_publisher(handle: MirrorHandle, interval: Duration) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let mut tick = 0u32;
            loop {
                handle.publish(TickDigest::new(tick, format!("mock summary {tick}")));
                tick = tick.wrapping_add(1);
                thread::sleep(interval);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_updates_and_streams() {
        let mirror = MirrorHandle::new();
        let rx = mirror.subscribe();

        assert_eq!(rx.recv().unwrap().tick, 0);

        mirror.publish(TickDigest::new(1, "tick one"));
        mirror.publish(TickDigest::new(2, "tick two"));

        assert_eq!(rx.recv().unwrap().tick, 1);
        assert_eq!(rx.recv().unwrap().tick, 2);
        assert_eq!(mirror.latest().tick, 2);
    }
}
