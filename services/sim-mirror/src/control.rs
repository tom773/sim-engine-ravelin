use crate::{ControlError, JetStreamConfig, JetStreamError, JetStreamSchema, MirrorControlStatus, MirrorController};
use async_nats::jetstream::{
    self,
    consumer::{self, DeliverPolicy, push},
};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    convert::TryFrom,
    process,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::Duration,
};
use tokio::runtime::{Builder as RuntimeBuilder, Handle};
use tokio::sync::{mpsc, oneshot};
use tokio::task;
use tokio_stream::StreamExt;
use tracing::{error, warn};

pub trait MirrorControl: Send + Sync {
    fn pause(&self) -> Result<(), ControlError>;
    fn resume(&self) -> Result<(), ControlError>;
    fn step(&self) -> Result<(), ControlError>;
    fn set_interval(&self, interval: Duration) -> Result<(), ControlError>;
    fn status(&self) -> MirrorControlStatus;
}

/// JetStream message carrying a control command issued by a remote client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RemoteControlCommand {
    Pause,
    Resume,
    Step,
    SetInterval { millis: u64 },
}

/// JetStream message emitted by the runtime so remote controllers observe liveness and state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteControlStatus {
    pub status: MirrorControlStatus,
    pub timestamp: DateTime<Utc>,
}

/// Guard that keeps the JetStream control worker alive for the embedded runtime.
pub struct JetStreamControlServer {
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
    join: Mutex<Option<thread::JoinHandle<()>>>,
}

impl JetStreamControlServer {
    pub fn start(
        config: JetStreamConfig, schema: JetStreamSchema, controller: MirrorController,
    ) -> Result<Self, JetStreamError> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let thread_name = format!("jetstream-control-{}", config.connection_name.replace(' ', ""));
        let join = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                if let Err(err) = run_control_server(config, schema, controller, shutdown_rx) {
                    error!(error = %err, "jetstream control server terminated");
                }
            })
            .map_err(|err| JetStreamError::new(format!("failed to spawn jetstream control thread: {err}")))?;

        Ok(Self { shutdown: Mutex::new(Some(shutdown_tx)), join: Mutex::new(Some(join)) })
    }
}

impl Drop for JetStreamControlServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.lock().expect("shutdown guard").take() {
            let _ = tx.send(());
        }

        if let Some(handle) = self.join.lock().expect("join guard").take() {
            if let Err(err) = handle.join() {
                warn!(?err, "failed to join jetstream control thread");
            }
        }
    }
}

fn run_control_server(
    config: JetStreamConfig, schema: JetStreamSchema, controller: MirrorController, shutdown: oneshot::Receiver<()>,
) -> Result<(), JetStreamError> {
    let runtime = RuntimeBuilder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| JetStreamError::new(format!("failed to build tokio runtime: {err}")))?;

    runtime.block_on(async move {
        let client = async_nats::connect(&config.url)
            .await
            .map_err(|err| JetStreamError::new(format!("failed to connect to NATS at {}: {err}", config.url)))?;
        let context = jetstream::new(client);

        ensure_stream(&context, &schema).await?;

        let stream = context
            .get_stream(&schema.stream_name)
            .await
            .map_err(|err| JetStreamError::new(format!("failed to fetch JetStream stream: {err}")))?;

        let deliver_subject = format!("{}.deliver.control.{}", schema.intent_subject, process::id());
        let consumer_name = format!("sim_mirror_control_{}", process::id());
        let consumer_config = push::Config {
            deliver_subject: deliver_subject.clone(),
            durable_name: Some(consumer_name.clone()),
            name: Some(consumer_name.clone()),
            ack_policy: consumer::AckPolicy::Explicit,
            deliver_policy: DeliverPolicy::New,
            filter_subject: schema.intent_subject.to_string(),
            ..Default::default()
        };

        let consumer = stream
            .get_or_create_consumer(&consumer_name, consumer_config)
            .await
            .map_err(|err| JetStreamError::new(format!("failed to create JetStream control consumer: {err}")))?;

        let mut messages = consumer
            .messages()
            .await
            .map_err(|err| JetStreamError::new(format!("failed to subscribe to control messages: {err}")))?;

        let mut heartbeat = tokio::time::interval(Duration::from_secs(1));
        let shutdown = shutdown;
        tokio::pin!(shutdown);

        publish_status(&context, &schema, controller.status()).await?;

        loop {
            tokio::select! {
                _ = &mut shutdown => {
                    break;
                }
                _ = heartbeat.tick() => {
                    publish_status(&context, &schema, controller.status()).await?;
                }
                maybe_msg = messages.next() => {
                    match maybe_msg {
                        Some(Ok(message)) => {
                            if let Err(err) = handle_control_message(&controller, &context, &schema, &message).await {
                                warn!(error = %err, "failed to handle control message");
                            }
                            if let Err(err) = message.ack().await {
                                warn!(error = %err, "failed to ack control message");
                            }
                        }
                        Some(Err(err)) => {
                            warn!(error = %err, "JetStream control consumer error");
                        }
                        None => break,
                    }
                }
            }
        }

        Ok::<(), JetStreamError>(())
    })
}

