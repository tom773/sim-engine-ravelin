use std::{
    convert::{Infallible, TryFrom},
    env,
    net::SocketAddr,
    process,
    sync::Arc,
    time::{Duration, Instant},
};

use async_nats::jetstream::{
    self,
    consumer::{self, DeliverPolicy, push},
};
use axum::{
    Json, Router,
    extract::ws::{Message, WebSocket},
    extract::{State, WebSocketUpgrade},
    http::StatusCode,
    response::{
        IntoResponse,
        sse::{Event, Sse},
    },
    routing::{get, post},
};
use chrono::Utc;
use futures_core::stream::Stream;
use metrics::{counter, histogram};
use serde::{Deserialize, Serialize};
use sim_mirror::{
    ControlError, DashboardDto, JetStreamConfig, JetStreamError, JetStreamSchema, MirrorControl, MirrorControlStatus,
    MirrorError, MirrorHandle, MirrorRuntime, QueryService, RemoteMirrorController, RiskDigest, StateDelta,
    StateDigest, StatusDigest,
    broker::{InMemoryBroker, InMemoryConsumer},
};
use tokio::{net::TcpListener, sync::mpsc, task, task::JoinHandle};
use tokio_stream::{StreamExt, wrappers::UnboundedReceiverStream};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

#[derive(Clone)]
struct AppState {
    mirror: MirrorHandle,
    controller: Option<Arc<dyn MirrorControl>>,
    queries: QueryService,
}

enum BackendGuard {
    Embedded(MirrorRuntime),
    JetStream(JetStreamConsumerGuard),
}

struct JetStreamConsumerGuard {
    handle: JoinHandle<()>,
}

