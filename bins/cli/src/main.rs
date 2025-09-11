use axum::{
    Router,
    extract::{
        Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
};
use engine_v3::*;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

mod bus;
use bus::*;
struct AppState {
    engine: Arc<parking_lot::RwLock<SimulationEngine>>,
    bus: EventsBus,
}

#[derive(Serialize)]
struct TickResponse {
    message: String,
    tick_number: u32,
    current_date: String,
}

#[derive(Deserialize)]
struct AgentQuery {
    #[serde(rename = "type")]
    agent_type: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let scenario = Scenario::from_toml_str(include_str!("../../../config/config.toml"))
        .expect("Failed to load scenario");
    let engine = Arc::new(parking_lot::RwLock::new(scenario.initialize_engine()));

    let bus = EventsBus::new(500); // keep last 500 ticks
    {
        let mut eng = engine.write();
        let mut rng = StdRng::from_os_rng();
        let tick = eng.state.ticknum;
        let date = eng.state.current_date;
        eng.state.financial_system.attach_default_pricing_feeds(date);
        let (_res, events) = eng.run_tick(&mut rng);

        bus.push_tick(tick, date, events);
    }
    let app_state = Arc::new(AppState {
        engine: engine.clone(),
        bus: bus.clone(),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let app = Router::new()
        .route("/api/live/stream", get(ws_upgrade))
        .route("/api/live/history/ticks", get(list_ticks))
        .route("/api/live/history/ticks/{n}/events", get(get_tick_events))
        .nest("/api/v1", api_v1_router())
        .layer(cors)
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8060));
    println!("Axum server listening on http://127.0.0.1:8060");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

fn api_v1_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/simulation/status", get(get_simulation_status))
        .route("/simulation/tick", post(tick_handler))
        .route("/agents", get(get_agents_list))
        .route("/agents/{id}", get(get_agent_detail))
        .route("/markets", get(get_markets_list))
        .route("/markets/catalog", get(get_market_catalog))
        .route("/instruments", get(get_instrument_registry))
        .route("/markets/infrastructure", get(get_infra))
        .route("/markets/{market_id}", get(get_market_detail))
        .route("/markets/{market_id}/history", get(get_market_history))
        .route("/markets/overview", get(get_market_overview))
        .route("/markets/credit/registry", get(get_credit_registry))
        .route("/exchange", get(get_exchange))
        .route("/history/ticks", get(list_ticks))
        .route("/history/ticks/{tick_number}", get(get_tick_detail))
}

async fn get_simulation_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let query_service = QueryService::new(state.engine.clone());
    match query_service.get_status_data() {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}

async fn tick_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut eng = state.engine.write();

    if eng.state.ticknum >= eng.state.config.iterations {
        return (
            StatusCode::BAD_REQUEST,
            Json(TickResponse {
                message: "Simulation has already finished.".to_string(),
                tick_number: eng.state.ticknum,
                current_date: eng.state.current_date.format("%Y-%m-%d").to_string(),
            }),
        )
            .into_response();
    }

    let tick_to_report = eng.state.ticknum;
    let date_to_report = eng.state.current_date;

    let mut rng = StdRng::from_os_rng();
    let (_te, events) = eng.run_tick(&mut rng);

    drop(eng);
    state.bus.push_tick(tick_to_report, date_to_report, events);

    (
        StatusCode::OK,
        Json(TickResponse {
            message: "Tick executed successfully.".to_string(),
            tick_number: tick_to_report,
            current_date: date_to_report.format("%Y-%m-%d").to_string(),
        }),
    )
        .into_response()
}

async fn get_agents_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AgentQuery>,
) -> impl IntoResponse {
    let query_service = QueryService::new(state.engine.clone());
    match query_service.get_agents_summary(query.agent_type) {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}
async fn get_agent_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let query_service = QueryService::new(state.engine.clone());
    match query_service.get_agent_detail(id) {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}

async fn get_markets_list(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let query_service = QueryService::new(state.engine.clone());
    match query_service.get_markets_summary() {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}

async fn get_market_catalog(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let query_service = QueryService::new(state.engine.clone());
    match query_service.get_catalog() {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}
async fn get_market_overview(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let query_service = QueryService::new(state.engine.clone());
    match query_service.get_market_overview() {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}
async fn get_market_detail(
    State(state): State<Arc<AppState>>,
    Path(market_id): Path<String>,
) -> impl IntoResponse {
    let query_service = QueryService::new(state.engine.clone());
    match query_service.get_market_detail(&market_id) {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}

async fn get_market_history(
    State(state): State<Arc<AppState>>,
    Path(market_id): Path<String>,
) -> impl IntoResponse {
    let query_service = QueryService::new(state.engine.clone());
    match query_service.get_market_history(&market_id) {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}

async fn get_tick_detail(
    State(state): State<Arc<AppState>>,
    Path(tick_number): Path<u32>,
) -> impl IntoResponse {
    let query_service = QueryService::new(state.engine.clone());
    match query_service.get_tick_detail(tick_number) {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}

async fn get_exchange(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let query_service = QueryService::new(state.engine.clone());
    match query_service.get_exchange() {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}

async fn get_instrument_registry(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let query_service = QueryService::new(state.engine.clone());
    match query_service.get_instrument_registry() {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}

async fn list_ticks(State(state): State<Arc<AppState>>) -> Json<Vec<u32>> {
    let v = state
        .bus
        .latest_n(500)
        .into_iter()
        .map(|(t, _, _)| t)
        .collect();
    Json(v)
}
async fn get_tick_events(
    State(state): State<Arc<AppState>>,
    Path(n): Path<u32>,
) -> Result<Json<Vec<SimEvent>>, StatusCode> {
    match state.bus.get(n) {
        Some(v) => Ok(Json(Arc::unwrap_or_clone(v))),
        None => Err(StatusCode::NOT_FOUND),
    }
}
async fn get_infra(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let query_service = QueryService::new(state.engine.clone());
    match query_service.get_financial_infrastructure_state() {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}
async fn ws_upgrade(State(state): State<Arc<AppState>>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_conn(socket, state))
}
async fn get_credit_registry(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let query_service = QueryService::new(state.engine.clone());
    match query_service.get_credit_registry() {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}
async fn ws_conn(mut socket: WebSocket, state: Arc<AppState>) {
    for (tick, date, evs) in state.bus.latest_n(3) {
        let msg = serde_json::to_string(&ServerEvent::Tick {
            tick,
            date,
            events: Arc::unwrap_or_clone(evs),
        })
        .unwrap();
        if socket.send(Message::Text(msg.into())).await.is_err() {
            return;
        }
    }

    let mut rx = state.bus.tx.subscribe();
    loop {
        tokio::select! {
            Ok(evt) = rx.recv() => {
                let msg = serde_json::to_string(&evt).unwrap();
                if socket.send(Message::Text(msg.into())).await.is_err() { break; }
            }
            Some(Ok(Message::Close(_))) = socket.recv() => break,
            else => break,
        }
    }
}