struct RemoteControllerGuard {
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
    join: Mutex<Option<thread::JoinHandle<()>>>,
}

impl Drop for RemoteControllerGuard {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.lock().expect("shutdown guard").take() {
            let _ = tx.send(());
        }

        if let Some(handle) = self.join.lock().expect("join guard").take() {
            if let Err(err) = handle.join() {
                warn!(?err, "failed to join jetstream control client thread");
            }
        }
    }
}

enum ControllerRequest {
    Pause { respond: oneshot::Sender<Result<(), ControlError>> },
    Resume { respond: oneshot::Sender<Result<(), ControlError>> },
    Step { respond: oneshot::Sender<Result<(), ControlError>> },
    SetInterval { millis: u64, respond: oneshot::Sender<Result<(), ControlError>> },
}

/// Lightweight client that forwards control commands over JetStream and tracks mirror status updates.
pub struct RemoteMirrorController {
    tx: mpsc::Sender<ControllerRequest>,
    status: Arc<RwLock<MirrorControlStatus>>,
    status_signal: Arc<(Mutex<u64>, Condvar)>,
    _guard: RemoteControllerGuard,
}

impl RemoteMirrorController {
    pub fn connect(config: JetStreamConfig, schema: JetStreamSchema) -> Result<Self, JetStreamError> {
        let (tx, rx) = mpsc::channel(64);
        let status = Arc::new(RwLock::new(MirrorControlStatus { running: false, tick_interval_ms: 0 }));
        let status_clone = status.clone();
        let status_signal = Arc::new((Mutex::new(0u64), Condvar::new()));
        let status_signal_clone = status_signal.clone();

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (ready_tx, ready_rx) = oneshot::channel();

        let thread_name = format!("jetstream-control-client-{}", config.connection_name.replace(' ', ""));
        let join = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                if let Err(err) =
                    run_remote_controller(config, schema, rx, status_clone, status_signal_clone, shutdown_rx, ready_tx)
                {
                    error!(error = %err, "jetstream remote controller terminated");
                }
            })
            .map_err(|err| JetStreamError::new(format!("failed to spawn jetstream control client thread: {err}")))?;

        let mut ready_rx_opt = Some(ready_rx);
        let mut recv_ready = || {
            ready_rx_opt
                .take()
                .expect("ready receiver consumed")
                .blocking_recv()
                .unwrap_or_else(|_| Err(JetStreamError::new("remote controller startup channel closed")))
        };

        let startup_result =
            if Handle::try_current().is_ok() { task::block_in_place(|| recv_ready()) } else { recv_ready() };

        match startup_result {
            Ok(()) => {}
            Err(err) => {
                let _ = shutdown_tx.send(());
                let _ = join.join();
                return Err(err);
            }
        }

        let guard = RemoteControllerGuard { shutdown: Mutex::new(Some(shutdown_tx)), join: Mutex::new(Some(join)) };

        Ok(Self { tx, status, status_signal, _guard: guard })
    }

    fn command(
        &self, request: ControllerRequest, respond: oneshot::Receiver<Result<(), ControlError>>,
    ) -> Result<(), ControlError> {
        let version_before = snapshot_status_version(&self.status_signal);
        if Handle::try_current().is_ok() {
            let tx = self.tx.clone();
            let signal = self.status_signal.clone();
            return task::block_in_place(move || {
                let result = send_and_wait_blocking(&tx, request, respond);
                if result.is_err() {
                    return result;
                }
                wait_for_status_update(&signal, version_before)
            });
        }

        let result = send_and_wait_blocking(&self.tx, request, respond);
        if result.is_err() {
            return result;
        }
        wait_for_status_update(&self.status_signal, version_before)
    }
}