impl Drop for JetStreamConsumerGuard {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl Drop for BackendGuard {
    fn drop(&mut self) {
        match self {
            BackendGuard::Embedded(runtime) => {
                let _ = runtime;
            }
            BackendGuard::JetStream(guard) => {
                let _ = guard;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let metrics_addr = SocketAddr::from(([127, 0, 0, 1], 9100));
    match engine_v3::init_prometheus_metrics(metrics_addr) {
        Ok(()) => info!(%metrics_addr, "sim-api exporting Prometheus metrics"),
        Err(err) => warn!(%metrics_addr, error = %err, "failed to install Prometheus recorder"),
    }

    let schema = JetStreamSchema::default();
    let jetstream_url = env::var("SIM_JETSTREAM_URL").ok().filter(|s| !s.is_empty());
    let remote_backend = env_truthy("SIM_REMOTE_BACKEND");

    let (mirror, controller_opt, _backend_guard) = if remote_backend {
        let url = match jetstream_url.clone() {
            Some(url) => url,
            None => {
                error!("SIM_REMOTE_BACKEND requires SIM_JETSTREAM_URL to be set");
                return;
            }
        };

        match start_remote_backend(schema.clone(), url.clone()).await {
            Ok((mirror, controller, guard)) => {
                info!(%url, "sim-api consuming JetStream digests");
                (mirror, Some(controller), BackendGuard::JetStream(guard))
            }
            Err(err) => {
                error!(error = %err, %url, "failed to connect to JetStream remote backend");
                return;
            }
        }
    } else {
        match start_embedded_backend(schema.clone()) {
            Ok((mirror, controller, guard)) => (mirror, Some(controller), guard),
            Err(err) => {
                error!(error = %err, "failed to start embedded mirror runtime");
                return;
            }
        }
    };

    let queries = QueryService::new(mirror.clone());
    let app_state = AppState { mirror: mirror.clone(), controller: controller_opt, queries };

    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .route("/latest", get(latest))
        .route("/metrics", get(metrics))
        .route("/highlights", get(highlights))
        .route("/simulation/status", get(simulation_status))
        .route("/dashboard", get(dashboard))
        .route("/delta", get(latest_delta))
        .route("/stream", get(stream_ws))
        .route("/stream/sse", get(stream_sse))
        .route("/control/status", get(control_status))
        .route("/control/pause", post(control_pause))
        .route("/control/resume", post(control_resume))
        .route("/control/step", post(control_step))
        .route("/control/interval", post(control_interval))
        .layer(cors)
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8060));
    let listener = TcpListener::bind(addr).await.expect("bind port 8060");
    info!(%addr, "sim-api listening");

    if let Err(err) = axum::serve(listener, app.into_make_service()).await {
        error!(error = %err, "sim-api server terminated");
    }
}

fn start_embedded_backend(
    schema: JetStreamSchema,
) -> Result<(MirrorHandle, Arc<dyn MirrorControl>, BackendGuard), MirrorError> {
    let runtime = MirrorRuntime::start_from_config("config/config.toml")?;
    let mirror = runtime.mirror_handle();
    let controller_arc: Arc<dyn MirrorControl> = Arc::new(runtime.controller());

    if env_truthy("SIM_PUBLISH_JETSTREAM") {
        let mut config = JetStreamConfig::default();
        if let Some(url) = env::var("SIM_JETSTREAM_URL").ok().filter(|s| !s.is_empty()) {
            config.url = url;
        }
        if let Err(err) = runtime.attach_jetstream_with_schema(config, schema.clone()) {
            warn!(error = %err, "failed to attach JetStream publisher");
        }
    }

    let broker = InMemoryBroker::default();
    let broker_consumer = broker.consumer();
    mirror.attach_publisher(broker.publisher());
    spawn_broker_monitor(broker_consumer);

    Ok((mirror, controller_arc, BackendGuard::Embedded(runtime)))
}

async fn start_remote_backend(
    schema: JetStreamSchema, url: String,
) -> Result<(MirrorHandle, Arc<dyn MirrorControl>, JetStreamConsumerGuard), JetStreamError> {
    let mirror = MirrorHandle::new();
    let guard = spawn_jetstream_consumer(mirror.clone(), schema.clone(), url.clone()).await?;

    let mut control_config = JetStreamConfig::default();
    control_config.url = url;
    control_config.connection_name = format!("sim-api-control-{}", process::id());
    let controller = RemoteMirrorController::connect(control_config, schema)?;

    Ok((mirror, Arc::new(controller), guard))
}

async fn spawn_jetstream_consumer(
    mirror: MirrorHandle, schema: JetStreamSchema, url: String,
) -> Result<JetStreamConsumerGuard, JetStreamError> {
    let client = async_nats::connect(url.clone())
        .await
        .map_err(|err| JetStreamError::new(format!("failed to connect to NATS at {}: {err}", url)))?;
    let context = jetstream::new(client);
    let stream_name = schema.stream_name.to_string();
    let digest_subject = schema.digest_subject.to_string();
    let delta_subject = schema.delta_subject.to_string();
    let retention_messages = schema.retention_messages;

    let stream = match context.get_stream(&stream_name).await {
        Ok(stream) => stream,
        Err(_) => {
            let mut config = jetstream::stream::Config::default();
            config.name = stream_name.clone();
            config.subjects = vec![digest_subject.clone(), delta_subject];
            config.max_messages = i64::try_from(retention_messages).unwrap_or(i64::MAX);
            context.get_or_create_stream(config).await.map_err(|err| {
                JetStreamError::new(format!("failed to create JetStream stream {}: {err}", stream_name))
            })?
        }
    };

    let consumer_name = format!("sim_api_digest_{}", std::process::id());
    let deliver_subject = format!("{}.deliver.{}", digest_subject, std::process::id());

    let consumer_config = push::Config {
        deliver_subject: deliver_subject.clone(),
        durable_name: Some(consumer_name.clone()),
        name: Some(consumer_name.clone()),
        deliver_policy: DeliverPolicy::Last,
        ack_policy: consumer::AckPolicy::Explicit,
        filter_subject: digest_subject.clone(),
        ..Default::default()
    };

    let consumer = stream
        .get_or_create_consumer(&consumer_name, consumer_config)
        .await
        .map_err(|err| JetStreamError::new(format!("failed to create JetStream consumer {}: {err}", consumer_name)))?;

    let handle = tokio::spawn(async move {
        match consumer.messages().await {
            Ok(mut messages) => {
                while let Some(message_result) = messages.next().await {
                    match message_result {
                        Ok(message) => match serde_json::from_slice::<StateDigest>(&message.payload) {
                            Ok(digest) => {
                                mirror.publish(digest);
                                if let Err(err) = message.ack().await {
                                    warn!(error = %err, "failed to ack JetStream digest message");
                                }
                            }
                            Err(err) => {
                                warn!(error = %err, "failed to deserialize JetStream digest");
                                if let Err(err) = message.ack().await {
                                    warn!(error = %err, "failed to ack after deserialize failure");
                                }
                            }
                        },
                        Err(err) => warn!(error = %err, "JetStream consumer error"),
                    }
                }
            }
            Err(err) => error!(error = %err, "failed to iterate JetStream consumer messages"),
        }
    });

    Ok(JetStreamConsumerGuard { handle })
}

fn spawn_broker_monitor(consumer: InMemoryConsumer) {
    task::spawn_blocking(move || {
        while let Ok(snapshot) = consumer.recv() {
            let latency_ms = Utc::now()
                .signed_duration_since(snapshot.digest.timings.generated_at)
                .num_microseconds()
                .unwrap_or_default() as f64
                / 1_000.0;
            histogram!("mirror.broker.delivery_ms", latency_ms, "consumer" => "in-memory");
            counter!("mirror.broker.deliveries", 1, "consumer" => "in-memory");
        }
    });
}

fn env_truthy(key: &str) -> bool {
    matches!(
        env::var(key).ok().map(|v| v.to_ascii_lowercase()),
        Some(ref v) if matches!(v.as_str(), "1" | "true" | "yes" | "on")
    )
}

async fn health() -> Json<&'static str> {
    Json("ok")
}

async fn latest(State(state): State<AppState>) -> Json<StateDigest> {
    Json(state.queries.latest_snapshot().digest.as_ref().clone())
}

async fn metrics(State(state): State<AppState>) -> Json<RiskDigest> {
    Json(state.queries.latest_snapshot().digest.risk.clone())
}

async fn highlights(State(state): State<AppState>) -> Json<Vec<sim_mirror::DigestEvent>> {
    Json(state.queries.latest_snapshot().digest.highlights.clone())
}

async fn simulation_status(State(state): State<AppState>) -> Json<StatusDigest> {
    Json(state.queries.status())
}

async fn dashboard(State(state): State<AppState>) -> Json<DashboardDto> {
    Json(state.queries.dashboard())
}

async fn latest_delta(State(state): State<AppState>) -> Json<Option<StateDelta>> {
    Json(state.queries.latest_delta())
}

async fn stream_ws(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| stream_socket(socket, state.mirror.clone()))
}

