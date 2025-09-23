use std::{net::SocketAddr, time::Duration};

use axum::{
    extract::{State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use sim_mirror::{testing, MirrorHandle, TickDigest};
use tokio::{net::TcpListener, sync::mpsc, task};

#[derive(Clone)]
struct AppState {
    mirror: MirrorHandle,
}

#[tokio::main]
async fn main() {
    let mirror = MirrorHandle::new();
    let _publisher = testing::spawn_counter_publisher(mirror.clone(), Duration::from_millis(200));

    let app_state = AppState { mirror: mirror.clone() };

    let app = Router::new()
        .route("/health", get(health))
        .route("/latest", get(latest))
        .route("/stream", get(stream))
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8062));
    let listener = TcpListener::bind(addr).await.expect("bind port 8060");
    println!("sim-api listening on http://{addr}");

    axum::serve(listener, app.into_make_service()).await.expect("server exit");
}

async fn health() -> Json<&'static str> {
    Json("ok")
}

async fn latest(State(state): State<AppState>) -> Json<TickDigest> {
    Json(state.mirror.latest().as_ref().clone())
}

async fn stream(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| stream_socket(socket, state.mirror.clone()))
}

async fn stream_socket(mut socket: WebSocket, mirror: MirrorHandle) {
    let rx = mirror.subscribe();
    let (tx, mut rx_async) = mpsc::unbounded_channel::<TickDigest>();

    task::spawn_blocking(move || {
        for snapshot in rx.iter() {
            let digest = snapshot.as_ref().clone();
            if tx.send(digest).is_err() {
                break;
            }
        }
    });

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