fn send_and_wait_blocking(
    tx: &mpsc::Sender<ControllerRequest>, request: ControllerRequest,
    respond: oneshot::Receiver<Result<(), ControlError>>,
) -> Result<(), ControlError> {
    tx.blocking_send(request).map_err(|_| ControlError::Disconnected)?;
    respond.blocking_recv().unwrap_or_else(|_| Err(ControlError::Disconnected))
}

fn snapshot_status_version(signal: &Arc<(Mutex<u64>, Condvar)>) -> u64 {
    let (lock, _) = &**signal;
    *lock.lock().expect("status version lock")
}

fn wait_for_status_update(signal: &Arc<(Mutex<u64>, Condvar)>, version_before: u64) -> Result<(), ControlError> {
    let (lock, condvar) = &**signal;
    let mut guard = lock.lock().expect("status version lock");
    if *guard != version_before {
        return Ok(());
    }

    let wait_result = condvar
        .wait_timeout_while(guard, Duration::from_secs(2), |version| *version == version_before)
        .expect("status condvar wait");
    guard = wait_result.0;
    if wait_result.1.timed_out() && *guard == version_before { Err(ControlError::Timeout) } else { Ok(()) }
}

impl MirrorControl for RemoteMirrorController {
    fn pause(&self) -> Result<(), ControlError> {
        let (respond_tx, respond_rx) = oneshot::channel();
        self.command(ControllerRequest::Pause { respond: respond_tx }, respond_rx)
    }

    fn resume(&self) -> Result<(), ControlError> {
        let (respond_tx, respond_rx) = oneshot::channel();
        self.command(ControllerRequest::Resume { respond: respond_tx }, respond_rx)
    }

    fn step(&self) -> Result<(), ControlError> {
        let (respond_tx, respond_rx) = oneshot::channel();
        self.command(ControllerRequest::Step { respond: respond_tx }, respond_rx)
    }

    fn set_interval(&self, interval: Duration) -> Result<(), ControlError> {
        let millis = interval.as_millis().max(1) as u64;
        let (respond_tx, respond_rx) = oneshot::channel();
        self.command(ControllerRequest::SetInterval { millis, respond: respond_tx }, respond_rx)
    }

    fn status(&self) -> MirrorControlStatus {
        self.status.read().clone()
    }
}

fn run_remote_controller(
    config: JetStreamConfig, schema: JetStreamSchema, rx: mpsc::Receiver<ControllerRequest>,
    status: Arc<RwLock<MirrorControlStatus>>, status_signal: Arc<(Mutex<u64>, Condvar)>,
    shutdown: oneshot::Receiver<()>, ready: oneshot::Sender<Result<(), JetStreamError>>,
) -> Result<(), JetStreamError> {
    let runtime = RuntimeBuilder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| JetStreamError::new(format!("failed to build tokio runtime: {err}")))?;

    runtime.block_on(remote_control_loop(config, schema, rx, status, status_signal, shutdown, ready))
}

