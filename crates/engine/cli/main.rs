#[allow(unused)]
use engine::SimState;
use axum::{
    http::Method,
    routing::get,
    Json, Router,
    extract::{State},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use shared::*;
use serde_json::json;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let ss = SimState::default();
    
    let state = Arc::new(RwLock::new(ss));
    
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)  // Allow any origin
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers(tower_http::cors::Any);  // Allow any headers
    
    let app = Router::new()
        .route("/health", get(|| async { "Welcome to the Ravelin Engine!" }))
        .route("/state", get(get_state))
        .route("/clear", get(clear_state))
        .route("/make_bank", get(make_bank))
        .route("/do_tx", get(do_tx))
        .with_state(state)
        .layer(cors);
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8070")
        .await
        .unwrap();
    
    println!("Server running on http://127.0.0.1:8070");
    println!("⚠️  CORS is configured to allow ANY origin - development only!");
    
    axum::serve(listener, app).await.unwrap();
}

async fn get_state(
    State(state): State<Arc<RwLock<SimState>>>,
) -> Json<SimState> {
    let state_guard = state.read().await;
    Json(state_guard.clone())
}

async fn make_bank(
    State(state): State<Arc<RwLock<SimState>>>,
) -> Json<String> {
    let mut state_guard = state.write().await;
    
    let bank = Bank::new("RBK Bancorp".to_string(), 200.0, -200.0);
    let bank2 = Bank::new("JUB Bank".to_string(), 220.0, -171.0);
    
    state_guard.financial_system.balance_sheets.insert(
        bank.id.clone(), 
        BalanceSheet::new(bank.id.clone())
    );
    state_guard.financial_system.balance_sheets.insert(
        bank2.id.clone(), 
        BalanceSheet::new(bank2.id.clone())
    );
    
    state_guard.financial_system.commercial_banks.insert(bank.id.clone(), bank.clone());
    state_guard.financial_system.commercial_banks.insert(bank2.id.clone(), bank2.clone());
    
    let mbs = FinancialInstrument {
        id: InstrumentId(Uuid::new_v4()),
        creditor: bank.id.clone(),
        debtor: bank2.id.clone(),
        principal: 100.0,
        interest_rate: 0.05,
        maturity: Some(365),
        originated_date: 0,
        instrument_type: InstrumentType::Bond { 
            bond_type: BondType::Corporate { spread: 323.0 }, 
            coupon_rate: 4.00, 
            face_value: 100000.0, 
            rating: CreditRating::BB 
        },
    };
    
    state_guard.financial_system.create_instrument(mbs).unwrap();
    
    let bank_bs = state_guard.financial_system.balance_sheets.get(&bank.id).unwrap();
    let bank2_bs = state_guard.financial_system.balance_sheets.get(&bank2.id).unwrap();
    
    println!("Bank 1 assets: ${:.2}", bank_bs.total_assets());
    println!("Bank 2 liabilities: ${:.2}", bank2_bs.total_liabilities());
    
    Json(json!(bank).to_string())
}

async fn clear_state(
    State(state): State<Arc<RwLock<SimState>>>,
) -> Json<String> {
    let mut state_guard = state.write().await;
    *state_guard = SimState::default();
    Json("State cleared".to_string())
}

async fn do_tx(
    State(state): State<Arc<RwLock<SimState>>>,
) -> Json<String> {
    let mut state_guard = state.write().await;
    let tx = state_guard.financial_system.process_payment(
        &AgentId(Uuid::new_v4()),  // Payer
        &AgentId(Uuid::new_v4()),  // Payee
        100.0,                     // Amount
    );
    Json(json!(tx).to_string())
}