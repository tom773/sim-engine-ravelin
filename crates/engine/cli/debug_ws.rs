use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::ConnectInfo,
    response::IntoResponse,
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use engine::debug_bus::subscribe;
async fn ws_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| client_loop(addr, socket))
}

async fn client_loop(_addr: SocketAddr, mut socket: WebSocket) {
    let mut rx = subscribe();

    loop {
        match rx.recv().await {
            Ok(json) => {
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                continue;
            }
            Err(_) => break,
        }
    }
}

pub async fn serve_ws(addr: &str) -> anyhow::Result<()> {
    let app = Router::new().route("/ws/debug", get(ws_handler));
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}
