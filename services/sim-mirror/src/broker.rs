use crate::{DigestPublisher, StateSnapshot};
use async_nats::jetstream;
use crossbeam_channel::{Receiver, Sender, TrySendError, bounded, unbounded};
use metrics::{counter, histogram};
use serde_json;
use std::convert::TryFrom;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use thiserror::Error;
use tracing::{error, trace, warn};

/// Declarative JetStream layout so the mirror, API, and downstream services share the same subject naming.
#[derive(Debug, Clone)]
pub struct JetStreamSchema {
    pub stream_name: &'static str,
    pub digest_subject: &'static str,
    pub delta_subject: &'static str,
    pub control_subject: &'static str,
    pub intent_subject: &'static str,
    pub retention_messages: usize,
}

impl Default for JetStreamSchema {
    fn default() -> Self {
        Self {
            stream_name: "SIM_MIRROR",
            digest_subject: "sim.mirror.digest",
            delta_subject: "sim.mirror.delta",
            control_subject: "sim.control.state",
            intent_subject: "sim.control.intent",
            retention_messages: 512,
        }
    }
}

impl JetStreamSchema {
    pub fn subjects(&self) -> Vec<String> {
        vec![
            self.digest_subject.to_string(),
            self.delta_subject.to_string(),
            self.control_subject.to_string(),
            self.intent_subject.to_string(),
        ]
    }
}

/// Simple in-memory broker used to validate publish costs and delivery latency without a running NATS instance.
#[derive(Clone)]
pub struct InMemoryBroker {
    schema: JetStreamSchema,
    tx: Sender<Arc<StateSnapshot>>,
    rx: Receiver<Arc<StateSnapshot>>,
}

impl InMemoryBroker {
    pub fn new(schema: JetStreamSchema) -> Self {
        let (tx, rx) = unbounded();
        Self { schema, tx, rx }
    }

    pub fn default() -> Self {
        Self::new(JetStreamSchema::default())
    }

    pub fn publisher(&self) -> InMemoryPublisher {
        InMemoryPublisher { _schema: self.schema.clone(), tx: self.tx.clone() }
    }

    pub fn consumer(&self) -> InMemoryConsumer {
        InMemoryConsumer { rx: self.rx.clone() }
    }
}

#[derive(Clone)]
pub struct InMemoryPublisher {
    _schema: JetStreamSchema,
    tx: Sender<Arc<StateSnapshot>>,
}

impl DigestPublisher for InMemoryPublisher {
    fn publish(&self, snapshot: Arc<StateSnapshot>) {
        let start = Instant::now();
        let result = self.tx.send(snapshot);
        if result.is_ok() {
            counter!("mirror.broker.published", 1, "publisher" => self.label());
            let elapsed = start.elapsed();
            metrics::histogram!("mirror.broker.enqueue_latency_ms", elapsed.as_secs_f64() * 1_000.0, "publisher" => self.label());
        } else {
            counter!("mirror.broker.publish_failures", 1, "publisher" => self.label());
        }
    }

    fn label(&self) -> &'static str {
        "in-memory"
    }
}

#[derive(Clone)]
pub struct InMemoryConsumer {
    rx: Receiver<Arc<StateSnapshot>>,
}

impl InMemoryConsumer {
    pub fn recv(&self) -> Result<Arc<StateSnapshot>, crossbeam_channel::RecvError> {
        let snapshot = self.rx.recv()?;
        trace!(tick = snapshot.digest.tick, "in-memory broker delivery");
        counter!("mirror.broker.delivered", 1, "consumer" => "in-memory");
        Ok(snapshot)
    }

    pub fn try_recv(&self) -> Result<Arc<StateSnapshot>, crossbeam_channel::TryRecvError> {
        self.rx.try_recv()
    }
}

#[derive(Debug, Clone)]
pub struct JetStreamConfig {
    pub url: String,
    pub connection_name: String,
    pub queue_capacity: usize,
}

impl Default for JetStreamConfig {
    fn default() -> Self {
        Self {
            url: "nats://127.0.0.1:4222".into(),
            connection_name: "sim-mirror-publisher".into(),
            queue_capacity: 1024,
        }
    }
}

#[derive(Debug, Error, Clone)]
pub enum JetStreamError {
    #[error("jetstream: {0}")]
    Generic(String),
}

impl JetStreamError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self::Generic(msg.into())
    }
}

enum PublishCommand {
    Snapshot(Arc<StateSnapshot>),
    Shutdown,
}

struct WorkerGuard {
    tx: Sender<PublishCommand>,
    shutdown: AtomicBool,
    join: Mutex<Option<thread::JoinHandle<()>>>,
}

impl WorkerGuard {
    fn new(tx: Sender<PublishCommand>, join: thread::JoinHandle<()>) -> Self {
        Self { tx, shutdown: AtomicBool::new(false), join: Mutex::new(Some(join)) }
    }
}

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        if !self.shutdown.swap(true, Ordering::SeqCst) {
            let _ = self.tx.send(PublishCommand::Shutdown);
        }

        if let Some(handle) = self.join.lock().expect("join guard").take() {
            if let Err(err) = handle.join() {
                warn!(?err, "failed to join jetstream publisher thread");
            }
        }
    }
}

