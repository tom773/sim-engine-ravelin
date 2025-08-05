#!/bin/bash

# Ravelin Project Restructure Setup Script
# Creates the new DAE-pattern directory structure

set -e  # Exit on any error

PROJECT_ROOT="."
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Create directory if it doesn't exist
create_dir() {
    local dir="$1"
    if [ ! -d "$dir" ]; then
        mkdir -p "$dir"
        log_success "Created directory: $dir"
    else
        log_warning "Directory already exists: $dir"
    fi
}

# Create file if it doesn't exist
create_file() {
    local file="$1"
    local content="$2"
    
    # Create parent directory if it doesn't exist
    local parent_dir=$(dirname "$file")
    create_dir "$parent_dir"
    
    if [ ! -f "$file" ]; then
        echo "$content" > "$file"
        log_success "Created file: $file"
    else
        log_warning "File already exists: $file"
    fi
}

# Create workspace Cargo.toml
create_workspace_cargo() {
    local content='[workspace]
members = [
    "sim_types",
    "sim_decisions", 
    "sim_actions",
    "sim_effects",
    "sim_traits",
    "sim_macros",
    "domain/banking",
    "domain/production", 
    "domain/trading",
    "domain/consumption",
    "engine",
]
resolver = "2"

[workspace.dependencies]
dyn-clone = "1.0.20"
rand = "0.9.2"
serde = { version = "1.0.219", features = ["derive"] }
typetag = "0.2.20"
uuid = { version = "1.17.0", features = ["serde", "v4"] }
ndarray = "0.15.0"
serde_with = "3.14.0"
chrono = { version = "0.4.41", features = ["serde"] }
sscanf = "0.4.3"
thiserror = "2.0.12"
once_cell = "1.21.3"
toml = "0.9.4"
axum = { version = "0.8", features = ["macros"] }
tower-http = { version = "0.6", features = ["cors"] }
tokio = { version = "1.46.1", features = ["rt-multi-thread", "time"] }
serde_json = "1.0.141"
fake = { version = "4.3.0", features = ["derive"] }
crossbeam-channel = "0.5.15"
'
    create_file "Cargo.toml" "$content"
}

# Create sim_types crate
create_sim_types() {
    log_info "Creating sim_types crate..."
    
    create_dir "sim_types/src"
    
    # Cargo.toml
    local cargo_content='[package]
name = "sim_types"
version = "0.1.0"
edition = "2021"

[dependencies]
uuid = { workspace = true }
serde = { workspace = true }
serde_with = { workspace = true }
chrono = { workspace = true }
sscanf = { workspace = true }
thiserror = { workspace = true }
'
    create_file "sim_types/Cargo.toml" "$cargo_content"
    
    # lib.rs
    local lib_content='//! Core types and data structures for the Ravelin simulation
//! 
//! This crate contains pure data structures with no business logic.
//! It defines "what things are" but not "what things do".

pub mod ids;
pub mod agents;
pub mod instruments;
pub mod goods;
pub mod markets;
pub mod state;
pub mod time;

// Re-export commonly used types
pub use ids::*;
pub use agents::*;
pub use instruments::*;
pub use goods::*;
pub use markets::*;
pub use state::*;
pub use time::*;
'
    create_file "sim_types/src/lib.rs" "$lib_content"
    
    # Individual module files
    create_file "sim_types/src/ids.rs" "//! Agent, Instrument, and Asset ID types

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, Copy)]
pub struct AgentId(pub Uuid);

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, Copy)]
pub struct InstrumentId(pub Uuid);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy)]
pub struct AssetId(pub Uuid);

// Add Display and FromStr implementations for serde_as compatibility
// TODO: Add macro for this boilerplate
"

    create_file "sim_types/src/agents.rs" "//! Agent data structures (no behavior)