async fn remote_control_loop(
    config: JetStreamConfig, schema: JetStreamSchema, mut rx: mpsc::Receiver<ControllerRequest>,
    status: Arc<RwLock<MirrorControlStatus>>, status_signal: Arc<(Mutex<u64>, Condvar)>,
    shutdown: oneshot::Receiver<()>, ready: oneshot::Sender<Result<(), JetStreamError>>,
) -> Result<(), JetStreamError> {
    let mut ready_opt = Some(ready);

    let client = match async_nats::connect(&config.url).await {
        Ok(client) => client,
        Err(err) => {
            let jet_err = JetStreamError::new(format!("failed to connect to NATS at {}: {err}", config.url));
            if let Some(sender) = ready_opt.take() {
                let _ = sender.send(Err(jet_err.clone()));
            }
            return Err(jet_err);
        }
    };
    let context = jetstream::new(client);

    if let Err(err) = ensure_stream(&context, &schema).await {
        if let Some(sender) = ready_opt.take() {
            let _ = sender.send(Err(err.clone()));
        }
        return Err(err);
    }

    let stream = match context.get_stream(&schema.stream_name).await {
        Ok(stream) => stream,
        Err(err) => {
            let jet_err = JetStreamError::new(format!("failed to fetch JetStream stream: {err}"));
            if let Some(sender) = ready_opt.take() {
                let _ = sender.send(Err(jet_err.clone()));
            }
            return Err(jet_err);
        }
    };

    let status_deliver_subject = format!("{}.deliver.status.{}", schema.control_subject, process::id());
    let status_consumer_name = format!("sim_api_control_status_{}", process::id());
    let status_consumer_config = push::Config {
        deliver_subject: status_deliver_subject.clone(),
        durable_name: Some(status_consumer_name.clone()),
        name: Some(status_consumer_name.clone()),
        ack_policy: consumer::AckPolicy::Explicit,
        deliver_policy: DeliverPolicy::Last,
        filter_subject: schema.control_subject.to_string(),
        ..Default::default()
    };

    let status_consumer = match stream.get_or_create_consumer(&status_consumer_name, status_consumer_config).await {
        Ok(consumer) => consumer,
        Err(err) => {
            let jet_err = JetStreamError::new(format!("failed to create control status consumer: {err}"));
            if let Some(sender) = ready_opt.take() {
                let _ = sender.send(Err(jet_err.clone()));
            }
            return Err(jet_err);
        }
    };

    let mut messages = match status_consumer.messages().await {
        Ok(messages) => messages,
        Err(err) => {
            let jet_err = JetStreamError::new(format!("failed to subscribe to control status: {err}"));
            if let Some(sender) = ready_opt.take() {
                let _ = sender.send(Err(jet_err.clone()));
            }
            return Err(jet_err);
        }
    };

    let shutdown = shutdown;
    tokio::pin!(shutdown);
    if let Some(sender) = ready_opt.take() {
        let _ = sender.send(Ok(()));
    }

    // Main loop: forward commands and keep the cached status fresh.
    loop {
        tokio::select! {
            _ = &mut shutdown => {
                if let Some(sender) = ready_opt.take() {
                    let _ = sender.send(Err(JetStreamError::new("remote controller shutdown")));
                }
                break;
            }
            maybe_request = rx.recv() => match maybe_request {
                Some(request) => {
                    handle_remote_request(request, &context, &schema).await;
                }
                None => break,
            },
            maybe_msg = messages.next() => match maybe_msg {
                Some(Ok(message)) => {
                    let success = apply_status_message(&status, &status_signal, message).await;
                    if success {
                        if let Some(sender) = ready_opt.take() {
                            let _ = sender.send(Ok(()));
                        }
                    }
                }
                Some(Err(err)) => {
                    warn!(error = %err, "control status consumer error");
                }
                None => break,
            },
        }
    }

    Ok(())
}