#[derive(Clone)]
pub struct JetStreamPublisher {
    tx: Sender<PublishCommand>,
    _guard: Arc<WorkerGuard>,
}

impl JetStreamPublisher {
    pub fn connect(config: JetStreamConfig, schema: JetStreamSchema) -> Result<Self, JetStreamError> {
        let capacity = config.queue_capacity.max(4);
        let (tx, rx) = bounded(capacity);
        let (ready_tx, ready_rx) = bounded(1);

        let thread_name = format!("jetstream-{}", config.connection_name.replace(' ', ""));
        let tx_for_guard = tx.clone();

        let join = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                if let Err(err) = run_jetstream_worker(config, schema, rx, ready_tx) {
                    error!(error = %err, "jetstream publisher terminated");
                }
            })
            .map_err(|err| JetStreamError::new(format!("failed to spawn jetstream publisher thread: {err}")))?;

        match ready_rx.recv() {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                let _ = tx.send(PublishCommand::Shutdown);
                let _ = join.join();
                return Err(err);
            }
            Err(_) => {
                let _ = tx.send(PublishCommand::Shutdown);
                let _ = join.join();
                return Err(JetStreamError::new("jetstream publisher startup channel closed"));
            }
        }

        let guard = Arc::new(WorkerGuard::new(tx_for_guard, join));
        Ok(Self { tx, _guard: guard })
    }
}

impl DigestPublisher for JetStreamPublisher {
    fn publish(&self, snapshot: Arc<StateSnapshot>) {
        let start = Instant::now();
        match self.tx.try_send(PublishCommand::Snapshot(snapshot)) {
            Ok(_) => {
                counter!("mirror.broker.published", 1, "publisher" => self.label());
                histogram!(
                    "mirror.broker.enqueue_latency_ms",
                    start.elapsed().as_secs_f64() * 1_000.0,
                    "publisher" => self.label()
                );
            }
            Err(TrySendError::Full(_)) => {
                counter!("mirror.broker.publish_dropped", 1, "publisher" => self.label());
            }
            Err(TrySendError::Disconnected(_)) => {
                counter!("mirror.broker.publish_failures", 1, "publisher" => self.label());
            }
        }
    }

    fn label(&self) -> &'static str {
        "jetstream"
    }
}

fn run_jetstream_worker(
    config: JetStreamConfig, schema: JetStreamSchema, rx: Receiver<PublishCommand>,
    ready: Sender<Result<(), JetStreamError>>,
) -> Result<(), JetStreamError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| JetStreamError::new(format!("failed to build tokio runtime: {err}")))?;

    let client = match runtime.block_on(async_nats::connect(&config.url)) {
        Ok(client) => client,
        Err(err) => {
            let jet_err = JetStreamError::new(format!("failed to connect to NATS at {}: {err}", config.url));
            let _ = ready.send(Err(jet_err.clone()));
            return Err(jet_err);
        }
    };

    let context = jetstream::new(client);
    if let Err(err) = runtime.block_on(ensure_stream(&context, &schema)) {
        let jet_err = JetStreamError::new(format!("failed to ensure JetStream: {err}"));
        let _ = ready.send(Err(jet_err.clone()));
        return Err(jet_err);
    }

    let _ = ready.send(Ok(()));

    while let Ok(cmd) = rx.recv() {
        match cmd {
            PublishCommand::Snapshot(snapshot) => {
                if let Err(err) = runtime.block_on(publish_snapshot(&context, &schema, snapshot)) {
                    error!(error = %err, "jetstream publish failed");
                    counter!("mirror.broker.publish_failures", 1, "publisher" => "jetstream");
                }
            }
            PublishCommand::Shutdown => break,
        }
    }

    Ok(())
}

async fn ensure_stream(context: &jetstream::Context, schema: &JetStreamSchema) -> Result<(), JetStreamError> {
    let mut config = jetstream::stream::Config::default();
    config.name = schema.stream_name.to_string();
    config.subjects = schema.subjects();
    config.max_messages = i64::try_from(schema.retention_messages).unwrap_or(i64::MAX);

    context
        .get_or_create_stream(config)
        .await
        .map_err(|err| JetStreamError::new(format!("stream setup failed: {err}")))?;
    Ok(())
}

async fn publish_snapshot(
    context: &jetstream::Context, schema: &JetStreamSchema, snapshot: Arc<StateSnapshot>,
) -> Result<(), JetStreamError> {
    let digest_payload = serde_json::to_vec(snapshot.digest.as_ref())
        .map_err(|err| JetStreamError::new(format!("serialize digest failed: {err}")))?;

    let start = Instant::now();
    context
        .publish(schema.digest_subject, digest_payload.into())
        .await
        .map_err(|err| JetStreamError::new(format!("digest publish failed: {err}")))?;
    histogram!(
        "mirror.broker.jetstream.publish_ms",
        start.elapsed().as_secs_f64() * 1_000.0,
        "kind" => "digest"
    );
    counter!("mirror.broker.jetstream.published", 1, "kind" => "digest");

    Ok(())
}