async fn stream_socket(mut socket: WebSocket, mirror: MirrorHandle) {
    let mut rx_async = subscribe_async(mirror);

    while let Some(update) = rx_async.recv().await {
        let latency_ms = compute_latency(&update);
        let payload = match serde_json::to_string(&update) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("serialize error: {err}");
                counter!("sim_api.stream.serialization_failures", 1, "channel" => "ws");
                continue;
            }
        };

        record_stream_metrics("ws", &update, payload.len(), latency_ms);

        if socket.send(Message::Text(payload.into())).await.is_err() {
            counter!("sim_api.stream.disconnects", 1, "channel" => "ws");
            break;
        }
    }
}

async fn stream_sse(State(state): State<AppState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx_async = subscribe_async(state.mirror.clone());
    let stream = UnboundedReceiverStream::new(rx_async).map(|update| {
        let latency_ms = compute_latency(&update);
        let payload = serde_json::to_string(&update).unwrap_or_else(|err| {
            eprintln!("serialize error: {err}");
            counter!("sim_api.stream.serialization_failures", 1, "channel" => "sse");
            "{}".to_string()
        });
        record_stream_metrics("sse", &update, payload.len(), latency_ms);
        Ok(Event::default().data(payload))
    });
    Sse::new(stream)
}

fn compute_latency(update: &StreamUpdate) -> f64 {
    let timestamp = match update {
        StreamUpdate::Snapshot(digest) => digest.timings.generated_at,
        StreamUpdate::Delta(delta) => delta.generated_at,
    };
    Utc::now().signed_duration_since(timestamp).num_microseconds().unwrap_or_default() as f64 / 1_000.0
}

