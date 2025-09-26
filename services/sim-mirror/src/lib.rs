use arc_swap::ArcSwap;
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::sync::{Arc, Mutex};

#[cfg(feature = "server")]
pub mod broker;
#[cfg(feature = "server")]
pub mod control;
pub mod digest;
#[cfg(feature = "server")]
pub mod query;
#[cfg(feature = "server")]
pub mod runtime;

#[cfg(feature = "server")]
pub use broker::*;
#[cfg(feature = "server")]
pub use control::*;
pub use digest::*;
#[cfg(feature = "server")]
pub use query::*;
#[cfg(feature = "server")]
pub use runtime::*;

pub trait DigestPublisher: Send + Sync {
    fn publish(&self, snapshot: Arc<StateSnapshot>);
    fn label(&self) -> &'static str;
}

type Snapshot = Arc<StateSnapshot>;

/// Shared handle that lets publishers push new digests and subscribers receive them.
/// Mimics the future production flow: the simulation publishes immutable digests,
/// while downstream services read cached snapshots.
#[derive(Clone)]
pub struct MirrorHandle {
    inner: Arc<CacheInner>,
}

struct CacheInner {
    latest: ArcSwap<StateSnapshot>,
    subscribers: Mutex<Vec<Sender<Snapshot>>>,
    publishers: Mutex<Vec<Arc<dyn DigestPublisher>>>,
}

impl MirrorHandle {
    /// Create a new cache initialised with a bootstrap digest.
    pub fn new() -> Self {
        Self::with_initial(StateDigest::bootstrap())
    }

    /// Create a cache seeded with a caller-provided digest.
    pub fn with_initial(initial: StateDigest) -> Self {
        let bootstrap = Arc::new(StateSnapshot::from_digest(initial));
        let latest = ArcSwap::from(bootstrap);
        let inner = CacheInner { latest, subscribers: Mutex::new(Vec::new()), publishers: Mutex::new(Vec::new()) };
        Self { inner: Arc::new(inner) }
    }

    /// Publish a new digest into the cache, compute a delta, and fan it out to subscribers.
    pub fn publish(&self, digest: StateDigest) -> Snapshot {
        let previous = self.inner.latest.load_full();
        let delta = StateDelta::between(Some(previous.digest.as_ref()), &digest);
        let snapshot = Arc::new(StateSnapshot { digest: Arc::new(digest), delta });
        self.inner.latest.store(snapshot.clone());

        let mut subscribers = self.inner.subscribers.lock().expect("subscriber lock");
        subscribers.retain(|sender| sender.send(snapshot.clone()).is_ok());
        drop(subscribers);

        let publishers = self.inner.publishers.lock().expect("publisher lock");
        for publisher in publishers.iter() {
            publisher.publish(snapshot.clone());
        }

        snapshot
    }

    /// Attach an out-of-process publisher (broker, telemetry bus, etc.).
    pub fn attach_publisher<P>(&self, publisher: P)
    where
        P: DigestPublisher + 'static,
    {
        let mut guard = self.inner.publishers.lock().expect("publisher lock");
        let arc: Arc<dyn DigestPublisher> = Arc::new(publisher);
        guard.push(arc);
    }

    /// Read the most recent snapshot without blocking.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_updates_and_streams() {
        let mirror = MirrorHandle::new();
        let rx = mirror.subscribe();

        assert_eq!(rx.recv().unwrap().digest.tick, 0);

        let mut digest = StateDigest::bootstrap();
        digest.tick = 1;
        digest.status.tick = 1;
        mirror.publish(digest.clone());

        digest.tick = 2;
        digest.status.tick = 2;
        mirror.publish(digest);

        assert_eq!(rx.recv().unwrap().digest.tick, 1);
        assert_eq!(rx.recv().unwrap().digest.tick, 2);
        assert_eq!(mirror.latest().digest.tick, 2);
    }
}
