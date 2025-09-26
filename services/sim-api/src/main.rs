use async_nats::jetstream::{
    self,
    consumer::{self, DeliverPolicy, push},
};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::{Json, Router, extract::State, routing::get};
use sim_mirror::*;
use std::{convert::TryFrom, env, net::SocketAddr, process, sync::Arc};
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_stream::StreamExt;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

mod wt;

#[derive(Clone)]
struct AppState {
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

    let schema = JetStreamSchema::default();
    let jetstream_url = env::var("SIM_JETSTREAM_URL").ok().filter(|s| !s.is_empty());
    let remote_backend = env_truthy("SIM_REMOTE_BACKEND");

    let (mirror, controller, _backend_guard) = if remote_backend {
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
                (mirror, controller, guard)
            }
            Err(err) => {
                error!(error = %err, %url, "failed to connect to JetStream remote backend");
                return;
            }
        }
    } else {
        match start_embedded_backend(schema.clone()) {
            Ok((mirror, controller, guard)) => (mirror, controller, guard),
            Err(err) => {
                error!(error = %err, "failed to start embedded mirror runtime");
                return;
            }
        }
    };

    let queries = QueryService::new(mirror.clone());
    let app_state = AppState { queries };

    // Start the WebTransport broadcast server alongside the HTTP API.
    let _wt_handle = wt::spawn_server(mirror.clone(), controller.clone());

    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .route("/state", get(state_snapshot))
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
) -> Result<(MirrorHandle, Arc<dyn MirrorControl + Send + Sync>, BackendGuard), MirrorError> {
    let runtime = MirrorRuntime::start_from_config("config/config.toml")?;
    let mirror = runtime.mirror_handle();
    let controller: Arc<dyn MirrorControl + Send + Sync> = Arc::new(runtime.controller());

    if env_truthy("SIM_PUBLISH_JETSTREAM") {
        let mut config = JetStreamConfig::default();
        if let Some(url) = env::var("SIM_JETSTREAM_URL").ok().filter(|s| !s.is_empty()) {
            config.url = url;
        }
        if let Err(err) = runtime.attach_jetstream_with_schema(config, schema) {
            warn!(error = %err, "failed to attach JetStream publisher");
        }
    }

    Ok((mirror, controller, BackendGuard::Embedded(runtime)))
}

async fn start_remote_backend(
    schema: JetStreamSchema, url: String,
) -> Result<(MirrorHandle, Arc<dyn MirrorControl + Send + Sync>, BackendGuard), JetStreamError> {
    let mirror = MirrorHandle::new();
    let guard = spawn_jetstream_consumer(mirror.clone(), schema.clone(), url.clone()).await?;

    let mut control_config = JetStreamConfig::default();
    control_config.url = url;
    control_config.connection_name = format!("sim-api-control-{}", process::id());
    let controller = RemoteMirrorController::connect(control_config, schema)?;
    let controller: Arc<dyn MirrorControl + Send + Sync> = Arc::new(controller);

    Ok((mirror, controller, BackendGuard::JetStream(guard)))
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

    let consumer_name = format!("sim_api_digest_{}", process::id());
    let deliver_subject = format!("{}.deliver.{}", digest_subject, process::id());

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

fn env_truthy(key: &str) -> bool {
    matches!(
        env::var(key).ok().map(|v| v.to_ascii_lowercase()),
        Some(ref v) if matches!(v.as_str(), "1" | "true" | "yes" | "on")
    )
}

async fn health() -> Json<&'static str> {
    Json("ok")
}

async fn state_snapshot(State(state): State<AppState>) -> (HeaderMap, Vec<u8>) {
    let snapshot = state.queries.latest_snapshot();

    let buffer = rmp_serde::to_vec_named(&snapshot.digest.as_ref()).unwrap(); // ✓ Already using MessagePack
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));

    (headers, buffer)
}
