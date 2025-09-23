use std::{convert::Infallible, net::SocketAddr, time::Duration};

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
use futures_core::stream::Stream;
use serde::Deserialize;
use sim_mirror::{
    ControlError, DigestEvent, DigestMetrics, MirrorControlStatus, MirrorController, MirrorHandle, MirrorRuntime,
    StateDigest,
};
use tokio::{net::TcpListener, sync::mpsc, task};
use tokio_stream::{StreamExt, wrappers::UnboundedReceiverStream};
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct AppState {
    mirror: MirrorHandle,
    controller: MirrorController,
}

#[tokio::main]
async fn main() {
    let runtime = MirrorRuntime::start_from_config("config/config.toml").expect("failed to start simulation mirror");
    let controller = runtime.controller();
    let app_state = AppState { mirror: runtime.mirror_handle(), controller: controller.clone() };

    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .route("/latest", get(latest))
        .route("/metrics", get(metrics))
        .route("/highlights", get(highlights))
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
    println!("sim-api listening on http://{addr}");

    axum::serve(listener, app.into_make_service()).await.expect("server exit");

    drop(runtime);
}

async fn health() -> Json<&'static str> {
    Json("ok")
}

async fn latest(State(state): State<AppState>) -> Json<StateDigest> {
    Json(state.mirror.latest().as_ref().clone())
}

async fn metrics(State(state): State<AppState>) -> Json<DigestMetrics> {
    Json(state.mirror.latest().metrics.clone())
}

async fn highlights(State(state): State<AppState>) -> Json<Vec<DigestEvent>> {
    Json(state.mirror.latest().highlights.clone())
}

async fn stream_ws(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| stream_socket(socket, state.mirror.clone()))
}

async fn stream_socket(mut socket: WebSocket, mirror: MirrorHandle) {
    let mut rx_async = subscribe_async(mirror);

    while let Some(digest) = rx_async.recv().await {
        let payload = match serde_json::to_string(&digest) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("serialize error: {err}");
                continue;
            }
        };

        if socket.send(Message::Text(payload.into())).await.is_err() {
            break;
        }
    }
}

async fn stream_sse(State(state): State<AppState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx_async = subscribe_async(state.mirror.clone());
    let stream = UnboundedReceiverStream::new(rx_async).map(|digest| {
        let payload = serde_json::to_string(&digest).unwrap_or_else(|err| {
            eprintln!("serialize error: {err}");
            String::from("{}")
        });
        Ok(Event::default().data(payload))
    });
    Sse::new(stream)
}

fn subscribe_async(mirror: MirrorHandle) -> mpsc::UnboundedReceiver<StateDigest> {
    let rx = mirror.subscribe();
    let (tx, rx_async) = mpsc::unbounded_channel();

    task::spawn_blocking(move || {
        for snapshot in rx.iter() {
            if tx.send(snapshot.as_ref().clone()).is_err() {
                break;
            }
        }
    });

    rx_async
}

async fn control_status(State(state): State<AppState>) -> Json<MirrorControlStatus> {
    Json(state.controller.status())
}

async fn control_pause(State(state): State<AppState>) -> Result<Json<MirrorControlStatus>, StatusCode> {
    map_control(&state.controller, state.controller.pause())
}

async fn control_resume(State(state): State<AppState>) -> Result<Json<MirrorControlStatus>, StatusCode> {
    map_control(&state.controller, state.controller.resume())
}

async fn control_step(State(state): State<AppState>) -> Result<Json<MirrorControlStatus>, StatusCode> {
    map_control(&state.controller, state.controller.step())
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
    map_control(&state.controller, state.controller.set_interval(duration))
}

fn map_control(
    controller: &MirrorController, result: Result<(), ControlError>,
) -> Result<Json<MirrorControlStatus>, StatusCode> {
    result.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(controller.status()))
}