async fn handle_remote_request(request: ControllerRequest, context: &jetstream::Context, schema: &JetStreamSchema) {
    match request {
        ControllerRequest::Pause { respond } => {
            let result = publish_command(context, schema, RemoteControlCommand::Pause).await;
            let _ = respond.send(result);
        }
        ControllerRequest::Resume { respond } => {
            let result = publish_command(context, schema, RemoteControlCommand::Resume).await;
            let _ = respond.send(result);
        }
        ControllerRequest::Step { respond } => {
            let result = publish_command(context, schema, RemoteControlCommand::Step).await;
            let _ = respond.send(result);
        }
        ControllerRequest::SetInterval { millis, respond } => {
            let result = if millis == 0 {
                Err(ControlError::Transport("set_interval requested with zero millis".into()))
            } else {
                publish_command(context, schema, RemoteControlCommand::SetInterval { millis }).await
            };
            let _ = respond.send(result);
        }
    }
}

async fn publish_command(
    context: &jetstream::Context, schema: &JetStreamSchema, command: RemoteControlCommand,
) -> Result<(), ControlError> {
    let payload = serde_json::to_vec(&command)
        .map_err(|err| ControlError::Transport(format!("failed to serialize control command: {err}")))?;
    context
        .publish(schema.intent_subject, payload.into())
        .await
        .map_err(|err| ControlError::Transport(format!("failed to publish control command: {err}")))?;
    Ok(())
}

async fn apply_status_message(
    status: &Arc<RwLock<MirrorControlStatus>>, status_signal: &Arc<(Mutex<u64>, Condvar)>,
    message: async_nats::jetstream::Message,
) -> bool {
    let mut success = false;
    match serde_json::from_slice::<RemoteControlStatus>(&message.payload) {
        Ok(remote_status) => {
            *status.write() = remote_status.status;
            notify_status_listeners(status_signal);
            success = true;
        }
        Err(err) => {
            warn!(error = %err, "failed to deserialize control status message");
        }
    }

    if let Err(err) = message.ack().await {
        warn!(error = %err, "failed to ack control status message");
    }

    success
}

fn notify_status_listeners(signal: &Arc<(Mutex<u64>, Condvar)>) {
    let (lock, condvar) = &**signal;
    let mut guard = lock.lock().expect("status version lock");
    let next = (*guard).wrapping_add(1);
    *guard = next;
    condvar.notify_all();
}

async fn handle_control_message(
    controller: &MirrorController, context: &jetstream::Context, schema: &JetStreamSchema,
    message: &async_nats::Message,
) -> Result<(), JetStreamError> {
    let command: RemoteControlCommand = serde_json::from_slice(&message.payload)
        .map_err(|err| JetStreamError::new(format!("failed to deserialize control command: {err}")))?;

    let result = match command {
        RemoteControlCommand::Pause => controller.pause(),
        RemoteControlCommand::Resume => controller.resume(),
        RemoteControlCommand::Step => controller.step(),
        RemoteControlCommand::SetInterval { millis } => {
            if millis == 0 {
                Err(ControlError::Transport("set_interval requested with zero millis".into()))
            } else {
                controller.set_interval(Duration::from_millis(millis))
            }
        }
    };

    if let Err(err) = result {
        warn!(error = %err, "control command failed");
    }

    publish_status(context, schema, controller.status()).await
}

async fn publish_status(
    context: &jetstream::Context, schema: &JetStreamSchema, status: MirrorControlStatus,
) -> Result<(), JetStreamError> {
    let payload = RemoteControlStatus { status, timestamp: Utc::now() };
    let bytes = serde_json::to_vec(&payload)
        .map_err(|err| JetStreamError::new(format!("failed to serialize control status: {err}")))?;
    context
        .publish(schema.control_subject, bytes.into())
        .await
        .map_err(|err| JetStreamError::new(format!("failed to publish control status: {err}")))?;
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