fn record_stream_metrics(channel: &'static str, update: &StreamUpdate, payload_len: usize, latency_ms: f64) {
    let kind = match update {
        StreamUpdate::Snapshot(_) => "snapshot",
        StreamUpdate::Delta(_) => "delta",
    };
    histogram!("sim_api.stream.latency_ms", latency_ms, "channel" => channel, "kind" => kind);
    histogram!(
        "sim_api.stream.payload_bytes",
        payload_len as f64,
        "channel" => channel,
        "kind" => kind
    );
    counter!("sim_api.stream.messages", 1, "channel" => channel, "kind" => kind);
}

async fn control_status(State(state): State<AppState>) -> Result<Json<MirrorControlStatus>, StatusCode> {
    state.controller.as_ref().map(|ctrl| Json(ctrl.status())).ok_or(StatusCode::SERVICE_UNAVAILABLE)
}

async fn control_pause(State(state): State<AppState>) -> Result<Json<MirrorControlStatus>, StatusCode> {
    measure_control(state.controller.as_ref(), "pause", |ctrl| ctrl.pause())
}

async fn control_resume(State(state): State<AppState>) -> Result<Json<MirrorControlStatus>, StatusCode> {
    measure_control(state.controller.as_ref(), "resume", |ctrl| ctrl.resume())
}

async fn control_step(State(state): State<AppState>) -> Result<Json<MirrorControlStatus>, StatusCode> {
    measure_control(state.controller.as_ref(), "step", |ctrl| ctrl.step())
}

#[derive(Deserialize)]
struct IntervalRequest {
    millis: u64,
}

async fn control_interval(
    State(state): State<AppState>, Json(payload): Json<IntervalRequest>,
) -> Result<Json<MirrorControlStatus>, StatusCode> {
    if payload.millis == 0 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let duration = Duration::from_millis(payload.millis);
    measure_control(state.controller.as_ref(), "interval", |ctrl| ctrl.set_interval(duration))
}

fn measure_control<F>(
    controller: Option<&Arc<dyn MirrorControl>>, operation: &'static str, action: F,
) -> Result<Json<MirrorControlStatus>, StatusCode>
where
    F: FnOnce(&dyn MirrorControl) -> Result<(), ControlError>,
{
    let controller = controller.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let start = Instant::now();
    if let Err(err) = action(controller.as_ref()) {
        let status = match err {
            ControlError::Disconnected => StatusCode::SERVICE_UNAVAILABLE,
            ControlError::Timeout => StatusCode::GATEWAY_TIMEOUT,
            ControlError::Transport(_) => StatusCode::BAD_GATEWAY,
        };
        return Err(status);
    }
    let status = controller.status();
    let elapsed_ms = start.elapsed().as_secs_f64() * 1_000.0;
    histogram!("sim_api.control.latency_ms", elapsed_ms, "operation" => operation);
    counter!("sim_api.control.invocations", 1, "operation" => operation);
    Ok(Json(status))
}

fn subscribe_async(mirror: MirrorHandle) -> mpsc::UnboundedReceiver<StreamUpdate> {
    let rx = mirror.subscribe();
    let (tx, rx_async) = mpsc::unbounded_channel();

    task::spawn_blocking(move || {
        let mut first = true;
        for snapshot in rx.iter() {
            let update = if first || snapshot.delta.is_none() {
                first = false;
                StreamUpdate::Snapshot(snapshot.digest.as_ref().clone())
            } else {
                StreamUpdate::Delta(snapshot.delta.clone().unwrap())
            };

            if tx.send(update).is_err() {
                break;
            }
        }
    });

    rx_async
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum StreamUpdate {
    Snapshot(StateDigest),
    Delta(StateDelta),
}
