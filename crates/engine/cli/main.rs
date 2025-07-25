use axum::{Json, Router, extract::State, http::Method, routing::get};
#[allow(unused)]
use engine::SimState;
use engine::*;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use uuid::Uuid;
use execution::*;

#[tokio::main]
async fn main() {
    let ss = SimState::default();

    let state = Arc::new(RwLock::new(ss));

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any) // Allow any origin
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers(tower_http::cors::Any); // Allow any headers

    let app = Router::new()
        .route(
            "/health",
            get(|| async { "Welcome to the Ravelin Engine!" }),
        )
        .route("/state", get(get_state))
        .route("/clear", get(clear_state))
        .route("/make_bank", get(make_bank))
        .route("/make_consumer", get(make_consumer))
        .route("/make_firm", get(make_firm))
        .route("/consumer_action", get(consumer_action))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8070")
        .await
        .unwrap();

    println!("Server running on http://127.0.0.1:8070");
    println!("⚠️  CORS is configured to allow ANY origin - development only!");

    axum::serve(listener, app).await.unwrap();
}

async fn get_state(State(state): State<Arc<RwLock<SimState>>>) -> Json<Res> {
    let state_guard = state.read().await;
    Json(Res {
        stats: Some(Stats {
            m0: state_guard.financial_system.m0(),
            m1: state_guard.financial_system.m1(),
            m2: state_guard.financial_system.m2(),
        }),
        messages: None,
        state: state_guard.clone(),
    })
}

async fn make_bank(State(state): State<Arc<RwLock<SimState>>>) -> Json<String> {
    let mut state_guard = state.write().await;
    let mut factory = AgentFactory {
        ss: &mut state_guard,
        rng: &mut StdRng::from_os_rng(),
    };
    let bank = factory.create_bank("Test Bank".to_string(), 230.0, 50.0);
    Json(json!(bank).to_string())
}

async fn make_firm(State(state): State<Arc<RwLock<SimState>>>) -> Json<String> {
    let mut state_guard = state.write().await;
    let mut factory = AgentFactory {
        ss: &mut state_guard,
        rng: &mut StdRng::from_os_rng(),
    };
    let firm = factory.create_firm("Test Firm".to_string(), AgentId(Uuid::new_v4()));
    Json(json!(firm).to_string())
}
async fn clear_state(State(state): State<Arc<RwLock<SimState>>>) -> Json<String> {
    let mut state_guard = state.write().await;
    *state_guard = SimState::default();
    Json("State cleared".to_string())
}


async fn consumer_action(State(state): State<Arc<RwLock<SimState>>>) -> Json<String> {
    let mut state_guard = state.write().await;

    if state_guard.consumers.is_empty() {
        return Json(
            json!({
                "error": "No consumers found. Create a consumer first."
            })
            .to_string(),
        );
    }

    let consumer = state_guard.consumers.first().unwrap().clone();
    let mut messages = Vec::new();
    
    let decision = consumer.decide(&state_guard.financial_system, &mut StdRng::from_os_rng());
    let actions = consumer.act(&decision);
    messages.push(format!(
        "Step 1: Consumer {} decided to spend ${:.2} and save ${:.2}",
        consumer.snip_id(), decision.spend_amount, decision.save_amount
    ));
    for (i, action) in actions.iter().enumerate() {
        messages.push(format!(
            "Step 2.{}: [C{} {:?}] Amount: ${:.2}",
            i, consumer.snip_id(), action.name(), action.amount()
        ));
    }
    let mut sim_actions = Vec::new();
    for action in actions {
        let sa = agent_action_to_sim_actions(&consumer.id, &action, &state_guard);
        sim_actions.extend(sa);
    }
    for action in sim_actions {
        let er = TransactionExecutor::execute_action(&action, &mut state_guard);
        for (i, effect) in er.effects.iter().enumerate() {
            messages.push(format!(
                "Step 3.{}: [C{}] -> Effect: {}",
                i, consumer.snip_id(), effect.name()
            ));
        }
        let res = TransactionExecutor::apply_effects(&er.effects, &mut state_guard);
        if let Err(e) = res {
            messages.push(format!(
                "Error applying effects: {}",
                e.to_string()
            ));
        } else {
            messages.push(format!(
                "Step 4: Effects applied successfully for C{}",
                consumer.snip_id()
            ));
        }
    }
    messages.push(format!(
        "Step 5.1: Consumer {} now has ${:.2} in cash and ${:.2} in deposits",
        consumer.snip_id(),
        consumer.get_cash_holdings(&state_guard.financial_system),
        consumer.get_deposits(&state_guard.financial_system)
    ));
    messages.push(format!(
        "Step 5.2: Bank {} now has ${:.2} in total liabilities",
        &consumer.bank_id.0.to_string()[0..4], &state_guard.financial_system.get_total_liabilities(&consumer.bank_id)
    ));

    let res = Res {
        messages: Some(messages),
        stats: Some(Stats {
            m0: state_guard.financial_system.m0(),
            m1: state_guard.financial_system.m1(),
            m2: state_guard.financial_system.m2(),
        }),
        state: state_guard.clone(),
    };

    Json(json!(res).to_string())
}

async fn make_consumer(State(state): State<Arc<RwLock<SimState>>>) -> Json<String> {
    let mut state_guard = state.write().await;

    if state_guard.financial_system.commercial_banks.is_empty() {
        return Json(
            json!({
                "error": "No banks found. Create a bank first."
            })
            .to_string(),
        );
    }

    let bank_id = state_guard
        .financial_system
        .commercial_banks
        .keys()
        .next()
        .unwrap()
        .clone();

    let mut factory = AgentFactory {
        ss: &mut state_guard,
        rng: &mut StdRng::from_os_rng(),
    };

    let consumer = factory.create_consumer(
        bank_id,
        Box::new(BasicDecisionModel {
            propensity_to_consume: 0.0,
        }), // 70% spending rate
    );

    Json(json!(consumer).to_string())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Res {
    pub messages: Option<Vec<String>>,
    pub stats: Option<Stats>,
    pub state: SimState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Stats {
    pub m0: f64, 
    pub m1: f64, 
    pub m2: f64, 
}