use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bank {
    pub id: AgentId,
    pub name: String,
    pub lending_spread: f64,
    pub deposit_spread: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Consumer {
    pub id: AgentId,
    pub age: u32,
    pub bank_id: AgentId,
    pub income: f64,
    pub personality: PersonalityArchetype,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Firm {
    pub id: AgentId,
    pub bank_id: AgentId,
    pub name: String,
    pub employees: Vec<AgentId>,
    pub wage_rate: f64,
    pub productivity: f64,
    pub recipe: Option<RecipeId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CentralBank {
    pub id: AgentId,
    pub policy_rate: f64,
    pub reserve_requirement: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PersonalityArchetype {
    Balanced,
    Spender,
    Saver,
}

// TODO: Move from existing codebase
"

    create_file "sim_types/src/instruments.rs" "//! Financial instrument types

use crate::*;
use serde::{Deserialize, Serialize};
use chrono::NaiveDate;

// TODO: Move instrument definitions from existing codebase
// - FinancialInstrument
// - InstrumentDetails trait and implementations
// - Cash, Deposits, Bonds, Loans, etc.
"

    create_file "sim_types/src/goods.rs" "//! Goods, recipes, and inventory types

use crate::*;
use serde::{Deserialize, Serialize};

// TODO: Move goods-related types from existing codebase
// - GoodId, RecipeId
// - Good, Recipe, InventoryItem
// - GoodsRegistry
"

    create_file "sim_types/src/markets.rs" "//! Market data structures

use crate::*;
use serde::{Deserialize, Serialize};

// TODO: Move market types from existing codebase
// - MarketId, Order, Bid, Ask
// - Market, Exchange
// - Trade, OrderBook
"

    create_file "sim_types/src/state.rs" "//! Core simulation state

use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimState {
    pub ticknum: u32,
    pub current_date: chrono::NaiveDate,
    pub financial_system: FinancialSystem,
    pub agents: AgentRegistry,
    pub config: SimConfig,
    pub history: SimHistory,
}

// TODO: Move state types from existing codebase
"

    create_file "sim_types/src/time.rs" "//! Time and date utilities

use chrono::NaiveDate;

// TODO: Add time utilities
// - Date handling
// - Time periods
// - Business day calculations
"
}

# Create sim_decisions crate  
create_sim_decisions() {
    log_info "Creating sim_decisions crate..."
    
    create_dir "sim_decisions/src"
    
    local cargo_content='[package]
name = "sim_decisions"
version = "0.1.0"
edition = "2021"

[dependencies]
sim_types = { path = "../sim_types" }
sim_traits = { path = "../sim_traits" }
serde = { workspace = true }
rand = { workspace = true }
ndarray = { workspace = true }
'
    create_file "sim_decisions/Cargo.toml" "$cargo_content"
    
    local lib_content='//! Decision types and decision-making logic
//!
//! This crate defines what agents *want* to do based on their
//! internal reasoning and the current state of the world.

pub mod bank_decisions;
pub mod consumer_decisions;
pub mod firm_decisions;
pub mod decision_models;
pub mod traits;

pub use bank_decisions::*;
pub use consumer_decisions::*;
pub use firm_decisions::*;
pub use decision_models::*;
pub use traits::*;
'
    create_file "sim_decisions/src/lib.rs" "$lib_content"
    
    create_file "sim_decisions/src/bank_decisions.rs" "//! Bank decision types and logic

use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BankDecision {
    LendOvernight { amount: f64, min_rate: f64 },
    BorrowOvernight { amount: f64, max_rate: f64 },
    SetDepositRate { rate: f64 },
    SetLendingRate { rate: f64 },
}

// TODO: Implement decision logic
"

    create_file "sim_decisions/src/consumer_decisions.rs" "//! Consumer decision types and logic

use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsumerDecision {
    Spend { amount: f64, good_id: GoodId },
    Save { amount: f64 },
    Work { hours: f64 },
}

// TODO: Implement decision logic
"

    create_file "sim_decisions/src/firm_decisions.rs" "//! Firm decision types and logic

use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FirmDecision {
    Produce { recipe_id: RecipeId, batches: u32 },
    Hire { count: u32 },
    SetPrice { good_id: GoodId, price: f64 },
    PayWages { employee: AgentId, amount: f64 },
}

// TODO: Implement decision logic
"

    create_file "sim_decisions/src/decision_models.rs" "//! Decision models (ML, basic, parametric)

// TODO: Move decision model implementations
// - BasicDecisionModel
// - MLDecisionModel  
// - ParametricMPC
"

    create_file "sim_decisions/src/traits.rs" "//! Decision-making traits

use sim_types::*;

pub trait DecisionMaker<D> {
    fn decide(&self, state: &SimState) -> Vec<D>;
}

// TODO: Add other decision-related traits
"
}

# Create sim_actions crate
create_sim_actions() {
    log_info "Creating sim_actions crate..."
    
    create_dir "sim_actions/src"
    
    local cargo_content='[package]
name = "sim_actions"
version = "0.1.0"  
edition = "2021"

[dependencies]
sim_types = { path = "../sim_types" }
serde = { workspace = true }
'
    create_file "sim_actions/Cargo.toml" "$cargo_content"
    
    local lib_content='//! Action types and validation
//!
//! This crate defines what agents *attempt* to do - the commands
//! they send to the simulation engine.

pub mod banking;
pub mod trading;
pub mod production;
pub mod consumption;
pub mod action_types;
pub mod validation;

pub use banking::*;
pub use trading::*;
pub use production::*;
pub use consumption::*;
pub use action_types::*;
pub use validation::*;
'
    create_file "sim_actions/src/lib.rs" "$lib_content"
    
    create_file "sim_actions/src/banking.rs" "//! Banking actions

use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BankingAction {
    Deposit { agent_id: AgentId, bank: AgentId, amount: f64 },
    Withdraw { agent_id: AgentId, bank: AgentId, amount: f64 },
    Transfer { from: AgentId, to: AgentId, amount: f64 },
    InjectLiquidity,
    UpdateReserves { bank: AgentId, amount_change: f64 },
}
"

    create_file "sim_actions/src/trading.rs" "//! Trading actions

use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TradingAction {
    PostBid { agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64 },
    PostAsk { agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64 },
}
"

    create_file "sim_actions/src/production.rs" "//! Production actions

use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProductionAction {
    Hire { agent_id: AgentId, count: u32 },
    Produce { agent_id: AgentId, recipe_id: RecipeId, batches: u32 },
    PayWages { agent_id: AgentId, employee: AgentId, amount: f64 },
}
"

    create_file "sim_actions/src/consumption.rs" "//! Consumption actions

use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsumptionAction {
    Purchase { agent_id: AgentId, seller: AgentId, good_id: GoodId, amount: f64 },
    Consume { agent_id: AgentId, good_id: GoodId, amount: f64 },
}
"

    create_file "sim_actions/src/action_types.rs" "//! Unified action type

use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimAction {
    Banking(BankingAction),
    Trading(TradingAction),
    Production(ProductionAction),
    Consumption(ConsumptionAction),
}

impl SimAction {
    pub fn name(&self) -> &'static str {
        match self {
            SimAction::Banking(_) => \"Banking\",
            SimAction::Trading(_) => \"Trading\", 
            SimAction::Production(_) => \"Production\",
            SimAction::Consumption(_) => \"Consumption\",
        }
    }
}
"

    create_file "sim_actions/src/validation.rs" "//! Action validation traits and utilities

use sim_types::*;
use crate::SimAction;

pub trait ActionValidator {
    fn validate(&self, action: &SimAction, state: &SimState) -> Result<(), String>;
}

// TODO: Add validation utilities
"
}

# Create sim_effects crate
create_sim_effects() {
    log_info "Creating sim_effects crate..."
    
    create_dir "sim_effects/src"
    
    local cargo_content='[package]
name = "sim_effects"
version = "0.1.0"
edition = "2021"

[dependencies]
sim_types = { path = "../sim_types" }
serde = { workspace = true }
thiserror = { workspace = true }
'
    create_file "sim_effects/Cargo.toml" "$cargo_content"
    
    local lib_content='//! Effect types and application logic
//!
//! This crate defines what *actually happens* to the world state
//! as a result of agent actions.

pub mod financial;
pub mod inventory;  
pub mod market;
pub mod agent;
pub mod effect_types;
pub mod application;

pub use financial::*;
pub use inventory::*;
pub use market::*;
pub use agent::*;
pub use effect_types::*;
pub use application::*;
'
    create_file "sim_effects/src/lib.rs" "$lib_content"
    
    create_file "sim_effects/src/financial.rs" "//! Financial effects

use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FinancialEffect {
    CreateInstrument(FinancialInstrument),
    UpdateInstrument { id: InstrumentId, new_principal: f64 },
    TransferInstrument { id: InstrumentId, new_creditor: AgentId },
    RemoveInstrument(InstrumentId),
}
"

    create_file "sim_effects/src/inventory.rs" "//! Inventory effects

use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InventoryEffect {
    AddInventory { owner: AgentId, good_id: GoodId, quantity: f64, unit_cost: f64 },
    RemoveInventory { owner: AgentId, good_id: GoodId, quantity: f64 },
}
"

    create_file "sim_effects/src/market.rs" "//! Market effects

use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MarketEffect {
    PlaceOrderInBook { market_id: MarketId, order: Order },
    ExecuteTrade(Trade),
    UpdatePrice { market_id: MarketId, new_price: f64 },
}
"

    create_file "sim_effects/src/agent.rs" "//! Agent-related effects

use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentEffect {
    Hire { firm: AgentId, count: u32 },
    UpdateIncome { id: AgentId, new_income: f64 },
    UpdateRevenue { id: AgentId, revenue: f64 },
}
"

    create_file "sim_effects/src/effect_types.rs" "//! Unified effect type

use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StateEffect {
    Financial(FinancialEffect),
    Inventory(InventoryEffect),
    Market(MarketEffect),
    Agent(AgentEffect),
}

impl StateEffect {
    pub fn name(&self) -> &'static str {
        match self {
            StateEffect::Financial(_) => \"Financial\",
            StateEffect::Inventory(_) => \"Inventory\",
            StateEffect::Market(_) => \"Market\",
            StateEffect::Agent(_) => \"Agent\",
        }
    }
}
"

    create_file "sim_effects/src/application.rs" "//! Effect application engine

use sim_types::*;
use crate::StateEffect;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum EffectError {
    #[error(\"Instrument not found: {id:?}\")]
    InstrumentNotFound { id: InstrumentId },
    #[error(\"Agent not found: {id:?}\")]
    AgentNotFound { id: AgentId },
    #[error(\"Invalid state: {0}\")]
    InvalidState(String),
}

pub trait EffectApplicator {
    fn apply_effect(&mut self, effect: &StateEffect) -> Result<(), EffectError>;
}

// TODO: Implement effect application logic
"
}

# Create sim_traits crate
create_sim_traits() {
    log_info "Creating sim_traits crate..."
    
    create_dir "sim_traits/src"
    
    local cargo_content='[package]
name = "sim_traits"
version = "0.1.0"
edition = "2021"

[dependencies]
sim_types = { path = "../sim_types" }
'
    create_file "sim_traits/Cargo.toml" "$cargo_content"
    
    local lib_content='//! Shared behavioral contracts and traits

pub mod decision_maker;
pub mod action_handler;
pub mod effect_applicator;
pub mod domain;

pub use decision_maker::*;
pub use action_handler::*;
pub use effect_applicator::*;
pub use domain::*;
'
    create_file "sim_traits/src/lib.rs" "$lib_content"
    
    create_file "sim_traits/src/decision_maker.rs" "//! Decision-making traits

use sim_types::*;

pub trait DecisionMaker {
    type Decision;
    
    fn decide(&self, state: &SimState) -> Vec<Self::Decision>;
}
"

    create_file "sim_traits/src/action_handler.rs" "//! Action handling traits

// TODO: Define action handling traits
"

    create_file "sim_traits/src/effect_applicator.rs" "//! Effect application traits

// TODO: Define effect application traits
"

    create_file "sim_traits/src/domain.rs" "//! Domain execution traits

// TODO: Define domain traits
"
}

# Create sim_macros crate
create_sim_macros() {
    log_info "Creating sim_macros crate..."
    
    create_dir "sim_macros/src"
    
    local cargo_content='[package]
name = "sim_macros"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true

[dependencies]
proc-macro2 = "1.0"
quote = "1.0"
syn = "2.0"
'
    create_file "sim_macros/Cargo.toml" "$cargo_content"
    
    local lib_content='//! Procedural macros for the simulation

extern crate proc_macro;

pub mod instruments;
pub mod validation;

pub use instruments::*;
pub use validation::*;
'
    create_file "sim_macros/src/lib.rs" "$lib_content"
    
    create_file "sim_macros/src/instruments.rs" "//! Instrument creation macros

// TODO: Move cash!, bond!, deposit! macros
"

    create_file "sim_macros/src/validation.rs" "//! Validation helper macros

// TODO: Add validation macros
"
}

# Create domain crates
create_domain_crates() {
    log_info "Creating domain crates..."
    
    # Banking domain
    create_dir "domain/banking/src"
    local banking_cargo='[package]
name = "domain_banking"
version = "0.1.0"
edition = "2021"

[dependencies]
sim_types = { path = "../../sim_types" }
sim_actions = { path = "../../sim_actions" }
sim_effects = { path = "../../sim_effects" }
sim_traits = { path = "../../sim_traits" }
serde = { workspace = true }
'
    create_file "domain/banking/Cargo.toml" "$banking_cargo"
    
    create_file "domain/banking/src/lib.rs" "//! Banking domain implementation

pub mod domain;
pub mod operations;
pub mod validation;
pub mod behavior;

pub use domain::*;
pub use operations::*;
pub use validation::*;
pub use behavior::*;
"

    create_file "domain/banking/src/domain.rs" "//! Banking domain handler

// TODO: Implement banking domain logic
"

    create_file "domain/banking/src/operations.rs" "//! Banking operations

// TODO: Implement deposit, withdraw, transfer operations
"

    create_file "domain/banking/src/validation.rs" "//! Banking validation

// TODO: Implement banking validation logic
"

    create_file "domain/banking/src/behavior.rs" "//! Bank behavior models

// TODO: Implement bank decision-making behavior
"

    # Similar structure for other domains
    for domain in "production" "trading" "consumption"; do
        create_dir "domain/$domain/src"
        
        local domain_cargo="[package]
name = \"domain_$domain\"
version = \"0.1.0\"
edition = \"2021\"

[dependencies]
sim_types = { path = \"../../sim_types\" }
sim_actions = { path = \"../../sim_actions\" }
sim_effects = { path = \"../../sim_effects\" }
sim_traits = { path = \"../../sim_traits\" }
serde = { workspace = true }
"
        create_file "domain/$domain/Cargo.toml" "$domain_cargo"
        
        create_file "domain/$domain/src/lib.rs" "//! $domain domain implementation

pub mod domain;
pub mod operations;
pub mod validation;
pub mod behavior;

pub use domain::*;
pub use operations::*;
pub use validation::*;
pub use behavior::*;
"

        create_file "domain/$domain/src/domain.rs" "//! $(echo $domain | sed 's/.*/\u&/') domain handler

// TODO: Implement $domain domain logic
"

        create_file "domain/$domain/src/operations.rs" "//! $(echo $domain | sed 's/.*/\u&/') operations

// TODO: Implement $domain operations
"

        create_file "domain/$domain/src/validation.rs" "//! $(echo $domain | sed 's/.*/\u&/') validation

// TODO: Implement $domain validation logic
"

        create_file "domain/$domain/src/behavior.rs" "//! $(echo $domain | sed 's/.*/\u&/') behavior models

// TODO: Implement $domain behavior models
"
    done
}

# Create engine crate
create_engine() {
    log_info "Creating engine crate..."
    
    create_dir "engine/cli"
    create_dir "engine/src/config"
    
    local engine_cargo='[package]
name = "engine"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "cli"
path = "cli/main.rs"

[dependencies]
sim_types = { path = "../sim_types" }
sim_actions = { path = "../sim_actions" }
sim_effects = { path = "../sim_effects" }
sim_decisions = { path = "../sim_decisions" }
sim_traits = { path = "../sim_traits" }
domain_banking = { path = "../domain/banking" }
domain_production = { path = "../domain/production" }
domain_trading = { path = "../domain/trading" }
domain_consumption = { path = "../domain/consumption" }

serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
axum = { workspace = true }
tower-http = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
rand = { workspace = true }
toml = { workspace = true }
thiserror = { workspace = true }
'
    create_file "engine/Cargo.toml" "$engine_cargo"
    
    create_file "engine/src/lib.rs" "//! Simulation engine - orchestrates the Decision → Action → Effect pipeline

pub mod executor;
pub mod scheduler;
pub mod registry;
pub mod state_manager;
pub mod scenario;
pub mod api;

pub use executor::*;
pub use scheduler::*;
pub use registry::*;
pub use state_manager::*;
pub use scenario::*;
pub use api::*;
"

    create_file "engine/src/executor.rs" "//! Main DAE execution pipeline

use sim_types::*;
use sim_actions::*;
use sim_effects::*;
use sim_decisions::*;

pub struct SimulationEngine {
    pub state: SimState,
    // TODO: Add domain registry, agent managers, etc.
}

impl SimulationEngine {
    pub fn new(state: SimState) -> Self {
        Self { state }
    }
    
    pub fn tick(&mut self) -> TickResult {
        // TODO: Implement DAE pipeline
        // 1. Collect decisions from agents
        // 2. Convert decisions to actions  
        // 3. Validate actions
        // 4. Convert actions to effects
        // 5. Apply effects to state
        // 6. Clear markets
        
        TickResult::default()
    }
}

#[derive(Debug, Default)]
pub struct TickResult {
    pub tick_number: u32,
    pub decisions_count: usize,
    pub actions_count: usize,
    pub effects_count: usize,
}
"

    create_file "engine/src/scheduler.rs" "//! Tick scheduling and timing

// TODO: Implement scheduling logic
"

    create_file "engine/src/registry.rs" "//! Domain registry management

// TODO: Implement domain registry
"

    create_file "engine/src/state_manager.rs" "//! State management and persistence

// TODO: Implement state management
"

    create_file "engine/src/scenario.rs" "//! Scenario loading and initialization

// TODO: Implement scenario loading
"

    create_file "engine/src/api.rs" "//! HTTP API handlers

// TODO: Implement API handlers
"

    create_file "engine/cli/main.rs" "//! CLI entry point

use engine::*;

#[tokio::main]
async fn main() {
    println!(\"Ravelin Simulation Engine\");
    
    // TODO: Implement CLI
    // - Load scenario
    // - Initialize engine
    // - Start API server
    // - Run simulation
}
"

    create_file "engine/cli/routes.rs" "//! API route handlers

// TODO: Implement API routes
"

    # Configuration files
    create_file "engine/src/config/scenario.toml" "# Default scenario configuration

name = \"Base Scenario\"
description = \"A simple economy with banks, firms, and consumers\"

[config]
iterations = 100

# TODO: Add scenario configuration
"

    create_file "engine/src/config/goods.toml" "# Goods and recipes configuration

[[goods]]
slug = \"oil\"
name = \"Crude Oil\"
unit = \"barrel\"
category = \"RawMaterial\"

# TODO: Add goods configuration
"
}

# Main execution
main() {
    log_info "Setting up Ravelin project structure with DAE pattern..."
    
    # Create all the crates and files
    create_workspace_cargo
    create_sim_types
    create_sim_decisions
    create_sim_actions
    create_sim_effects
    create_sim_traits
    create_sim_macros
    create_domain_crates
    create_engine
    
    log_success "Project structure created successfully!"
    echo
    log_info "Next steps:"
    echo "  1. Run 'cargo check' to verify the workspace builds"
    echo "  2. Start migrating code from the existing crates"
    echo "  3. Implement the TODOs in each file"
    echo "  4. Set up tests for each crate"
    echo
    log_info "The new structure follows the Decision → Action → Effect pattern:"
    echo "  - sim_types: Pure data structures"
    echo "  - sim_decisions: What agents want to do"  
    echo "  - sim_actions: What agents attempt to do"
    echo "  - sim_effects: What actually happens"
    echo "  - domain/*: Business logic for each domain"
    echo "  - engine: Orchestration and execution"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    log_warning "No Cargo.toml found in current directory."
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_info "Exiting..."
        exit 1
    fi
fi

# Run main function
main "$@"