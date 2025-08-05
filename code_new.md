# Included Files

- `.rustfmt.toml`
- `Cargo.toml`
- `config/config.toml`
- `config/goods.toml`
- `crates/core/sim_actions/Cargo.toml`
- `crates/core/sim_actions/src/action_types.rs`
- `crates/core/sim_actions/src/banking.rs`
- `crates/core/sim_actions/src/consumption.rs`
- `crates/core/sim_actions/src/lib.rs`
- `crates/core/sim_actions/src/production.rs`
- `crates/core/sim_actions/src/trading.rs`
- `crates/core/sim_actions/src/validation.rs`
- `crates/core/sim_decisions/Cargo.toml`
- `crates/core/sim_decisions/src/bank_decisions.rs`
- `crates/core/sim_decisions/src/consumer_decisions.rs`
- `crates/core/sim_decisions/src/decision_models.rs`
- `crates/core/sim_decisions/src/firm_decisions.rs`
- `crates/core/sim_decisions/src/lib.rs`
- `crates/core/sim_effects/Cargo.toml`
- `crates/core/sim_effects/src/agent.rs`
- `crates/core/sim_effects/src/application.rs`
- `crates/core/sim_effects/src/effect_types.rs`
- `crates/core/sim_effects/src/financial.rs`
- `crates/core/sim_effects/src/inventory.rs`
- `crates/core/sim_effects/src/lib.rs`
- `crates/core/sim_effects/src/market.rs`
- `crates/core/sim_prelude/Cargo.toml`
- `crates/core/sim_prelude/src/lib.rs`
- `crates/core/sim_traits/Cargo.toml`
- `crates/core/sim_traits/src/action_handler.rs`
- `crates/core/sim_traits/src/decision_maker.rs`
- `crates/core/sim_traits/src/domain.rs`
- `crates/core/sim_traits/src/effect_applicator.rs`
- `crates/core/sim_traits/src/lib.rs`
- `crates/core/sim_types/Cargo.toml`
- `crates/core/sim_types/src/agents.rs`
- `crates/core/sim_types/src/balance_sheet.rs`
- `crates/core/sim_types/src/goods.rs`
- `crates/core/sim_types/src/ids.rs`
- `crates/core/sim_types/src/instruments.rs`
- `crates/core/sim_types/src/lib.rs`
- `crates/core/sim_types/src/macros.rs`
- `crates/core/sim_types/src/markets.rs`
- `crates/core/sim_types/src/state.rs`
- `crates/core/sim_types/src/time.rs`
- `crates/domains/banking/Cargo.toml`
- `crates/domains/banking/src/behavior.rs`
- `crates/domains/banking/src/domain.rs`
- `crates/domains/banking/src/lib.rs`
- `crates/domains/banking/src/operations.rs`
- `crates/domains/banking/src/validation.rs`
- `crates/domains/consumption/Cargo.toml`
- `crates/domains/consumption/src/behavior.rs`
- `crates/domains/consumption/src/domain.rs`
- `crates/domains/consumption/src/lib.rs`
- `crates/domains/consumption/src/operations.rs`
- `crates/domains/consumption/src/validation.rs`
- `crates/domains/prelude/Cargo.toml`
- `crates/domains/prelude/src/lib.rs`
- `crates/domains/production/Cargo.toml`
- `crates/domains/production/src/behavior.rs`
- `crates/domains/production/src/domain.rs`
- `crates/domains/production/src/lib.rs`
- `crates/domains/production/src/operations.rs`
- `crates/domains/production/src/validation.rs`
- `crates/domains/trading/Cargo.toml`
- `crates/domains/trading/src/behavior.rs`
- `crates/domains/trading/src/domain.rs`
- `crates/domains/trading/src/lib.rs`
- `crates/domains/trading/src/operations.rs`
- `crates/domains/trading/src/validation.rs`
- `crates/engine/Cargo.toml`
- `crates/engine/cli/bridge.rs`
- `crates/engine/cli/main.rs`
- `crates/engine/cli/routes.rs`
- `crates/engine/src/api.rs`
- `crates/engine/src/executor.rs`
- `crates/engine/src/factory.rs`
- `crates/engine/src/lib.rs`
- `crates/engine/src/registry.rs`
- `crates/engine/src/scenario.rs`
- `crates/engine/src/scheduler.rs`
- `crates/engine/src/state_manager.rs`

---

## `.rustfmt.toml`

```toml
max_width          = 120

use_small_heuristics = "Max"

fn_call_width      = 120      # same as max_width above
attr_fn_like_width = 120      # for `#[derive(...)]`‑style macros

fn_params_layout   = "Compressed"  # renamed from `fn_args_layout` :contentReference[oaicite:3]{index=3}

```

---

## `Cargo.toml`

```toml
[workspace]
members = [
    "crates/ml", 
    "crates/engine", 
    "crates/core/sim_actions", 
    "crates/core/sim_traits", 
    "crates/core/sim_prelude", 
    "crates/core/sim_types", 
    "crates/core/sim_effects", 
    "crates/core/sim_decisions",
    "crates/domains/consumption", 
    "crates/domains/banking",
    "crates/domains/prelude", 
    "crates/domains/production",
    "crates/domains/trading", 
]
resolver = "3"

[workspace.dependencies]
# Core Rust ecosystem
dyn-clone = "1.0.20"
rand = "0.9.2"
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.141"
typetag = "0.2.20"
uuid = { version = "1.17.0", features = ["serde", "v4", "v5"] }
thiserror = "2.0.12"
once_cell = "1.21.3"
toml = "0.9.4"

ndarray = "0.15.0"
serde_with = "3.14.0"

chrono = { version = "0.4.41", features = ["serde"] }

sscanf = "0.4.3"

tokio = { version = "1.46.1", features = ["rt-multi-thread", "time", "macros"] }
axum = { version = "0.8", features = ["macros"] }
tower-http = { version = "0.6", features = ["cors"] }

crossbeam-channel = "0.5.15"

fake = { version = "4.3.0", features = ["derive"] }

proc-macro2 = "1.0"
quote = "1.0"
syn = "2.0"

polars = { version = "0.50.0", features = ["lazy", "csv", "serde", "parquet"] }
linfa = { version = "0.7.1", features = ["serde"] }
linfa-logistic = { version = "0.7.1", features = ["serde"] }
lightgbm3 = "1.0.8"
bincode = { version = "2.0.1", features = ["serde"] }
```

---

## `config/config.toml`

```toml
name = "Base Scenario"
description = "A simple economy with two banks, one oil refinery, and a few consumers."

[config]
iterations = 100
treasuryTenorsToRegister = ["T2Y", "T5Y", "T10Y", "T30Y"]

[[banks]]
id = "bank_a"
name = "Ravelin National Bank"
initialReserves = 5000000.0
initialBonds = [
    { tenor = "T10Y", faceValue = 500000.0 }
]

[[banks]]
id = "bank_b"
name = "Economic Sim Savings & Loan"
initialReserves = 3000000.0
initialBonds = [
    { tenor = "T2Y", faceValue = 200000.0 },
    { tenor = "T10Y", faceValue = 150000.0 }
]

[[firms]]
id = "global_oil"
name = "Global Oil Corp"
bankId = "bank_a"
recipeName = "Oil Refining"
initialCash = 250000.0
initialInventory = [
    { goodSlug = "oil", quantity = 1000.0, unitCost = 50.0 }
]

[[consumers]]
id = "consumer_1"
bankId = "bank_a"
initialCash = 5000.0
income = 60000.0 # Annual

[[consumers]]
id = "consumer_2"
bankId = "bank_b"
initialCash = 7500.0
income = 85000.0 # Annual
```

---

## `config/goods.toml`

```toml
[[goods]]
slug = "oil"
name = "Crude Oil"
unit = "barrel"
category = "RawMaterial"

[[goods]]
slug = "petrol"
name = "Petrol"
unit = "gallon"
category = "Energy"

[[recipes]]
name = "Oil Refining"
output = { slug = "petrol", qty = 19.5 }
inputs = [
  { slug = "oil",   qty = 1.0 }
]
labourHours = 0.5
capitalRequired = 100.0
efficiency = 0.85
```

---

## `crates/core/sim_actions/Cargo.toml`

```toml
[package]
name = "sim_actions"
version = "0.1.0"  
edition = "2024"

[dependencies]
sim_types = { path = "../sim_types" }
serde = { workspace = true }


```

---

## `crates/core/sim_actions/src/action_types.rs`

```rust
use crate::*;
use sim_types::AgentId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimAction {
    Banking(BankingAction),
    Trading(TradingAction),
    Production(ProductionAction),
    Consumption(ConsumptionAction),
}

impl SimAction {
    pub fn name(&self) -> String {
        match self {
            SimAction::Banking(action) => format!("Banking::{}", action.name()),
            SimAction::Trading(action) => format!("Trading::{}", action.name()),
            SimAction::Production(action) => format!("Production::{}", action.name()),
            SimAction::Consumption(action) => format!("Consumption::{}", action.name()),
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            SimAction::Banking(action) => action.agent_id(),
            SimAction::Trading(action) => action.agent_id(),
            SimAction::Production(action) => action.agent_id(),
            SimAction::Consumption(action) => action.agent_id(),
        }
    }
}
```

---

## `crates/core/sim_actions/src/banking.rs`

```rust
use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BankingAction {
    Deposit { agent_id: AgentId, bank: AgentId, amount: f64 },
    Withdraw { agent_id: AgentId, bank: AgentId, amount: f64 },
    Transfer { from: AgentId, to: AgentId, amount: f64 },
    PayWages { agent_id: AgentId, employee: AgentId, amount: f64 },
    UpdateReserves { bank: AgentId, amount_change: f64 },
    InjectLiquidity,
}

impl BankingAction {
    pub fn name(&self) -> &'static str {
        match self {
            BankingAction::Deposit { .. } => "Deposit",
            BankingAction::Withdraw { .. } => "Withdraw", 
            BankingAction::Transfer { .. } => "Transfer",
            BankingAction::PayWages { .. } => "PayWages",
            BankingAction::UpdateReserves { .. } => "UpdateReserves",
            BankingAction::InjectLiquidity => "InjectLiquidity",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            BankingAction::Deposit { agent_id, .. } => *agent_id,
            BankingAction::Withdraw { agent_id, .. } => *agent_id,
            BankingAction::Transfer { from, .. } => *from,
            BankingAction::PayWages { agent_id, .. } => *agent_id,
            BankingAction::UpdateReserves { bank, .. } => *bank,
            BankingAction::InjectLiquidity => AgentId::default(), // System action
        }
    }
}

```

---

## `crates/core/sim_actions/src/consumption.rs`

```rust
use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsumptionAction {
    Purchase { agent_id: AgentId, seller: AgentId, good_id: GoodId, amount: f64 },
    Consume { agent_id: AgentId, good_id: GoodId, amount: f64 },
}

impl ConsumptionAction {
    pub fn name(&self) -> &'static str {
        match self {
            ConsumptionAction::Purchase { .. } => "Purchase",
            ConsumptionAction::Consume { .. } => "Consume",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            ConsumptionAction::Purchase { agent_id, .. } => *agent_id,
            ConsumptionAction::Consume { agent_id, .. } => *agent_id,
        }
    }
}
```

---

## `crates/core/sim_actions/src/lib.rs`

```rust
//! Action types and validation
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


```

---

## `crates/core/sim_actions/src/production.rs`

```rust
use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProductionAction {
    Hire { agent_id: AgentId, count: u32 },
    Produce { agent_id: AgentId, recipe_id: RecipeId, batches: u32 },
    PayWages { agent_id: AgentId, employee: AgentId, amount: f64 },
}

impl ProductionAction {
    pub fn name(&self) -> &'static str {
        match self {
            ProductionAction::Hire { .. } => "Hire",
            ProductionAction::Produce { .. } => "Produce",
            ProductionAction::PayWages { .. } => "PayWages",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            ProductionAction::Hire { agent_id, .. } => *agent_id,
            ProductionAction::Produce { agent_id, .. } => *agent_id,
            ProductionAction::PayWages { agent_id, .. } => *agent_id,
        }
    }
}
```

---

## `crates/core/sim_actions/src/trading.rs`

```rust
use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TradingAction {
    PostBid { agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64 },
    PostAsk { agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64 },
}

impl TradingAction {
    pub fn name(&self) -> &'static str {
        match self {
            TradingAction::PostBid { .. } => "PostBid",
            TradingAction::PostAsk { .. } => "PostAsk",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            TradingAction::PostBid { agent_id, .. } => *agent_id,
            TradingAction::PostAsk { agent_id, .. } => *agent_id,
        }
    }
}
```

---

## `crates/core/sim_actions/src/validation.rs`

```rust
use sim_types::*;
use crate::SimAction;

pub trait ActionValidator {
    fn validate(&self, action: &SimAction, state: &SimState) -> Result<(), String>;
}

pub struct Validator;

impl Validator {
    pub fn positive_amount(amount: f64) -> Result<(), String> {
        if amount <= 0.0 {
            Err(format!("Amount must be positive, got: {:.2}", amount))
        } else {
            Ok(())
        }
    }
    
    pub fn non_negative_amount(amount: f64) -> Result<(), String> {
        if amount < 0.0 {
            Err(format!("Amount cannot be negative, got: {:.2}", amount))
        } else {
            Ok(())
        }
    }
    
    pub fn positive_integer(value: u32, field_name: &str) -> Result<(), String> {
        if value == 0 {
            Err(format!("{} must be greater than 0", field_name))
        } else {
            Ok(())
        }
    }
    
    pub fn percentage(value: f64) -> Result<(), String> {
        if value < 0.0 || value > 1.0 {
            Err(format!("Percentage must be between 0 and 1, got: {:.4}", value))
        } else {
            Ok(())
        }
    }
}
```

---

## `crates/core/sim_decisions/Cargo.toml`

```toml
[package]
name = "sim_decisions"
version = "0.1.0"
edition = "2024"

[dependencies]
sim_types = { path = "../sim_types" }
sim_traits = { path = "../sim_traits" }
serde = { workspace = true }
rand = { workspace = true }
ndarray = { workspace = true }
dyn-clone = { workspace = true }
typetag = { workspace = true }
```

---

## `crates/core/sim_decisions/src/bank_decisions.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BankDecision {
    BorrowOvernight { amount_dollars: f64, max_annual_rate_bps: f64 },
    LendOvernight { amount_dollars: f64, min_annual_rate_bps: f64 },
    SetDepositRate { rate: f64 },
    SetLendingRate { rate: f64 },
    ManageReserves { target_level: f64 },
}

impl BankDecision {
    pub fn name(&self) -> &'static str {
        match self {
            BankDecision::BorrowOvernight { .. } => "BorrowOvernight",
            BankDecision::LendOvernight { .. } => "LendOvernight",
            BankDecision::SetDepositRate { .. } => "SetDepositRate",
            BankDecision::SetLendingRate { .. } => "SetLendingRate",
            BankDecision::ManageReserves { .. } => "ManageReserves",
        }
    }
}
```

---

## `crates/core/sim_decisions/src/consumer_decisions.rs`

```rust
use sim_types::*;
use serde::{Deserialize, Serialize};
use ndarray::Array1;
use dyn_clone::{clone_trait_object, DynClone};
use std::fmt::Debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsumerDecision {
    Spend { agent_id: AgentId, seller_id: AgentId, amount: f64, good_id: GoodId },
    Save { agent_id: AgentId, amount: f64 },
    Work { agent_id: AgentId, hours: f64 },
}

impl ConsumerDecision {
    pub fn name(&self) -> &'static str {
        match self {
            ConsumerDecision::Spend { .. } => "Spend",
            ConsumerDecision::Save { .. } => "Save",
            ConsumerDecision::Work { .. } => "Work",
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            ConsumerDecision::Spend { agent_id, .. } => *agent_id,
            ConsumerDecision::Save { agent_id, .. } => *agent_id,
            ConsumerDecision::Work { agent_id, .. } => *agent_id,
        }
    }
}

pub trait FeatureSource {
    fn get_age(&self) -> u32;
    fn get_income(&self) -> f64;
    fn get_savings(&self) -> f64;
    fn get_debt(&self) -> f64;
    fn get_family_size(&self) -> u32 { 1 }
    fn get_has_children(&self) -> bool { false }
    fn get_education_level_numeric(&self) -> u32 { 2 }
    fn get_housing_status_numeric(&self) -> u32 { 0 }
    fn get_is_urban(&self) -> bool { true }
    fn get_region_numeric(&self) -> u32 { 1 }
}

pub trait SpendingPredictor: DynClone + Send + Sync {
    fn predict_spending(&self, features: &Array1<f64>) -> f64;
    fn get_feature_names(&self) -> &[String];
}

clone_trait_object!(SpendingPredictor);

impl Debug for dyn SpendingPredictor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SpendingPredictor")
    }
}
```

---

## `crates/core/sim_decisions/src/decision_models.rs`

```rust
use sim_types::*;
use crate::*;
use dyn_clone::{DynClone, clone_trait_object};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use ndarray::Array1;

#[typetag::serde(tag = "type")]
pub trait DecisionModel: DynClone + Send + Sync {
    fn decide(&self, consumer: &Consumer, fs: &FinancialSystem, rng: &mut dyn RngCore) -> Vec<ConsumerDecision>;
}

clone_trait_object!(DecisionModel);

/// Bank decision-making model
#[typetag::serde(tag = "type")]
pub trait BankDecisionModel: DynClone + Send + Sync {
    fn decide(&self, bank: &Bank, fs: &FinancialSystem, rng: &mut dyn RngCore) -> Vec<BankDecision>;
}
clone_trait_object!(BankDecisionModel);

impl Debug for dyn BankDecisionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BankDecisionModel")
    }
}

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct BasicBankDecisionModel;

#[typetag::serde]
impl BankDecisionModel for BasicBankDecisionModel {
    fn decide(&self, bank: &Bank, fs: &FinancialSystem, _rng: &mut dyn RngCore) -> Vec<BankDecision> {
        let mut decisions = Vec::new();

        let total_deposits = bank.total_liabilities(fs);
        let required_reserves = total_deposits * fs.central_bank.reserve_requirement;
        let desired_buffer = total_deposits * 0.02; // 2% buffer
        let target_reserve_level = required_reserves + desired_buffer;

        let current_reserves = bank.get_reserves(fs);
        let reserve_surplus_or_shortfall = current_reserves - target_reserve_level;

        if reserve_surplus_or_shortfall < -1.0 {
            // Borrow if short
            let amount_needed = -reserve_surplus_or_shortfall;
            decisions.push(BankDecision::BorrowOvernight {
                amount_dollars: amount_needed,
                max_annual_rate_bps: (fs.central_bank.policy_rate * 10000.0) + 50.0,
            });
        } else if reserve_surplus_or_shortfall > 1.0 {
            // Lend if surplus
            let amount_to_lend = reserve_surplus_or_shortfall * 0.75; // Lend 75% of surplus
            if amount_to_lend > 100.0 {
                // Threshold to act
                decisions.push(BankDecision::LendOvernight {
                    amount_dollars: amount_to_lend,
                    min_annual_rate_bps: (fs.central_bank.policy_rate * 10000.0) - 25.0,
                });
            }
        }

        decisions
    }
}

/// Consumer decision-making model
#[typetag::serde(tag = "type")]
pub trait ConsumerDecisionModel: DynClone + Send + Sync {
    fn decide(&self, consumer: &Consumer, fs: &FinancialSystem, rng: &mut dyn RngCore) -> Vec<ConsumerDecision>;
}
clone_trait_object!(ConsumerDecisionModel);

impl Debug for dyn ConsumerDecisionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConsumerDecisionModel")
    }
}

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct BasicConsumerDecisionModel;

#[typetag::serde]
impl ConsumerDecisionModel for BasicConsumerDecisionModel {
    fn decide(&self, consumer: &Consumer, fs: &FinancialSystem, _rng: &mut dyn RngCore) -> Vec<ConsumerDecision> {
        let mut decisions = Vec::new();

        // Income is annual, convert to weekly for decision making
        let weekly_income = consumer.income / 52.0;
        let cash_holdings = consumer.get_cash_holdings(fs);
        let total_available = weekly_income + cash_holdings;

        // Basic spending decision based on personality
        let prop_to_consume = match consumer.personality {
            PersonalityArchetype::Balanced => 0.7,
            PersonalityArchetype::Spender => 0.8,
            PersonalityArchetype::Saver => 0.6,
        };
        let spend_amount = total_available * prop_to_consume;
        let save_amount = total_available - spend_amount;

        let good_to_buy = good_id!("petrol");
        if let Some(seller) = fs.exchange.goods_market(&good_to_buy).and_then(|m| m.best_ask()) {
            if spend_amount > 1.0 {
                decisions.push(ConsumerDecision::Spend {
                    agent_id: consumer.id,
                    seller_id: seller.agent_id,
                    amount: spend_amount,
                    good_id: good_to_buy,
                });
            }
        }

        if save_amount > 1.0 {
            decisions.push(ConsumerDecision::Save { agent_id: consumer.id, amount: save_amount });
        }

        decisions
    }
}

/// Firm decision-making model
#[typetag::serde(tag = "type")]
pub trait FirmDecisionModel: DynClone + Send + Sync {
    fn decide(&self, firm: &Firm, fs: &FinancialSystem, rng: &mut dyn RngCore) -> Vec<FirmDecision>;
}
clone_trait_object!(FirmDecisionModel);

impl Debug for dyn FirmDecisionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FirmDecisionModel")
    }
}

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct BasicFirmDecisionModel;

#[typetag::serde]
impl FirmDecisionModel for BasicFirmDecisionModel {
    fn decide(&self, firm: &Firm, fs: &FinancialSystem, _rng: &mut dyn RngCore) -> Vec<FirmDecision> {
        let mut decisions = Vec::new();

        // Hiring decision
        if firm.employees.len() < 5 {
            decisions.push(FirmDecision::Hire { count: 1 });
        }

        // Production decision
        if let Some(recipe_id) = firm.recipe {
            if !firm.employees.is_empty() {
                if let Some(recipe) = fs.goods.get_recipe(&recipe_id) {
                    if let Some(bs) = fs.get_bs_by_id(&firm.id) {
                        if let Some(inventory) = bs.get_inventory() {
                            let can_produce = recipe.inputs.iter().all(|(good, qty)| {
                                inventory.get(good).map_or(false, |item| item.quantity >= *qty)
                            });
                            if can_produce {
                                decisions.push(FirmDecision::Produce { recipe_id, batches: 1 });
                            }
                        }
                    }
                }
            }
        }

        // Pay wages weekly
        for employee_id in firm.get_employees() {
            let weekly_wage = firm.wage_rate * 40.0;
            decisions.push(FirmDecision::PayWages { employee: *employee_id, amount: weekly_wage });
        }

        // Sell inventory
        if let Some(bs) = fs.get_bs_by_id(&firm.id) {
            if let Some(inventory) = bs.get_inventory() {
                for (good_id, item) in inventory.iter() {
                    if let Some(recipe) = firm.recipe.and_then(|id| fs.goods.get_recipe(&id)) {
                        if recipe.output.0 == *good_id && item.quantity > 0.0 {
                            decisions.push(FirmDecision::SellInventory {
                                good_id: *good_id,
                                quantity: item.quantity,
                            });
                        }
                    }
                }
            }
        }

        // Set price based on costs
        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = fs.goods.get_recipe(&recipe_id) {
                // Crude cost calculation for one week
                let weekly_labor_cost = firm.employees.len() as f64 * firm.wage_rate * 40.0;
                let weekly_output = recipe.output.1 * recipe.efficiency * firm.employees.len() as f64;
                if weekly_output > 0.0 {
                    let unit_cost = weekly_labor_cost / weekly_output;
                    let target_price = unit_cost * 1.25; // 25% markup
                    decisions.push(FirmDecision::SetPrice { good_id: recipe.output.0, price: target_price });
                }
            }
        }

        decisions
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MLDecisionModel {
    #[serde(skip)]
    pub predictor: Option<Box<dyn SpendingPredictor>>,
    pub model_path: String,
}

#[typetag::serde]
impl DecisionModel for MLDecisionModel {
    fn decide(&self, consumer: &Consumer, fs: &FinancialSystem, _rng: &mut dyn RngCore) -> Vec<ConsumerDecision> {
        if let Some(predictor) = &self.predictor {
            let features = extract_consumer_features(consumer, fs);
            let predicted_annual_spending = predictor.predict_spending(&features);

            let cash_holdings = fs
                .balance_sheets
                .get(&consumer.id)
                .map(|bs| {
                    bs.assets
                        .values()
                        .filter(|inst| inst.details.as_any().is::<CashDetails>())
                        .map(|inst| inst.principal)
                        .sum::<f64>()
                })
                .unwrap_or(0.0);

            // Use weekly income for decision making
            let weekly_income = consumer.income / 52.0;
            let total_available = weekly_income + cash_holdings;
            let spending_per_period = predicted_annual_spending / 52.0;
            let spend_amount = spending_per_period.min(total_available);
            let save_amount = total_available - spend_amount;

            let mut decisions = Vec::new();

            let good_to_buy = good_id!("petrol");
            let seller_id = fs
                .exchange
                .goods_market(&good_to_buy)
                .and_then(|market| market.best_ask())
                .map(|ask| ask.agent_id.clone());

            if spend_amount > 0.0 && seller_id.is_some() {
                decisions.push(ConsumerDecision::Spend {
                    agent_id: consumer.id.clone(),
                    seller_id: seller_id.unwrap(),
                    amount: spend_amount,
                    good_id: good_to_buy,
                });
            }
            if save_amount > 0.0 {
                decisions.push(ConsumerDecision::Save { agent_id: consumer.id.clone(), amount: save_amount });
            }

            decisions
        } else {
            let basic = BasicConsumerDecisionModel {};
            basic.decide(consumer, fs, _rng)
        }
    }
}

fn extract_consumer_features(consumer: &Consumer, _fs: &FinancialSystem) -> Array1<f64> {
    let income = consumer.income; // Annual income
    let log_income = income.max(1000.0).ln();

    let income_bracket = if income < 30000.0 {
        1.0
    } else if income < 50000.0 {
        2.0
    } else if income < 75000.0 {
        3.0
    } else if income < 100000.0 {
        4.0
    } else if income < 150000.0 {
        5.0
    } else {
        6.0
    };

    let food_share = 0.15;
    let housing_share = 0.30;
    let transport_share = 0.20;
    let health_share = 0.10;

    let age = consumer.age;
    let age_group = if age < 35 {
        1.0
    } else if age < 55 {
        2.0
    } else if age < 65 {
        3.0
    } else {
        4.0
    };

    // These would ideally come from a FeatureSource implementation
    let education = 2.0; // Some College
    let family_size = 1.0;
    let has_children = false;
    let housing_status = 0.0; // Owned
    let is_urban = true;
    let region = 1.0; // Northeast

    Array1::from(vec![
        income,
        log_income,
        age_group,
        family_size,
        if has_children { 1.0 } else { 0.0 },
        education,
        housing_status,
        if is_urban { 1.0 } else { 0.0 },
        1.0, // earner_ratio
        region,
        food_share,
        housing_share,
        transport_share,
        health_share,
        income_bracket * age_group,
        income_bracket * education,
    ])
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParametricMPC {
    pub mpc_min: f64,
    pub mpc_max: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

#[typetag::serde]
impl DecisionModel for ParametricMPC {
    fn decide(&self, consumer: &Consumer, fs: &FinancialSystem, _rng: &mut dyn RngCore) -> Vec<ConsumerDecision> {
        let weekly_income = consumer.income / 52.0;
        let cash = fs.get_liquid_assets(&consumer.id);
        let total = weekly_income + cash;
        let wealth_ratio = fs.get_total_assets(&consumer.id) / consumer.income.max(1.0);

        let mpc = self.mpc_min
            + (self.mpc_max - self.mpc_min)
                / (1.0 + (self.a + self.b * consumer.income.ln() + self.c * wealth_ratio).exp());

        let spend_amount = mpc * total;
        let save_amount = total - spend_amount;

        let mut decisions = Vec::new();

        let good_to_buy = good_id!("petrol");
        let seller_id =
            fs.exchange.goods_market(&good_to_buy).and_then(|market| market.best_ask()).map(|ask| ask.agent_id.clone());

        if spend_amount > 0.0 && seller_id.is_some() {
            decisions.push(ConsumerDecision::Spend {
                agent_id: consumer.id.clone(),
                seller_id: seller_id.unwrap(),
                amount: spend_amount,
                good_id: good_to_buy,
            });
        }

        if save_amount > 0.0 {
            decisions.push(ConsumerDecision::Save { agent_id: consumer.id.clone(), amount: save_amount });
        }

        decisions
    }
}
```

---

## `crates/core/sim_decisions/src/firm_decisions.rs`

```rust
use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FirmDecision {
    Produce { recipe_id: RecipeId, batches: u32 },
    Hire { count: u32 },
    SetPrice { good_id: GoodId, price: f64 },
    PayWages { employee: AgentId, amount: f64 },
    SellInventory { good_id: GoodId, quantity: f64 },
}

impl FirmDecision {
    pub fn name(&self) -> &'static str {
        match self {
            FirmDecision::Produce { .. } => "Produce",
            FirmDecision::Hire { .. } => "Hire",
            FirmDecision::SetPrice { .. } => "SetPrice",
            FirmDecision::PayWages { .. } => "PayWages",
            FirmDecision::SellInventory { .. } => "SellInventory",
        }
    }
}
```

---

## `crates/core/sim_decisions/src/lib.rs`

```rust
pub mod bank_decisions;
pub mod consumer_decisions;
pub mod firm_decisions;
pub mod decision_models;

pub use bank_decisions::*;
pub use consumer_decisions::*;
pub use firm_decisions::*;
pub use decision_models::*;


```

---

## `crates/core/sim_effects/Cargo.toml`

```toml
[package]
name = "sim_effects"
version = "0.1.0"
edition = "2024"

[dependencies]
sim_types = { path = "../sim_types" }
serde = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true }

```

---

## `crates/core/sim_effects/src/agent.rs`

```rust
use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentEffect {
    Hire { firm: AgentId, count: u32 },
    UpdateIncome { id: AgentId, new_income: f64 },
    UpdateRevenue { id: AgentId, revenue: f64 },
    Produce { firm: AgentId, good_id: GoodId, amount: f64 },
}

impl AgentEffect {
    pub fn name(&self) -> &'static str {
        match self {
            AgentEffect::Hire { .. } => "Hire",
            AgentEffect::UpdateIncome { .. } => "UpdateIncome",
            AgentEffect::UpdateRevenue { .. } => "UpdateRevenue",
            AgentEffect::Produce { .. } => "Produce",
        }
    }
}
```

---

## `crates/core/sim_effects/src/application.rs`

```rust
use sim_types::*;
use crate::*;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum EffectError {
    #[error("Instrument not found: {id:?}")]
    InstrumentNotFound { id: InstrumentId },
    #[error("Agent not found: {id:?}")]
    AgentNotFound { id: AgentId },
    #[error("Firm not found: {id:?}")]
    FirmNotFound { id: AgentId },
    #[error("Market not found: {market:?}")]
    MarketNotFound { market: String },
    #[error("Insufficient inventory for {good:?}: have {have}, need {need}")]
    InsufficientInventory { good: GoodId, have: f64, need: f64 },
    #[error("Financial system error: {0}")]
    FinancialSystemError(String),
    #[error("Invalid state: {0}")]
    InvalidState(String),
    #[error("Invalid recipe: {id:?}")]
    RecipeError { id: RecipeId },
    #[error("Unimplemented action: {0}")]
    UnimplementedAction(String),
    #[error("Unhandled action: {0}")]
    Unhandled(String),
    #[error("Bank transaction failed: Action {0}, reason {1}")]
    TransactionFailure(String, String),
}

pub trait EffectApplicator {
    fn apply_effect(&mut self, effect: &StateEffect) -> Result<(), EffectError>;
    fn apply_effects(&mut self, effects: &[StateEffect]) -> Result<(), EffectError> {
        for effect in effects {
            self.apply_effect(effect)?;
        }
        Ok(())
    }
}

pub struct StateEffectApplicator;

impl StateEffectApplicator {
    pub fn apply_to_state(state: &mut SimState, effect: &StateEffect) -> Result<(), EffectError> {
        match effect {
            StateEffect::Financial(financial_effect) => Self::apply_financial_effect(state, financial_effect),
            StateEffect::Inventory(inventory_effect) => Self::apply_inventory_effect(state, inventory_effect),
            StateEffect::Market(market_effect) => Self::apply_market_effect(state, market_effect),
            StateEffect::Agent(agent_effect) => Self::apply_agent_effect(state, agent_effect),
        }
    }

    fn apply_financial_effect(state: &mut SimState, effect: &FinancialEffect) -> Result<(), EffectError> {
        match effect {
            FinancialEffect::CreateInstrument(inst) => state
                .financial_system
                .create_or_consolidate_instrument(inst.clone())
                .map(|_| ())
                .map_err(EffectError::FinancialSystemError),

            FinancialEffect::UpdateInstrument { id, new_principal } => state
                .financial_system
                .update_instrument(id, *new_principal)
                .map_err(|e| EffectError::FinancialSystemError(e)),

            FinancialEffect::TransferInstrument { id, new_creditor } => state
                .financial_system
                .transfer_instrument(id, *new_creditor)
                .map_err(|e| EffectError::FinancialSystemError(e)),

            FinancialEffect::RemoveInstrument(id) => {
                state.financial_system.remove_instrument(id).map_err(|e| EffectError::FinancialSystemError(e))
            }

            FinancialEffect::SwapInstrument { id, new_debtor, new_creditor } => state
                .financial_system
                .swap_instrument(id, new_debtor, new_creditor)
                .map_err(|e| EffectError::FinancialSystemError(e)),

            FinancialEffect::RecordTransaction(tx) => {
                state.history.transactions.push(tx.clone());
                Ok(())
            }
        }
    }

    fn apply_inventory_effect(state: &mut SimState, effect: &InventoryEffect) -> Result<(), EffectError> {
        match effect {
            InventoryEffect::AddInventory { owner, good_id, quantity, unit_cost } => {
                let bs = state.financial_system.balance_sheets.get_mut(owner).ok_or(EffectError::AgentNotFound { id: *owner })?;
                bs.add_to_inventory(good_id, *quantity, *unit_cost);
                Ok(())
            }
            InventoryEffect::RemoveInventory { owner, good_id, quantity } => {
                let bs = state.financial_system.balance_sheets.get_mut(owner).ok_or(EffectError::AgentNotFound { id: *owner })?;
                bs.remove_from_inventory(good_id, *quantity).map_err(EffectError::FinancialSystemError)
            }
        }
    }

    fn apply_market_effect(state: &mut SimState, effect: &MarketEffect) -> Result<(), EffectError> {
        match effect {
            MarketEffect::PlaceOrderInBook { market_id, order } => {
                match market_id {
                    MarketId::Goods(good_id) => {
                        let market = state
                            .financial_system
                            .exchange
                            .goods_market_mut(good_id)
                            .ok_or_else(|| EffectError::MarketNotFound { market: format!("Goods({})", good_id.0) })?;
                        match order {
                            Order::Bid(bid) => market.order_book.bids.push(bid.clone()),
                            Order::Ask(ask) => market.order_book.asks.push(ask.clone()),
                        }
                    }
                    MarketId::Financial(fin_market_id) => {
                        let market = state
                            .financial_system
                            .exchange
                            .financial_market_mut(fin_market_id)
                            .ok_or_else(|| EffectError::MarketNotFound { market: format!("Financial({:?})", fin_market_id) })?;
                        match order {
                            Order::Bid(bid) => market.order_book.bids.push(bid.clone()),
                            Order::Ask(ask) => market.order_book.asks.push(ask.clone()),
                        }
                    }
                    MarketId::Labour(_) => {
                        return Err(EffectError::UnimplementedAction("Labour market not implemented".to_string()))
                    }
                }
                Ok(())
            }
            MarketEffect::ExecuteTrade(trade) => {
                println!("[EFFECT] Executing Trade: {} buys {} of {:?} from {} @ ${}", trade.buyer, trade.quantity, trade.market_id, trade.seller, trade.price);
                Ok(())
            }
            MarketEffect::UpdatePrice { .. } | MarketEffect::ClearMarket { .. } => {
                Ok(())
            }
        }
    }

    fn apply_agent_effect(state: &mut SimState, effect: &AgentEffect) -> Result<(), EffectError> {
        match effect {
            AgentEffect::Hire { firm, count } => {
                if state.agents.get_agent_by_id(firm.0) {
                    println!("[EFFECT] Firm {} hiring {} agents", firm, count);
                    Ok(())
                } else {
                    Err(EffectError::FirmNotFound { id: *firm })
                }
            }
            AgentEffect::UpdateIncome { id, new_income } => {
                if let Some(consumer) = state.agents.get_agent_by_id_mut(id.clone()) {
                    consumer.income = *new_income;
                    Ok(())
                } else {
                    Err(EffectError::AgentNotFound { id: *id })
                }
            }
            AgentEffect::UpdateRevenue { id, revenue } => {
                 let tx = Transaction {
                    id: uuid::Uuid::new_v4(),
                    date: state.ticknum,
                    qty: *revenue,
                    from: *id, // Source is abstract
                    to: *id,
                    tx_type: TransactionType::Transfer { from: *id, to: *id, amount: *revenue },
                    instrument_id: None,
                };
                state.history.transactions.push(tx);
                Ok(())
            }
            AgentEffect::Produce { firm, good_id, amount } => {
                 println!("[EFFECT] Firm {} producing {} of {:?}", firm, amount, good_id);
                 Ok(())
            }
        }
    }
}

impl EffectApplicator for SimState {
    fn apply_effect(&mut self, effect: &StateEffect) -> Result<(), EffectError> {
        StateEffectApplicator::apply_to_state(self, effect)
    }
}
```

---

## `crates/core/sim_effects/src/effect_types.rs`

```rust
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
    pub fn name(&self) -> String {
        match self {
            StateEffect::Financial(effect) => format!("Financial::{}", effect.name()),
            StateEffect::Inventory(effect) => format!("Inventory::{}", effect.name()),
            StateEffect::Market(effect) => format!("Market::{}", effect.name()),
            StateEffect::Agent(effect) => format!("Agent::{}", effect.name()),
        }
    }
}
```

---

## `crates/core/sim_effects/src/financial.rs`

```rust
use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FinancialEffect {
    CreateInstrument(FinancialInstrument),
    UpdateInstrument { id: InstrumentId, new_principal: f64 },
    TransferInstrument { id: InstrumentId, new_creditor: AgentId },
    RemoveInstrument(InstrumentId),
    SwapInstrument { id: InstrumentId, new_debtor: AgentId, new_creditor: AgentId },
    RecordTransaction(Transaction),
}

impl FinancialEffect {
    pub fn name(&self) -> &'static str {
        match self {
            FinancialEffect::CreateInstrument(_) => "CreateInstrument",
            FinancialEffect::UpdateInstrument { .. } => "UpdateInstrument",
            FinancialEffect::TransferInstrument { .. } => "TransferInstrument",
            FinancialEffect::RemoveInstrument(_) => "RemoveInstrument",
            FinancialEffect::SwapInstrument { .. } => "SwapInstrument",
            FinancialEffect::RecordTransaction(_) => "RecordTransaction",
        }
    }
}
```

---

## `crates/core/sim_effects/src/inventory.rs`

```rust
use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InventoryEffect {
    AddInventory { owner: AgentId, good_id: GoodId, quantity: f64, unit_cost: f64 },
    RemoveInventory { owner: AgentId, good_id: GoodId, quantity: f64 },
}

impl InventoryEffect {
    pub fn name(&self) -> &'static str {
        match self {
            InventoryEffect::AddInventory { .. } => "AddInventory",
            InventoryEffect::RemoveInventory { .. } => "RemoveInventory",
        }
    }
}

```

---

## `crates/core/sim_effects/src/lib.rs`

```rust
//! Effect types and application logic
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


```

---

## `crates/core/sim_effects/src/market.rs`

```rust
use sim_types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MarketEffect {
    PlaceOrderInBook { market_id: MarketId, order: Order },
    ExecuteTrade(Trade),
    UpdatePrice { market_id: MarketId, new_price: f64 },
    ClearMarket { market_id: MarketId },
}

impl MarketEffect {
    pub fn name(&self) -> &'static str {
        match self {
            MarketEffect::PlaceOrderInBook { .. } => "PlaceOrderInBook",
            MarketEffect::ExecuteTrade(_) => "ExecuteTrade",
            MarketEffect::UpdatePrice { .. } => "UpdatePrice",
            MarketEffect::ClearMarket { .. } => "ClearMarket",
        }
    }
}
```

---

## `crates/core/sim_prelude/Cargo.toml`

```toml
[package]
name = "sim_prelude"
version = "0.1.0"
edition = "2024"

[dependencies]
# Depend on all the core crates whose items you want to re-export.
sim_types = { path = "../sim_types" }
sim_actions = { path = "../sim_actions" }
sim_effects = { path = "../sim_effects" }
sim_traits = { path = "../sim_traits" }
sim_decisions = { path = "../sim_decisions" }
```

---

## `crates/core/sim_prelude/src/lib.rs`

```rust
pub use sim_types::{*};
pub use sim_actions::{
    SimAction,
    BankingAction,
    ConsumptionAction,
    ProductionAction,
    TradingAction,
    ActionValidator,
    Validator,
};

pub use sim_effects::{
    StateEffect,
    AgentEffect,
    FinancialEffect,
    InventoryEffect,
    MarketEffect,
    EffectApplicator,
    EffectError,
};

pub use sim_traits::{
    DecisionMaker,
};

pub use sim_decisions::{
    DecisionModel,
    BankDecision,
    ConsumerDecision,
    FirmDecision,
    BankDecisionModel,
    BasicBankDecisionModel,
    ConsumerDecisionModel,
    BasicConsumerDecisionModel,
    FirmDecisionModel,
    BasicFirmDecisionModel,
    MLDecisionModel,
    FeatureSource,
    SpendingPredictor,
};
```

---

## `crates/core/sim_traits/Cargo.toml`

```toml
[package]
name = "sim_traits"
version = "0.1.0"  
edition = "2024"

[dependencies]
sim_types = { path = "../sim_types" }
serde = { workspace = true }


```

---

## `crates/core/sim_traits/src/action_handler.rs`

```rust


```

---

## `crates/core/sim_traits/src/decision_maker.rs`

```rust
//! Decision-making traits

use sim_types::*;

pub trait DecisionMaker {
    type Decision;
    
    fn decide(&self, state: &SimState) -> Vec<Self::Decision>;
}


```

---

## `crates/core/sim_traits/src/domain.rs`

```rust
//! Domain execution traits

// TODO: Define domain traits


```

---

## `crates/core/sim_traits/src/effect_applicator.rs`

```rust
//! Effect application traits

// TODO: Define effect application traits


```

---

## `crates/core/sim_traits/src/lib.rs`

```rust
//! Shared behavioral contracts and traits

pub mod decision_maker;
pub mod action_handler;
pub mod effect_applicator;
pub mod domain;

pub use decision_maker::*;


```

---

## `crates/core/sim_types/Cargo.toml`

```toml
[package]
name = "sim_types"
version = "0.1.0"
edition = "2024"

[dependencies]
uuid = { workspace = true }
serde = { workspace = true }
serde_with = { workspace = true }
chrono = { workspace = true }
sscanf = { workspace = true }
thiserror = { workspace = true }
dyn-clone = { workspace = true }
typetag = { workspace = true }
once_cell = { workspace = true }
toml = { workspace = true }

```

---

## `crates/core/sim_types/src/agents.rs`

```rust
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bank {
    pub id: AgentId,
    pub name: String,
    pub lending_spread: f64,
    pub deposit_spread: f64,
}

impl Bank {
    pub fn new(name: String, lending_spread: f64, deposit_spread: f64) -> Self {
        Self { id: AgentId(uuid::Uuid::new_v4()), name, lending_spread, deposit_spread }
    }

    pub fn total_liabilities(&self, fs: &FinancialSystem) -> f64 {
        fs.get_total_liabilities(&self.id)
    }

    pub fn get_reserves(&self, fs: &FinancialSystem) -> f64 {
        fs.get_bank_reserves(&self.id).unwrap_or(0.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Consumer {
    pub id: AgentId,
    pub age: u32,
    pub bank_id: AgentId,
    pub income: f64, // Annual income
    pub personality: PersonalityArchetype,
}

impl Consumer {
    pub fn new(age: u32, bank_id: AgentId, personality: PersonalityArchetype) -> Self {
        Self { id: AgentId(uuid::Uuid::new_v4()), age, bank_id, income: 0.0, personality }
    }

    pub fn get_cash_holdings(&self, fs: &FinancialSystem) -> f64 {
        fs.get_cash_assets(&self.id)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Firm {
    pub id: AgentId,
    pub bank_id: AgentId,
    pub name: String,
    pub employees: Vec<AgentId>,
    pub wage_rate: f64, // Hourly wage
    pub productivity: f64,
    pub recipe: Option<RecipeId>,
}

impl Firm {
    pub fn new(bank_id: AgentId, name: String, recipe: Option<RecipeId>) -> Self {
        Self {
            id: AgentId(uuid::Uuid::new_v4()),
            bank_id,
            name,
            employees: Vec::new(),
            wage_rate: 25.0,
            productivity: 1.0,
            recipe,
        }
    }

    pub fn get_employees(&self) -> &Vec<AgentId> {
        &self.employees
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CentralBank {
    pub id: AgentId,
    pub policy_rate: f64, // e.g., 0.05 for 5%
    pub reserve_requirement: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PersonalityArchetype {
    Balanced,
    Spender,
    Saver,
}
```

---

## `crates/core/sim_types/src/balance_sheet.rs`

```rust
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;
use uuid::Uuid;
use crate::*;

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub agent_id: AgentId,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub assets: HashMap<InstrumentId, FinancialInstrument>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub liabilities: HashMap<InstrumentId, FinancialInstrument>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub real_assets: HashMap<AssetId, RealAsset>,
}

impl BalanceSheet {
    pub fn new(owner: AgentId) -> Self {
        Self { agent_id: owner, assets: HashMap::new(), liabilities: HashMap::new(), real_assets: HashMap::new() }
    }

    pub fn liquid_assets(&self) -> f64 {
        self.assets
            .values()
            .filter(|inst| {
                inst.details.as_any().is::<CashDetails>() || inst.details.as_any().is::<DemandDepositDetails>()
            })
            .map(|inst| inst.principal)
            .sum()
    }

    pub fn deposits_at_bank(&self, bank_id: &AgentId) -> f64 {
        self.assets
            .values()
            .filter(|inst| {
                inst.debtor == *bank_id && (
                    inst.details.as_any().is::<DemandDepositDetails>() 
                    || inst.details.as_any().is::<SavingsDepositDetails>()
                )
            })
            .map(|inst| inst.principal)
            .sum()
    }

    pub fn total_assets(&self) -> f64 {
        let financial = self.assets.values().map(|inst| inst.principal).sum::<f64>();
        let real = self.real_assets.values().map(|asset| asset.market_value).sum::<f64>();
        financial + real
    }

    pub fn total_liabilities(&self) -> f64 {
        self.liabilities.values().map(|inst| inst.principal).sum()
    }

    pub fn net_worth(&self) -> f64 {
        self.total_assets() - self.total_liabilities()
    }
}
pub trait BalanceSheetQuery {
    fn get_bs_by_id(&self, agent_id: &AgentId) -> Option<&BalanceSheet>;
    fn get_bs_mut_by_id(&mut self, agent_id: &AgentId) -> Option<&mut BalanceSheet>;
    fn get_total_assets(&self, agent_id: &AgentId) -> f64;
    fn get_total_liabilities(&self, agent_id: &AgentId) -> f64;
    fn get_liquid_assets(&self, agent_id: &AgentId) -> f64;
    fn get_deposits_at_bank(&self, agent_id: &AgentId, bank_id: &AgentId) -> f64;
    fn get_cash_assets(&self, agent_id: &AgentId) -> f64;
    fn liquidity(&self, agent_id: &AgentId) -> f64;
    fn get_bank_reserves(&self, agent_id: &AgentId) -> Option<f64>;
}

impl BalanceSheetQuery for FinancialSystem {
    fn get_bs_by_id(&self, agent_id: &AgentId) -> Option<&BalanceSheet> {
        self.balance_sheets.get(agent_id)
    }
    fn get_bs_mut_by_id(&mut self, agent_id: &AgentId) -> Option<&mut BalanceSheet> {
        self.balance_sheets.get_mut(agent_id)
    }
    fn get_total_assets(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map(|bs| bs.total_assets()).unwrap_or(0.0)
    }
    fn get_cash_assets(&self, agent_id: &AgentId) -> f64 {
        self.get_bs_by_id(agent_id)
            .map(|bs| {
                bs.assets
                    .values()
                    .filter(|inst| inst.details.as_any().is::<CashDetails>())
                    .map(|inst| inst.principal)
                    .sum::<f64>()
            })
            .unwrap_or(0.0)
    }
    fn get_total_liabilities(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map(|bs| bs.total_liabilities()).unwrap_or(0.0)
    }
    fn get_liquid_assets(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map(|bs| bs.liquid_assets()).unwrap_or(0.0)
    }
    fn get_deposits_at_bank(&self, agent_id: &AgentId, bank_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map(|bs| bs.deposits_at_bank(bank_id)).unwrap_or(0.0)
    }
    fn liquidity(&self, agent_id: &AgentId) -> f64 {
        self.balance_sheets.get(agent_id).map(|bs| bs.liquid_assets()).unwrap_or(0.0)
    }
    fn get_bank_reserves(&self, agent_id: &AgentId) -> Option<f64> {
        self.balance_sheets.get(agent_id).map(|bs| {
            bs.assets
                .values()
                .filter(|inst| inst.details.as_any().is::<CentralBankReservesDetails>())
                .map(|inst| inst.principal)
                .sum::<f64>()
        })
    }
}

pub trait InventoryQuery {
    fn update_inventory_market_value(&mut self);
    fn get_or_create_inventory_mut(&mut self) -> &mut HashMap<GoodId, InventoryItem>;
    fn get_inventory(&self) -> Option<&HashMap<GoodId, InventoryItem>>;
    fn add_to_inventory(&mut self, good_id: &GoodId, quantity: f64, unit_cost: f64);
    fn remove_from_inventory(&mut self, good_id: &GoodId, quantity: f64) -> Result<(), String>;
}

impl InventoryQuery for BalanceSheet {
    fn update_inventory_market_value(&mut self) {
        let mut inventory_value = 0.0;
        let mut inventory_asset_id: Option<AssetId> = None;

        for asset in self.real_assets.values() {
            if let RealAssetType::Inventory { goods } = &asset.asset_type {
                inventory_asset_id = Some(asset.id);
                inventory_value = goods.values().map(|item| item.quantity * item.unit_cost).sum();
                break;
            }
        }

        if let Some(id) = inventory_asset_id {
            if let Some(asset) = self.real_assets.get_mut(&id) {
                asset.market_value = inventory_value;
            }
        }
    }
    fn get_inventory(&self) -> Option<&HashMap<GoodId, InventoryItem>> {
        let inventory_asset_id = self
            .real_assets
            .values()
            .find(|asset| matches!(asset.asset_type, RealAssetType::Inventory { .. }))
            .map(|asset| asset.id);

        if let Some(id) = inventory_asset_id {
            if let RealAssetType::Inventory { goods } = &self.real_assets[&id].asset_type {
                return Some(goods);
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    fn get_or_create_inventory_mut(&mut self) -> &mut HashMap<GoodId, InventoryItem> {
        let inventory_asset_id = self
            .real_assets
            .values()
            .find(|asset| matches!(asset.asset_type, RealAssetType::Inventory { .. }))
            .map(|asset| asset.id);

        let id_to_use = inventory_asset_id.unwrap_or_else(|| {
            let new_inventory_asset = RealAsset {
                id: AssetId(Uuid::new_v4()),
                asset_type: RealAssetType::Inventory { goods: HashMap::new() },
                owner: self.agent_id,
                market_value: 0.0,
                acquired_date: 0,
            };
            let new_id = new_inventory_asset.id;
            self.real_assets.insert(new_id, new_inventory_asset);
            new_id
        });

        if let RealAssetType::Inventory { goods } = &mut self.real_assets.get_mut(&id_to_use).unwrap().asset_type {
            goods
        } else {
            unreachable!();
        }
    }

    fn add_to_inventory(&mut self, good_id: &GoodId, quantity: f64, unit_cost: f64) {
        let inventory = self.get_or_create_inventory_mut();
        let item = inventory.entry(*good_id).or_insert(InventoryItem { quantity: 0.0, unit_cost: 0.0 });

        let new_total_quantity = item.quantity + quantity;
        if new_total_quantity > 0.0 {
            item.unit_cost = (item.quantity * item.unit_cost + quantity * unit_cost) / new_total_quantity;
        } else {
            item.unit_cost = 0.0;
        }
        item.quantity = new_total_quantity;

        self.update_inventory_market_value();
    }

    fn remove_from_inventory(&mut self, good_id: &GoodId, quantity: f64) -> Result<(), String> {
        let inventory = self.get_or_create_inventory_mut();
        if let Some(item) = inventory.get_mut(good_id) {
            if item.quantity >= quantity {
                item.quantity -= quantity;
                self.update_inventory_market_value();
                Ok(())
            } else {
                Err(format!(
                    "Insufficient inventory for good {:?}: have {}, need {}",
                    good_id.0, item.quantity, quantity
                ))
            }
        } else {
            Err(format!("No inventory for good {:?}", good_id.0))
        }
    }
}
```

---

## `crates/core/sim_types/src/goods.rs`

```rust
use crate::*;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml;
use uuid::Uuid;
use serde_with::{serde_as, DisplayFromStr};

const GOODS_NAMESPACE: Uuid = Uuid::from_u128(0x4A8B382D22C14A4C8F1A2E3D4B5C6F7A);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Good {
    pub id: GoodId,
    pub name: String,
    pub unit: String,
    pub category: GoodCategory,
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy)]
pub enum GoodCategory {
    RawMaterial,
    IntermediateGood,
    FinalGood,
    Energy,
    Service,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionRecipe {
    pub id: RecipeId,
    pub name: String,
    pub inputs: Vec<(GoodId, f64)>,
    pub output: (GoodId, f64),
    pub labour_hours: f64,
    pub capital_required: f64,
    pub efficiency: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InventoryItem {
    pub quantity: f64,
    pub unit_cost: f64,
}

impl GoodId {
    pub fn from_slug(slug: &str) -> Self {
        Self(Uuid::new_v5(&GOODS_NAMESPACE, slug.as_bytes()))
    }
}

impl RecipeId {
    pub fn from_name(name: &str) -> Self {
        Self(Uuid::new_v5(&GOODS_NAMESPACE, name.as_bytes()))
    }
}

#[derive(Debug, Deserialize)]
struct TomlConfig {
    goods: Vec<TomlGood>,
    recipes: Vec<TomlRecipe>,
}

#[derive(Debug, Deserialize)]
struct TomlGood {
    slug: String,
    name: String,
    unit: String,
    category: GoodCategory,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TomlRecipe {
    name: String,
    output: TomlRecipeItem,
    inputs: Vec<TomlRecipeItem>,
    labour_hours: f64,
    capital_required: f64,
    efficiency: f64,
}

#[derive(Debug, Deserialize)]
struct TomlRecipeItem {
    slug: String,
    qty: f64,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoodsRegistry {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub goods: HashMap<GoodId, Good>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub recipes: HashMap<RecipeId, ProductionRecipe>,
    #[serde(skip)]
    slug_to_id: HashMap<String, GoodId>,
    #[serde(skip)]
    name_to_recipe_id: HashMap<String, RecipeId>,
}

impl Default for GoodsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl GoodsRegistry {
    pub fn new() -> Self {
        Self {
            goods: HashMap::new(),
            recipes: HashMap::new(),
            slug_to_id: HashMap::new(),
            name_to_recipe_id: HashMap::new(),
        }
    }

    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        let config: TomlConfig = toml::from_str(toml_str)?;
        let mut registry = Self::new();

        for good_def in config.goods {
            let id = GoodId::from_slug(&good_def.slug);
            let good = Good { id, name: good_def.name, unit: good_def.unit, category: good_def.category };
            registry.goods.insert(id, good);
            registry.slug_to_id.insert(good_def.slug, id);
        }

        for recipe_def in config.recipes {
            let recipe_id = RecipeId::from_name(&recipe_def.name);

            let output_good_id = registry.get_good_id_by_slug(&recipe_def.output.slug).unwrap_or_else(|| {
                panic!("Output good '{}' for recipe '{}' not found", recipe_def.output.slug, recipe_def.name)
            });

            let inputs = recipe_def
                .inputs
                .iter()
                .map(|input_def| {
                    let input_good_id = registry.get_good_id_by_slug(&input_def.slug).unwrap_or_else(|| {
                        panic!("Input good '{}' for recipe '{}' not found", input_def.slug, recipe_def.name)
                    });
                    (input_good_id, input_def.qty)
                })
                .collect();

            let recipe = ProductionRecipe {
                id: recipe_id,
                name: recipe_def.name.clone(),
                inputs,
                output: (output_good_id, recipe_def.output.qty),
                labour_hours: recipe_def.labour_hours,
                capital_required: recipe_def.capital_required,
                efficiency: recipe_def.efficiency,
            };
            registry.recipes.insert(recipe_id, recipe.clone());
            registry.name_to_recipe_id.insert(recipe_def.name, recipe_id);
        }

        Ok(registry)
    }

    pub fn get_good_id_by_slug(&self, slug: &str) -> Option<GoodId> {
        self.slug_to_id.get(slug).copied()
    }

    pub fn get_recipe_id_by_name(&self, name: &str) -> Option<RecipeId> {
        self.name_to_recipe_id.get(name).copied()
    }

    pub fn get_good_name(&self, id: &GoodId) -> Option<&str> {
        self.goods.get(id).map(|good| good.name.as_str())
    }

    pub fn get_recipe(&self, id: &RecipeId) -> Option<&ProductionRecipe> {
        self.recipes.get(id)
    }
}

pub static CATALOGUE: Lazy<GoodsRegistry> = Lazy::new(|| {
    GoodsRegistry::from_toml(include_str!("../../../../config/goods.toml")).expect("failed to parse goods catalogue")
});
```

---

## `crates/core/sim_types/src/ids.rs`

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::pserde;

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, Copy, Default)]
pub struct AgentId(pub Uuid);
pserde!(AgentId, Uuid);

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, Copy, Default)]
pub struct InstrumentId(pub Uuid);
pserde!(InstrumentId, Uuid);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy, Default)]
pub struct AssetId(pub Uuid);
pserde!(AssetId, Uuid);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy, Default)]
pub struct GoodId(pub Uuid);
pserde!(GoodId, Uuid);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy, Default)]
pub struct RecipeId(pub Uuid);
pserde!(RecipeId, Uuid);
```

---

## `crates/core/sim_types/src/instruments.rs`

```rust
use crate::*;
use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use dyn_clone::{clone_trait_object, DynClone};
use std::any::Any;
use std::fmt::Debug;
use std::str::FromStr;
use std::fmt;
use thiserror::Error;

#[typetag::serde(tag = "instrument_details_type")]
pub trait InstrumentDetails: DynClone + Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}
clone_trait_object!(InstrumentDetails);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialInstrument {
    pub id: InstrumentId,
    pub debtor: AgentId,
    pub creditor: AgentId,
    pub principal: f64,
    pub originated_date: NaiveDate,
    pub details: Box<dyn InstrumentDetails>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CashDetails;
#[typetag::serde]
impl InstrumentDetails for CashDetails {
    fn as_any(&self) -> &dyn Any { self }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemandDepositDetails {
    pub interest_rate: f64,
}
#[typetag::serde]
impl InstrumentDetails for DemandDepositDetails {
    fn as_any(&self) -> &dyn Any { self }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavingsDepositDetails {
    pub interest_rate: f64,
}
#[typetag::serde]
impl InstrumentDetails for SavingsDepositDetails {
    fn as_any(&self) -> &dyn Any { self }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CentralBankReservesDetails;
#[typetag::serde]
impl InstrumentDetails for CentralBankReservesDetails {
    fn as_any(&self) -> &dyn Any { self }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BondDetails {
    pub bond_type: BondType,
    pub coupon_rate: f64,
    pub face_value: f64,
    pub maturity_date: NaiveDate,
    pub frequency: usize,
}
#[typetag::serde]
impl InstrumentDetails for BondDetails {
    fn as_any(&self) -> &dyn Any { self }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoanDetails {
    pub loan_type: LoanType,
    pub interest_rate: f64,
    pub maturity_date: NaiveDate,
    pub collateral: Option<CollateralInfo>,
}
#[typetag::serde]
impl InstrumentDetails for LoanDetails {
    fn as_any(&self) -> &dyn Any { self }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BondType {
    Corporate { spread: f64 },
    Government,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum LoanType {
    Mortgage,
    Personal,
    Auto,
    Student,
    CreditCard,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CollateralInfo {
    pub collateral_type: String,
    pub value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CreditRating {
    AAA, AA, A, BBB, BB, B, CCC, CC, C, D,
}

impl fmt::Display for CreditRating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Error)]
#[error("Invalid CreditRating string: {0}")]
pub struct ParseCreditRatingError(String);

impl FromStr for CreditRating {
    type Err = ParseCreditRatingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AAA" => Ok(CreditRating::AAA),
            "AA" => Ok(CreditRating::AA),
            "A" => Ok(CreditRating::A),
            "BBB" => Ok(CreditRating::BBB),
            "BB" => Ok(CreditRating::BB),
            "B" => Ok(CreditRating::B),
            "CCC" => Ok(CreditRating::CCC),
            "CC" => Ok(CreditRating::CC),
            "C" => Ok(CreditRating::C),
            "D" => Ok(CreditRating::D),
            _ => Err(ParseCreditRatingError(s.to_string())),
        }
    }
}

impl Default for FinancialInstrument {
    fn default() -> Self {
        Self {
            id: Default::default(),
            debtor: Default::default(),
            creditor: Default::default(),
            principal: 0.0,
            originated_date: chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            details: Box::new(CashDetails),
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConsolidationKey {
    pub creditor: AgentId,
    pub debtor: AgentId,
    pub instrument_type: String,
    pub subtype: Option<String>,
}

pub trait Consolidatable {
    fn consolidation_key(&self) -> Option<ConsolidationKey>;
}

impl Consolidatable for FinancialInstrument {
    fn consolidation_key(&self) -> Option<ConsolidationKey> {
        if self.details.as_any().is::<CashDetails>() {
            return Some(ConsolidationKey {
                creditor: self.creditor,
                debtor: self.debtor,
                instrument_type: "Cash".to_string(),
                subtype: None,
            });
        }
        if self.details.as_any().is::<CentralBankReservesDetails>() {
            return Some(ConsolidationKey {
                creditor: self.creditor,
                debtor: self.debtor,
                instrument_type: "Reserves".to_string(),
                subtype: None,
            });
        }
        if let Some(details) = self.details.as_any().downcast_ref::<DemandDepositDetails>() {
            return Some(ConsolidationKey {
                creditor: self.creditor,
                debtor: self.debtor,
                instrument_type: "DemandDeposit".to_string(),
                subtype: Some(format!("rate_{}", (details.interest_rate * 10000.0) as i32)),
            });
        }
        None
    }
}
```

---

## `crates/core/sim_types/src/lib.rs`

```rust
pub mod ids;
pub mod agents;
pub mod instruments;
pub mod goods;
pub mod markets;
pub mod state;
pub mod time;
pub mod balance_sheet;
pub mod macros;
// Re-export commonly used types
pub use ids::*;
pub use agents::*;
pub use instruments::*;
pub use goods::*;
pub use markets::*;
pub use state::*;
pub use time::*;
pub use balance_sheet::*;
```

---

## `crates/core/sim_types/src/macros.rs`

```rust
#[macro_export]
macro_rules! good_id {
    ($slug:literal) => {
        $crate::goods::CATALOGUE
            .get_good_id_by_slug($slug)
            .expect(concat!("unknown good slug: ", $slug))
    };
}

#[macro_export]
macro_rules! recipe_id {
    ($name:literal) => {
        sim_types::goods::CATALOGUE
            .get_recipe_id_by_name($name)
            .expect(concat!("unknown recipe name: ", $name))
    };
}

#[macro_export]
macro_rules! cash {
    ($creditor:expr, $amount:expr, $cb_id:expr, $originated:expr) => {
        $crate::FinancialInstrument {
            id: $crate::InstrumentId(uuid::Uuid::new_v4()),
            creditor: $creditor,
            debtor: $cb_id,
            principal: $amount,
            details: Box::new($crate::CashDetails),
            originated_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! deposit {
    ($depositor:expr, $bank:expr, $amount:expr, $rate:expr, $originated:expr) => {
        $crate::FinancialInstrument {
            id: $crate::InstrumentId(uuid::Uuid::new_v4()),
            creditor: $depositor,
            debtor: $bank,
            principal: $amount,
            details: Box::new($crate::DemandDepositDetails { interest_rate: $rate }),
            originated_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! reserves {
    ($bank:expr, $cb_id:expr, $amount:expr, $originated:expr) => {
        $crate::FinancialInstrument {
            id: $crate::InstrumentId(uuid::Uuid::new_v4()),
            creditor: $bank,
            debtor: $cb_id,
            principal: $amount,
            details: Box::new($crate::CentralBankReservesDetails),
            originated_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! bond {
    ($investor:expr, $issuer:expr, $principal:expr, $coupon_rate:expr, $maturity_date:expr, $face_value:expr, $bond_type:expr, $frequency:expr, $originated:expr) => {
        $crate::FinancialInstrument {
            id: $crate::InstrumentId(uuid::Uuid::new_v4()),
            creditor: $investor,
            debtor: $issuer,
            principal: $principal,
            details: Box::new($crate::BondDetails {
                bond_type: $bond_type,
                coupon_rate: $coupon_rate,
                face_value: $face_value,
                maturity_date: $maturity_date,
                frequency: $frequency,
            }),
            originated_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! pserde {
    ($outer:ty, $inner:ty) => {
        impl std::fmt::Display for $outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        
        impl std::str::FromStr for $outer {
            type Err = <$inner as std::str::FromStr>::Err;
            
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.parse::<$inner>()?))
            }
        }
    };
}
```

---

## `crates/core/sim_types/src/markets.rs`

```rust
use crate::*;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy)]
pub enum Tenor {
    T2Y, T5Y, T10Y, T30Y,
}
impl Tenor {
    pub fn to_days(&self) -> u32 {
        match self {
            Tenor::T2Y => 730,
            Tenor::T5Y => 1825,
            Tenor::T10Y => 3650,
            Tenor::T30Y => 10950,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FinancialMarketId {
    SecuredOvernightFinancing,
    Treasury { tenor: Tenor },
    CorporateBond { rating: CreditRating },
}

impl fmt::Display for FinancialMarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FinancialMarketId::SecuredOvernightFinancing => write!(f, "SOFR"),
            FinancialMarketId::Treasury { tenor } => write!(f, "Treasury_{}", tenor),
            FinancialMarketId::CorporateBond { rating } => write!(f, "CorpBond_{}", rating),
        }
    }
}

#[derive(Debug, Error)]
pub enum ParseFinancialMarketIdError {
    #[error("Invalid FinancialMarketId string format: {0}")]
    InvalidFormat(String),
    #[error("Failed to parse tenor: {0}")]
    ParseTenor(#[from] ParseTenorError),
    #[error("Failed to parse credit rating: {0}")]
    ParseRating(#[from] ParseCreditRatingError),
}

impl FromStr for FinancialMarketId {
    type Err = ParseFinancialMarketIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "SOFR" {
            return Ok(FinancialMarketId::SecuredOvernightFinancing);
        }
        if let Some(tenor_str) = s.strip_prefix("Treasury_") {
            let tenor = tenor_str.parse()?;
            return Ok(FinancialMarketId::Treasury { tenor });
        }
        if let Some(rating_str) = s.strip_prefix("CorpBond_") {
            let rating = rating_str.parse()?;
            return Ok(FinancialMarketId::CorporateBond { rating });
        }
        Err(ParseFinancialMarketIdError::InvalidFormat(s.to_string()))
    }
}


pub trait RatesMarket {
    fn price_to_daily_rate(&self, price: f64) -> f64;
    fn daily_rate_to_annual_bps(&self, daily_rate: f64) -> f64;
    fn annual_bps_to_daily_rate(&self, annual_bps: f64) -> f64;
}

impl RatesMarket for FinancialMarketId {
    fn price_to_daily_rate(&self, price: f64) -> f64 {
        if price <= 0.0 { return f64::INFINITY; }
        (1.0 / price) - 1.0
    }
    fn daily_rate_to_annual_bps(&self, daily_rate: f64) -> f64 {
        daily_rate * 360.0 * 10000.0
    }
    fn annual_bps_to_daily_rate(&self, annual_bps: f64) -> f64 {
        annual_bps / 10000.0 / 360.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LabourMarketId {
    Labour,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketId {
    Goods(GoodId),
    Financial(FinancialMarketId),
    Labour(LabourMarketId),
}

impl Default for MarketId {
    fn default() -> Self {
        MarketId::Goods(GoodId::default())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trade {
    pub market_id: MarketId,
    pub buyer: AgentId,
    pub seller: AgentId,
    pub quantity: f64,
    pub price: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bid {
    pub agent_id: AgentId,
    pub price: f64,
    pub quantity: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ask {
    pub agent_id: AgentId,
    pub price: f64,
    pub quantity: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Order {
    Bid(Bid),
    Ask(Ask),
}

impl Default for Order {
    fn default() -> Self {
        Order::Bid(Bid {
            agent_id: Default::default(),
            price: 0.0,
            quantity: 0.0,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrderBook {
    pub bids: Vec<Bid>,
    pub asks: Vec<Ask>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self { bids: Vec::new(), asks: Vec::new() }
    }

    pub fn best_bid(&self) -> Option<&Bid> {
        self.bids.iter().max_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
    }

    pub fn best_ask(&self) -> Option<&Ask> {
        self.asks.iter().min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
    }

    pub fn clear_and_match(&mut self, market_id: &MarketId) -> Vec<Trade> {
        let mut trades = Vec::new();
        self.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
        self.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

        let mut bid_idx = 0;
        let mut ask_idx = 0;

        while bid_idx < self.bids.len() && ask_idx < self.asks.len() {
            let bid = &mut self.bids[bid_idx];
            let ask = &mut self.asks[ask_idx];

            if bid.price >= ask.price {
                let trade_qty = bid.quantity.min(ask.quantity);
                let trade_price = (bid.price + ask.price) / 2.0;

                trades.push(Trade {
                    market_id: market_id.clone(),
                    buyer: bid.agent_id,
                    seller: ask.agent_id,
                    quantity: trade_qty,
                    price: trade_price,
                });

                bid.quantity -= trade_qty;
                ask.quantity -= trade_qty;

                if bid.quantity < 1e-6 { bid_idx += 1; }
                if ask.quantity < 1e-6 { ask_idx += 1; }
            } else {
                break;
            }
        }

        self.bids.retain(|b| b.quantity > 1e-6);
        self.asks.retain(|a| a.quantity > 1e-6);

        trades
    }
}

impl fmt::Display for Tenor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Error)]
#[error("Invalid Tenor string: {0}")]
pub struct ParseTenorError(String);

impl FromStr for Tenor {
    type Err = ParseTenorError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "T2Y" => Ok(Tenor::T2Y),
            "T5Y" => Ok(Tenor::T5Y),
            "T10Y" => Ok(Tenor::T10Y),
            "T30Y" => Ok(Tenor::T30Y),
            _ => Err(ParseTenorError(s.to_string())),
        }
    }
}
```

---

## `crates/core/sim_types/src/state.rs`

```rust
use crate::*;
use serde::{Deserialize, Serialize};
use std::{collections::{HashMap, HashSet}, hash::Hash};
use serde_with::{serde_as, DisplayFromStr};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimState {
    pub ticknum: u32,
    pub current_date: chrono::NaiveDate,
    pub financial_system: FinancialSystem,
    pub agents: AgentRegistry,
    pub config: SimConfig,
    pub history: SimHistory,
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            ticknum: 0,
            current_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            financial_system: FinancialSystem::default(),
            agents: AgentRegistry::default(),
            config: SimConfig::default(),
            history: SimHistory::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimConfig {
    pub iterations: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimHistory {
    pub transactions: Vec<Transaction>,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialSystem {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub instruments: HashMap<InstrumentId, FinancialInstrument>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub balance_sheets: HashMap<AgentId, BalanceSheet>,
    pub central_bank: CentralBank,
    pub exchange: Exchange,
    pub goods: GoodsRegistry,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AgentRegistry {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub banks: HashMap<AgentId, Bank>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub consumers: HashMap<AgentId, Consumer>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub firms: HashMap<AgentId, Firm>,
}
impl AgentRegistry {
    pub fn get_agent_by_id(&self, id: &AgentId) -> Option<&dyn Agent> {
        self.banks.get(id).or_else(|| self.consumers.get(id)).or_else(|| self.firms.get(id))
    }
    pub fn get_agent_by_id_mut(&mut self, id: &AgentId) -> Option<&mut dyn Agent> {
        self.banks.get_mut(id).or_else(|| self.consumers.get_mut(id)).or_else(|| self.firms.get_mut(id))
    }
    fn get_bank(&self, id: &AgentId) -> Option<&Bank> { self.banks.get(id) }
    fn get_consumer(&self, id: &AgentId) -> Option<&Consumer> { self.consumers.get(id) }
    fn get_firm(&self, id: &AgentId) -> Option<&Firm> { self.firms.get(id)}
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RealAsset {
    pub id: AssetId,
    pub asset_type: RealAssetType,
    pub owner: AgentId,
    pub market_value: f64,
    pub acquired_date: u32,
}

#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RealAssetType {
    RealEstate { address: String, property_type: String },
    Inventory {
        #[serde_as(as = "HashMap<DisplayFromStr, _>")]
        goods: HashMap<GoodId, InventoryItem>
    },
    Equipment { description: String, depreciation_rate: f64 },
    IntellectualProperty { description: String },
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Exchange {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub goods_markets: HashMap<GoodId, GoodsMarket>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub financial_markets: HashMap<FinancialMarketId, FinancialMarket>,
}

impl Exchange {
    pub fn register_goods_market(&mut self, good_id: GoodId, goods_registry: &GoodsRegistry) {
        let name = goods_registry.get_good_name(&good_id).unwrap_or("Unknown Good").to_string();
        self.goods_markets.entry(good_id).or_insert_with(|| GoodsMarket::new(good_id, name));
    }

    pub fn register_financial_market(&mut self, market_id: FinancialMarketId) {
        let name = match &market_id {
            FinancialMarketId::SecuredOvernightFinancing => "Secured Overnight Financing".to_string(),
            FinancialMarketId::Treasury { tenor } => format!("Treasury {}", tenor),
            FinancialMarketId::CorporateBond { rating } => format!("Corporate Bond {:?}", rating),
        };
        self.financial_markets.entry(market_id.clone()).or_insert_with(|| FinancialMarket::new(market_id, name));
    }

    pub fn goods_market(&self, good_id: &GoodId) -> Option<&GoodsMarket> {
        self.goods_markets.get(good_id)
    }

    pub fn goods_market_mut(&mut self, good_id: &GoodId) -> Option<&mut GoodsMarket> {
        self.goods_markets.get_mut(good_id)
    }

    pub fn financial_market(&self, market_id: &FinancialMarketId) -> Option<&FinancialMarket> {
        self.financial_markets.get(market_id)
    }

    pub fn financial_market_mut(&mut self, market_id: &FinancialMarketId) -> Option<&mut FinancialMarket> {
        self.financial_markets.get_mut(market_id)
    }

    pub fn clear_markets(&mut self) -> Vec<Trade> {
        let mut all_trades = Vec::new();
        for (id, market) in self.goods_markets.iter_mut() {
            all_trades.extend(market.order_book.clear_and_match(&MarketId::Goods(*id)));
        }
        for (id, market) in self.financial_markets.iter_mut() {
            all_trades.extend(market.order_book.clear_and_match(&MarketId::Financial(id.clone())));
        }
        all_trades
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoodsMarket {
    pub good_id: GoodId,
    pub name: String,
    pub order_book: OrderBook,
}

impl GoodsMarket {
    pub fn new(good_id: GoodId, name: String) -> Self {
        Self { good_id, name, order_book: OrderBook::new() }
    }

    pub fn best_ask(&self) -> Option<&Ask> {
        self.order_book.asks.iter().min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialMarket {
    pub market_id: FinancialMarketId,
    pub name: String,
    pub order_book: OrderBook,
}

impl FinancialMarket {
    pub fn new(market_id: FinancialMarketId, name: String) -> Self {
        Self { market_id, name, order_book: OrderBook::new() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub id: uuid::Uuid,
    pub date: u32,
    pub qty: f64,
    pub from: AgentId,
    pub to: AgentId,
    pub tx_type: TransactionType,
    pub instrument_id: Option<InstrumentId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionType {
    Deposit { holder: AgentId, bank: AgentId, amount: f64 },
    Withdrawal { holder: AgentId, bank: AgentId, amount: f64 },
    Transfer { from: AgentId, to: AgentId, amount: f64 },
    InterestPayment,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self { iterations: 100 }
    }
}

impl Default for SimHistory {
    fn default() -> Self {
        Self { transactions: Vec::new() }
    }
}

impl Default for FinancialSystem {
    fn default() -> Self {
        let central_bank =
            CentralBank { id: AgentId(uuid::Uuid::new_v4()), policy_rate: 430.0, reserve_requirement: 0.1 };
        let mut balance_sheets = HashMap::new();
        balance_sheets.insert(central_bank.id, BalanceSheet::new(central_bank.id));

        Self {
            instruments: HashMap::new(),
            balance_sheets,
            central_bank,
            exchange: Exchange::default(),
            goods: GoodsRegistry::new(),
        }
    }
}

impl Default for Exchange {
    fn default() -> Self {
        Self { goods_markets: HashMap::new(), financial_markets: HashMap::new() }
    }
}

impl SimState {
    pub fn advance_time(&mut self) {
        self.ticknum += 1;
        self.current_date = self.current_date + chrono::Duration::days(1);
    }
}

pub trait InstrumentManager {
    fn update_instrument(&mut self, id: &InstrumentId, new_principal: f64) -> Result<(), String>;
    fn create_instrument(&mut self, instrument: FinancialInstrument) -> Result<(), String>;
    fn create_or_consolidate_instrument(&mut self, instrument: FinancialInstrument) -> Result<InstrumentId, String>;
    fn find_consolidatable_instrument(&self, new_inst: &FinancialInstrument) -> Option<InstrumentId>;
    fn remove_instrument(&mut self, id: &InstrumentId) -> Result<(), String>;
    fn transfer_instrument(&mut self, id: &InstrumentId, new_creditor: AgentId) -> Result<(), String>;
    fn swap_instrument(
        &mut self, id: &InstrumentId, new_debtor: &AgentId, new_creditor: &AgentId,
    ) -> Result<(), String>;
}

pub trait FinancialStatistics {
    fn m0(&self) -> f64;
    fn m1(&self, agents: &AgentRegistry) -> f64;
    fn m2(&self, agents: &AgentRegistry) -> f64;
}

impl InstrumentManager for FinancialSystem {
    fn create_instrument(&mut self, instrument: FinancialInstrument) -> Result<(), String> {
        let id = instrument.id;

        self.balance_sheets
            .get_mut(&instrument.creditor)
            .ok_or("Creditor not found")?
            .assets
            .insert(id, instrument.clone());

        self.balance_sheets
            .get_mut(&instrument.debtor)
            .ok_or("Debtor not found")?
            .liabilities
            .insert(id, instrument.clone());

        self.instruments.insert(id, instrument);
        Ok(())
    }

    fn transfer_instrument(&mut self, instrument_id: &InstrumentId, new_creditor: AgentId) -> Result<(), String> {
        let instrument = self.instruments.get_mut(instrument_id).ok_or("Instrument not found")?;
        let old_creditor = instrument.creditor;

        self.balance_sheets.get_mut(&old_creditor).ok_or("Old creditor not found")?.assets.remove(instrument_id);

        instrument.creditor = new_creditor;
        self.balance_sheets
            .get_mut(&new_creditor)
            .ok_or("New creditor not found")?
            .assets
            .insert(*instrument_id, instrument.clone());

        Ok(())
    }

    fn find_consolidatable_instrument(&self, new_inst: &FinancialInstrument) -> Option<InstrumentId> {
        if let Some(key) = new_inst.consolidation_key() {
            if let Some(creditor_bs) = self.balance_sheets.get(&new_inst.creditor) {
                for (id, existing) in &creditor_bs.assets {
                    if existing.consolidation_key() == Some(key.clone()) {
                        return Some(*id);
                    }
                }
            }
        }
        None
    }

    fn create_or_consolidate_instrument(&mut self, instrument: FinancialInstrument) -> Result<InstrumentId, String> {
        if let Some(existing_id) = self.find_consolidatable_instrument(&instrument) {
            let principal_change = instrument.principal;
            let existing =
                self.instruments.get_mut(&existing_id).ok_or("Consolidatable instrument not found in main registry")?;
            existing.principal += principal_change;

            self.balance_sheets
                .get_mut(&existing.creditor)
                .and_then(|bs| bs.assets.get_mut(&existing_id))
                .map(|inst| inst.principal += principal_change);
            self.balance_sheets
                .get_mut(&existing.debtor)
                .and_then(|bs| bs.liabilities.get_mut(&existing_id))
                .map(|inst| inst.principal += principal_change);

            Ok(existing_id)
        } else {
            let id = instrument.id;
            self.create_instrument(instrument)?;
            Ok(id)
        }
    }

    fn update_instrument(&mut self, id: &InstrumentId, new_principal: f64) -> Result<(), String> {
        let instrument = self.instruments.get_mut(id).ok_or("Instrument not found")?;
        instrument.principal = new_principal;

        self.balance_sheets
            .get_mut(&instrument.creditor)
            .and_then(|bs| bs.assets.get_mut(id))
            .map(|inst| inst.principal = new_principal);
        self.balance_sheets
            .get_mut(&instrument.debtor)
            .and_then(|bs| bs.liabilities.get_mut(id))
            .map(|inst| inst.principal = new_principal);

        Ok(())
    }

    fn remove_instrument(&mut self, id: &InstrumentId) -> Result<(), String> {
        if let Some(instrument) = self.instruments.remove(id) {
            self.balance_sheets.get_mut(&instrument.creditor).and_then(|bs| bs.assets.remove(id));
            self.balance_sheets.get_mut(&instrument.debtor).and_then(|bs| bs.liabilities.remove(id));
            Ok(())
        } else {
            Err("Instrument not found".to_string())
        }
    }

    fn swap_instrument(
        &mut self, id: &InstrumentId, new_debtor: &AgentId, new_creditor: &AgentId,
    ) -> Result<(), String> {
        let instrument = self.instruments.get_mut(id).ok_or("Instrument not found")?;
        let old_debtor = instrument.debtor;
        let old_creditor = instrument.creditor;

        instrument.debtor = *new_debtor;
        instrument.creditor = *new_creditor;

        if let Some(liability) = self.balance_sheets.get_mut(&old_debtor).and_then(|bs| bs.liabilities.remove(id)) {
            self.balance_sheets.get_mut(new_debtor).and_then(|bs| bs.liabilities.insert(*id, liability));
        }

        if let Some(asset) = self.balance_sheets.get_mut(&old_creditor).and_then(|bs| bs.assets.remove(id)) {
            self.balance_sheets.get_mut(new_creditor).and_then(|bs| bs.assets.insert(*id, asset));
        }

        Ok(())
    }
}

impl FinancialStatistics for FinancialSystem {
    fn m0(&self) -> f64 {
        self.balance_sheets
            .get(&self.central_bank.id)
            .map(|cb_bs| cb_bs.liabilities.values().map(|inst| inst.principal).sum())
            .unwrap_or(0.0)
    }

    fn m1(&self, agents: &AgentRegistry) -> f64 {
        let bank_ids: HashSet<AgentId> = agents.banks.keys().cloned().collect();

        self.balance_sheets
            .values()
            .filter(|bs| !bank_ids.contains(&bs.agent_id) && bs.agent_id != self.central_bank.id)
            .map(|bs| {
                bs.assets
                    .values()
                    .filter(|inst| {
                        inst.details.as_any().is::<CashDetails>() || inst.details.as_any().is::<DemandDepositDetails>()
                    })
                    .map(|inst| inst.principal)
                    .sum::<f64>()
            })
            .sum()
    }

    fn m2(&self, agents: &AgentRegistry) -> f64 {
        let m1 = self.m1(agents);

        let bank_ids: HashSet<AgentId> = agents.banks.keys().cloned().collect();

        let savings_deposits: f64 = self
            .balance_sheets
            .values()
            .filter(|bs| !bank_ids.contains(&bs.agent_id) && bs.agent_id != self.central_bank.id)
            .map(|bs| {
                bs.assets
                    .values()
                    .filter(|inst| inst.details.as_any().is::<SavingsDepositDetails>())
                    .map(|inst| inst.principal)
                    .sum::<f64>()
            })
            .sum();

        m1 + savings_deposits
    }
}
```

---

## `crates/core/sim_types/src/time.rs`

```rust
use chrono::{NaiveDate, Datelike, Duration};
use serde::{Deserialize, Serialize};

pub fn year_fraction(start: NaiveDate, end: NaiveDate) -> f64 {
    (end - start).num_days() as f64 / 365.0
}

pub fn year_fraction_360(start: NaiveDate, end: NaiveDate) -> f64 {
    (end - start).num_days() as f64 / 360.0
}

pub fn is_weekend(date: NaiveDate) -> bool {
    matches!(date.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun)
}

pub fn next_business_day(date: NaiveDate) -> NaiveDate {
    let mut next = date + Duration::days(1);
    while is_weekend(next) {
        next = next + Duration::days(1);
    }
    next
}

pub fn previous_business_day(date: NaiveDate) -> NaiveDate {
    let mut prev = date - Duration::days(1);
    while is_weekend(prev) {
        prev = prev - Duration::days(1);
    }
    prev
}

pub fn add_business_days(date: NaiveDate, days: i32) -> NaiveDate {
    let mut current = date;
    let mut remaining = days.abs();
    let step = if days >= 0 { 1 } else { -1 };
    
    while remaining > 0 {
        current = current + Duration::days(step as i64);
        if !is_weekend(current) {
            remaining -= 1;
        }
    }
    current
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TimePeriod {
    Days(u32),
    Weeks(u32),
    Months(u32),
    Years(u32),
    Overnight,
    Weekly,
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
}

impl TimePeriod {
    pub fn to_days(&self) -> u32 {
        match self {
            TimePeriod::Days(d) => *d,
            TimePeriod::Weeks(w) => w * 7,
            TimePeriod::Months(m) => m * 30,
            TimePeriod::Years(y) => y * 365,
            TimePeriod::Overnight => 1,
            TimePeriod::Weekly => 7,
            TimePeriod::Monthly => 30,
            TimePeriod::Quarterly => 90,
            TimePeriod::SemiAnnual => 183,
            TimePeriod::Annual => 365,
        }
    }
    
    pub fn add_to_date(&self, date: NaiveDate) -> NaiveDate {
        match self {
            TimePeriod::Days(d) => date + Duration::days(*d as i64),
            TimePeriod::Weeks(w) => date + Duration::weeks(*w as i64),
            TimePeriod::Months(m) => {
                let mut result = date;
                for _ in 0..*m {
                    result = add_months(result, 1);
                }
                result
            },
            TimePeriod::Years(y) => {
                date.with_year(date.year() + *y as i32)
                    .unwrap_or(date)
            },
            TimePeriod::Overnight => date + Duration::days(1),
            TimePeriod::Weekly => date + Duration::weeks(1),
            TimePeriod::Monthly => add_months(date, 1),
            TimePeriod::Quarterly => add_months(date, 3),
            TimePeriod::SemiAnnual => add_months(date, 6),
            TimePeriod::Annual => date.with_year(date.year() + 1).unwrap_or(date),
        }
    }
}

pub fn add_months(date: NaiveDate, months: u32) -> NaiveDate {
    let mut year = date.year();
    let mut month = date.month() as i32 + months as i32;
    
    while month > 12 {
        year += 1;
        month -= 12;
    }
    
    let day = date.day();
    
    let max_day = days_in_month(year, month as u32);
    let adjusted_day = day.min(max_day);
    
    NaiveDate::from_ymd_opt(year, month as u32, adjusted_day)
        .unwrap_or(date)
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) { 29 } else { 28 }
        },
        _ => 30,
    }
}

pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BusinessDayConvention {
    None,
    Following,
    Preceding,
    ModifiedFollowing,
    ModifiedPreceding,
}

impl BusinessDayConvention {
    pub fn adjust(&self, date: NaiveDate) -> NaiveDate {
        match self {
            BusinessDayConvention::None => date,
            BusinessDayConvention::Following => {
                if is_weekend(date) {
                    next_business_day(date)
                } else {
                    date
                }
            },
            BusinessDayConvention::Preceding => {
                if is_weekend(date) {
                    previous_business_day(date)
                } else {
                    date
                }
            },
            BusinessDayConvention::ModifiedFollowing => {
                if is_weekend(date) {
                    let next = next_business_day(date);
                    if next.month() != date.month() {
                        previous_business_day(date)
                    } else {
                        next
                    }
                } else {
                    date
                }
            },
            BusinessDayConvention::ModifiedPreceding => {
                if is_weekend(date) {
                    let prev = previous_business_day(date);
                    if prev.month() != date.month() {
                        next_business_day(date)
                    } else {
                        prev
                    }
                } else {
                    date
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    
    #[test]
    fn test_year_fraction() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        
        let frac = year_fraction(start, end);
        assert!((frac - 1.0).abs() < 0.01);
    }
    
    #[test]
    fn test_business_days() {
        let friday = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap();
        let next = next_business_day(friday);
        let monday = NaiveDate::from_ymd_opt(2024, 1, 8).unwrap();
        
        assert_eq!(next, monday);
    }
    
    #[test]
    fn test_add_months() {
        let jan_31 = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let result = add_months(jan_31, 1);
        let feb_29 = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        
        assert_eq!(result, feb_29);
    }
}
```

---

## `crates/domains/banking/Cargo.toml`

```toml
[package]
name = "domains_banking"
version = "0.1.0"
edition = "2024"

[dependencies]
sim_prelude = { path = "../../core/sim_prelude" }
serde = { workspace = true }
uuid = { workspace = true }
rand = { workspace = true }
```

---

## `crates/domains/banking/src/behavior.rs`

```rust
use rand::RngCore;
use sim_prelude::*;
use serde::{Deserialize, Serialize};

/// Extended Bank with decision-making capabilities
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankingAgent {
    pub bank: Bank,
    pub decision_model: Box<dyn BankDecisionModel>,
}

impl BankingAgent {
    pub fn new(bank: Bank) -> Self {
        Self {
            bank,
            decision_model: Box::new(BasicBankDecisionModel),
        }
    }

    pub fn with_decision_model(mut self, model: Box<dyn BankDecisionModel>) -> Self {
        self.decision_model = model;
        self
    }

    pub fn decide(&self, fs: &FinancialSystem, rng: &mut dyn RngCore) -> Vec<BankDecision> {
        self.decision_model.decide(&self.bank, fs, rng)
    }

    pub fn act(&self, decisions: &[BankDecision]) -> Vec<SimAction> {
        let mut actions = Vec::new();

        for decision in decisions {
            match decision {
                BankDecision::BorrowOvernight { amount_dollars, max_annual_rate_bps } => {
                    // TODO: Convert to market action
                    println!("Bank {} wants to borrow ${} at max rate {}", 
                        self.bank.id.0, amount_dollars, max_annual_rate_bps);
                }
                BankDecision::LendOvernight { amount_dollars, min_annual_rate_bps } => {
                    // TODO: Convert to market action
                    println!("Bank {} wants to lend ${} at min rate {}", 
                        self.bank.id.0, amount_dollars, min_annual_rate_bps);
                }
                BankDecision::SetDepositRate { rate } => {
                    println!("Bank {} setting deposit rate to {}", self.bank.id.0, rate);
                }
                BankDecision::SetLendingRate { rate } => {
                    println!("Bank {} setting lending rate to {}", self.bank.id.0, rate);
                }
                BankDecision::ManageReserves { target_level } => {
                    // Calculate needed reserve adjustment
                    let current_reserves = self.get_current_reserves(); // TODO: implement
                    let adjustment = target_level - current_reserves;
                    
                    if adjustment.abs() > 100.0 { // Only act if significant difference
                        actions.push(SimAction::Banking(BankingAction::UpdateReserves {
                            bank: self.bank.id,
                            amount_change: adjustment,
                        }));
                    }
                }
            }
        }

        actions
    }

    fn get_current_reserves(&self) -> f64 {
        // TODO: Calculate current reserves from financial system
        10000.0 // Placeholder
    }
}

```

---

## `crates/domains/banking/src/domain.rs`

```rust
use sim_prelude::*;
use crate::{BankingValidator, BankingOperations};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankingDomain {
    validator: BankingValidator,
    operations: BankingOperations,
}

impl BankingDomain {
    pub fn new() -> Self {
        Self {
            validator: BankingValidator::new(),
            operations: BankingOperations::new(),
        }
    }

    pub fn can_handle(&self, action: &BankingAction) -> bool {
        match action {
            BankingAction::Deposit { .. } => true,
            BankingAction::Withdraw { .. } => true,
            BankingAction::Transfer { .. } => true,
            BankingAction::PayWages { .. } => true,
            BankingAction::UpdateReserves { .. } => true,
            BankingAction::InjectLiquidity => true,
        }
    }

    pub fn validate(&self, action: &BankingAction, state: &SimState) -> Result<(), String> {
        self.validator.validate(action, state)
    }

    pub fn execute(&self, action: &BankingAction, state: &SimState) -> BankingResult {
        // First validate the action
        if let Err(error) = self.validate(action, state) {
            return BankingResult {
                success: false,
                effects: vec![],
                errors: vec![error],
            };
        }

        // Then execute the operation
        match action {
            BankingAction::Deposit { agent_id, bank, amount } => {
                self.operations.execute_deposit(*agent_id, *bank, *amount, state)
            }
            BankingAction::Withdraw { agent_id, bank, amount } => {
                self.operations.execute_withdraw(*agent_id, *bank, *amount, state)
            }
            BankingAction::Transfer { from, to, amount } => {
                self.operations.execute_transfer(*from, *to, *amount, state)
            }
            BankingAction::PayWages { agent_id, employee, amount } => {
                self.operations.execute_transfer(*agent_id, *employee, *amount, state)
            }
            BankingAction::UpdateReserves { bank, amount_change } => {
                self.operations.execute_update_reserves(*bank, *amount_change, state)
            }
            BankingAction::InjectLiquidity => {
                self.operations.execute_inject_liquidity(state)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct BankingResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl Default for BankingDomain {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## `crates/domains/banking/src/lib.rs`

```rust
pub mod domain;
pub mod operations;
pub mod validation;
pub mod behavior;

pub use domain::*;
pub use operations::*;
pub use validation::*;
pub use behavior::*;


```

---

## `crates/domains/banking/src/operations.rs`

```rust
use sim_prelude::*;
use crate::BankingResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankingOperations;

impl BankingOperations {
    pub fn new() -> Self {
        Self
    }

    pub fn execute_deposit(&self, depositor: AgentId, bank: AgentId, amount: f64, state: &SimState) -> BankingResult {
        let mut effects = vec![];

        let deposit_rate = state.financial_system.central_bank.policy_rate - 0.02;
        let deposit = deposit!(depositor, bank, amount, deposit_rate, state.current_date);
        effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(deposit)));

        let transfer_effects = self.create_transfer_effects(depositor, bank, amount, state);
        effects.extend(transfer_effects);

        BankingResult { success: !effects.is_empty(), effects, errors: vec![] }
    }

    pub fn execute_withdraw(&self, account_holder: AgentId, bank: AgentId, amount: f64, state: &SimState) -> BankingResult {
        let mut effects = vec![];

        if let Some((deposit_id, deposit)) = state.financial_system.get_bs_by_id(&account_holder)
            .and_then(|bs| bs.assets.iter().find(|(_, inst)| inst.debtor == bank && inst.details.as_any().is::<DemandDepositDetails>()))
        {
            let new_principal = deposit.principal - amount;
            if new_principal < 1e-6 {
                effects.push(StateEffect::Financial(FinancialEffect::RemoveInstrument(*deposit_id)));
            } else {
                effects.push(StateEffect::Financial(FinancialEffect::UpdateInstrument { id: *deposit_id, new_principal }));
            }

            let transfer_effects = self.create_transfer_effects(bank, account_holder, amount, state);
            effects.extend(transfer_effects);
        }

        BankingResult { success: !effects.is_empty(), effects, errors: vec![] }
    }

    pub fn execute_transfer(&self, from: AgentId, to: AgentId, amount: f64, state: &SimState) -> BankingResult {
        let effects = self.create_transfer_effects(from, to, amount, state);
        BankingResult { success: !effects.is_empty(), effects, errors: vec![] }
    }

    pub fn execute_update_reserves(&self, _bank: AgentId, _amount_change: f64, _state: &SimState) -> BankingResult {
        BankingResult {
            success: false,
            effects: vec![],
            errors: vec!["Reserve update not yet implemented".to_string()],
        }
    }

    pub fn execute_inject_liquidity(&self, state: &SimState) -> BankingResult {
        let effects: Vec<StateEffect> = state.agents.consumers.iter().map(|consumer| {
            let cash = cash!(consumer.id, 1000.0, state.financial_system.central_bank.id, state.current_date);
            StateEffect::Financial(FinancialEffect::CreateInstrument(cash))
        }).collect();

        BankingResult { success: true, effects, errors: vec![] }
    }

    fn create_transfer_effects(&self, from: AgentId, to: AgentId, amount: f64, state: &SimState) -> Vec<StateEffect> {
        let mut effects = vec![];
        let cb_id = state.financial_system.central_bank.id;

        if let Some((inst_id, inst)) = state.financial_system.get_bs_by_id(&from)
            .and_then(|bs| bs.assets.iter().find(|(_, i)| i.principal >= amount && (i.details.as_any().is::<CashDetails>() || i.details.as_any().is::<CentralBankReservesDetails>())))
        {
            effects.push(StateEffect::Financial(FinancialEffect::UpdateInstrument { id: *inst_id, new_principal: inst.principal - amount }));

            let new_inst = if inst.details.as_any().is::<CashDetails>() {
                cash!(to, amount, cb_id, state.current_date)
            } else {
                reserves!(to, cb_id, amount, state.current_date)
            };
            effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(new_inst)));
        }
        effects
    }
}
```

---

## `crates/domains/banking/src/validation.rs`

```rust
use serde::{Deserialize, Serialize};
use sim_prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankingValidator;

impl BankingValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, action: &BankingAction, state: &SimState) -> Result<(), String> {
        match action {
            BankingAction::Deposit { agent_id, bank, amount } => {
                self.validate_deposit(*agent_id, *bank, *amount, state)
            }
            BankingAction::Withdraw { agent_id, bank, amount } => {
                self.validate_withdraw(*agent_id, *bank, *amount, state)
            }
            BankingAction::Transfer { from, to, amount } => {
                self.validate_transfer(*from, *to, *amount, state)
            }
            BankingAction::PayWages { agent_id, employee, amount } => {
                self.validate_transfer(*agent_id, *employee, *amount, state)
            }
            BankingAction::UpdateReserves { bank, .. } => self.validate_bank_exists(*bank, state),
            BankingAction::InjectLiquidity => Ok(()),
        }
    }

    fn validate_deposit(&self, depositor: AgentId, bank: AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        Validator::positive_amount(amount)?;
        self.validate_agent_exists(depositor, state)?;
        self.validate_bank_exists(bank, state)?;
        self.validate_sufficient_cash(depositor, amount, state)
    }

    fn validate_withdraw(&self, account_holder: AgentId, bank: AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        Validator::positive_amount(amount)?;
        self.validate_agent_exists(account_holder, state)?;
        self.validate_bank_exists(bank, state)?;
        self.validate_sufficient_deposits(account_holder, bank, amount, state)?;
        self.validate_bank_liquidity(bank, amount, state)
    }

    fn validate_transfer(&self, from: AgentId, to: AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        Validator::positive_amount(amount)?;
        self.validate_agent_exists(from, state)?;
        self.validate_agent_exists(to, state)?;
        self.validate_sufficient_cash(from, amount, state)
    }

    fn validate_agent_exists(&self, agent_id: AgentId, state: &SimState) -> Result<(), String> {
        if state.financial_system.balance_sheets.contains_key(&agent_id) {
            Ok(())
        } else {
            Err(format!("Agent {} does not exist", agent_id.0))
        }
    }

    fn validate_bank_exists(&self, bank_id: AgentId, state: &SimState) -> Result<(), String> {
        if state.agents.banks.contains_key(&bank_id) {
            Ok(())
        } else {
            Err("Target is not a valid commercial bank".to_string())
        }
    }

    fn validate_sufficient_cash(&self, agent_id: AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        let cash = state.financial_system.get_cash_assets(&agent_id);
        if cash >= amount {
            Ok(())
        } else {
            Err(format!("Insufficient cash for agent {}: have ${:.2}, need ${:.2}", agent_id, cash, amount))
        }
    }

    fn validate_sufficient_deposits(&self, account_holder: AgentId, bank: AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        let deposits = state.financial_system.get_deposits_at_bank(&account_holder, &bank);
        if deposits >= amount {
            Ok(())
        } else {
            Err(format!("Insufficient deposits for agent {}: have ${:.2}, need ${:.2}", account_holder, deposits, amount))
        }
    }

    fn validate_bank_liquidity(&self, bank: AgentId, amount: f64, state: &SimState) -> Result<(), String> {
        let liquidity = state.financial_system.get_liquid_assets(&bank);
        if liquidity >= amount {
            Ok(())
        } else {
            Err(format!("Insufficient bank liquidity for {}: have ${:.2}, need ${:.2}", bank, liquidity, amount))
        }
    }
}
```

---

## `crates/domains/consumption/Cargo.toml`

```toml
[package]
name = "domains_consumption"
version = "0.1.0"
edition = "2024"

[dependencies]
sim_prelude = { path = "../../core/sim_prelude" }
serde = { workspace = true }
uuid = { workspace = true }
rand = { workspace = true }
ndarray = { workspace = true }
```

---

## `crates/domains/consumption/src/behavior.rs`

```rust
//! uconsumption behavior models

// TODO: Implement consumption behavior models


```

---

## `crates/domains/consumption/src/domain.rs`

```rust
use sim_prelude::*;
use crate::{ConsumptionOperations, ConsumptionValidator};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumptionDomain {
    validator: ConsumptionValidator,
    operations: ConsumptionOperations,
}

#[derive(Debug, Clone)]
pub struct ConsumptionResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl ConsumptionDomain {
    pub fn new() -> Self {
        Self {
            validator: ConsumptionValidator::new(),
            operations: ConsumptionOperations::new(),
        }
    }

    pub fn can_handle(&self, action: &ConsumptionAction) -> bool {
        matches!(action, ConsumptionAction::Purchase { .. } | ConsumptionAction::Consume { .. })
    }

    pub fn validate(&self, action: &ConsumptionAction, state: &SimState) -> Result<(), String> {
        self.validator.validate(action, state)
    }

    pub fn execute(&self, action: &ConsumptionAction, state: &SimState) -> ConsumptionResult {
        if let Err(error) = self.validate(action, state) {
            return ConsumptionResult { success: false, effects: vec![], errors: vec![error] };
        }

        match action {
            ConsumptionAction::Purchase { agent_id, seller, good_id, amount } => {
                self.operations.execute_purchase(*agent_id, *seller, *good_id, *amount, state)
            }
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                self.operations.execute_consume(*agent_id, *good_id, *amount)
            }
        }
    }
}

impl Default for ConsumptionDomain {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## `crates/domains/consumption/src/lib.rs`

```rust
//! consumption domain implementation

pub mod domain;
pub mod operations;
pub mod validation;
pub mod behavior;

pub use domain::*;
pub use operations::*;
pub use validation::*;


```

---

## `crates/domains/consumption/src/operations.rs`

```rust
use sim_prelude::*;
use crate::ConsumptionResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumptionOperations;

impl ConsumptionOperations {
    pub fn new() -> Self {
        Self
    }

    pub fn execute_purchase(
        &self,
        buyer: AgentId,
        seller: AgentId,
        good_id: GoodId,
        amount: f64,
        state: &SimState,
    ) -> ConsumptionResult {
        let mut effects = vec![];

        // Determine price from the best ask on the market
        let price = state
            .financial_system
            .exchange
            .goods_market(&good_id)
            .and_then(|m| m.best_ask())
            .map_or(1.0, |ask| ask.price); // Default price if market is empty

        let total_cost = amount * price;

        // 1. Transfer funds from buyer to seller
        // This is a meta-operation; we create the primitive effects directly.
        if let Some((cash_id, cash)) = state.financial_system.get_bs_by_id(&buyer)
            .and_then(|bs| bs.assets.iter().find(|(_, inst)| inst.details.as_any().is::<CashDetails>()))
        {
            effects.push(StateEffect::Financial(FinancialEffect::UpdateInstrument {
                id: *cash_id,
                new_principal: cash.principal - total_cost,
            }));
            let seller_cash = cash!(seller, total_cost, state.financial_system.central_bank.id, state.current_date);
            effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(seller_cash)));
        } else {
            return ConsumptionResult { success: false, effects: vec![], errors: vec!["Buyer has no cash instrument".to_string()] };
        }

        // 2. Transfer inventory from seller to buyer
        effects.push(StateEffect::Inventory(InventoryEffect::RemoveInventory {
            owner: seller,
            good_id,
            quantity: amount,
        }));
        effects.push(StateEffect::Inventory(InventoryEffect::AddInventory {
            owner: buyer,
            good_id,
            quantity: amount,
            unit_cost: price,
        }));

        ConsumptionResult { success: true, effects, errors: vec![] }
    }

    pub fn execute_consume(&self, agent_id: AgentId, good_id: GoodId, amount: f64) -> ConsumptionResult {
        let effects = vec![StateEffect::Inventory(InventoryEffect::RemoveInventory {
            owner: agent_id,
            good_id,
            quantity: amount,
        })];

        ConsumptionResult { success: true, effects, errors: vec![] }
    }
}
```

---

## `crates/domains/consumption/src/validation.rs`

```rust
use sim_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumptionValidator;

impl ConsumptionValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, action: &ConsumptionAction, state: &SimState) -> Result<(), String> {
        match action {
            ConsumptionAction::Purchase { agent_id, seller, good_id, amount } => {
                self.validate_purchase(*agent_id, *seller, *good_id, *amount, state)
            }
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                self.validate_consume(*agent_id, *good_id, *amount, state)
            }
        }
    }

    fn validate_purchase(
        &self,
        buyer: AgentId,
        seller: AgentId,
        good_id: GoodId,
        amount: f64,
        state: &SimState,
    ) -> Result<(), String> {
        Validator::positive_amount(amount)?;

        // Check agents exist
        if !state.financial_system.balance_sheets.contains_key(&buyer) {
            return Err(format!("Buyer {:?} not found", buyer));
        }
        if !state.financial_system.balance_sheets.contains_key(&seller) {
            return Err(format!("Seller {:?} not found", seller));
        }

        // Check seller has inventory
        let seller_bs = state.financial_system.balance_sheets.get(&seller).unwrap();
        let available_inventory = seller_bs.get_inventory().and_then(|inv| inv.get(&good_id)).map_or(0.0, |item| item.quantity);
        if available_inventory < amount {
            return Err(format!("Seller has insufficient inventory: needs {:.2}, has {:.2}", amount, available_inventory));
        }

        // Check buyer has funds
        let price = state.financial_system.exchange.goods_market(&good_id)
            .and_then(|m| m.best_ask()).map_or(1.0, |ask| ask.price);
        let total_cost = amount * price;
        let available_funds = state.financial_system.get_liquid_assets(&buyer);
        if available_funds < total_cost {
            return Err(format!("Buyer has insufficient funds: needs ${:.2}, has ${:.2}", total_cost, available_funds));
        }

        Ok(())
    }

    fn validate_consume(&self, agent_id: AgentId, good_id: GoodId, amount: f64, state: &SimState) -> Result<(), String> {
        Validator::positive_amount(amount)?;

        let bs = state.financial_system.balance_sheets.get(&agent_id).ok_or(format!("Agent {:?} not found", agent_id))?;
        let available = bs.get_inventory().and_then(|inv| inv.get(&good_id)).map_or(0.0, |item| item.quantity);

        if available < amount {
            return Err(format!("Agent has insufficient goods to consume: needs {:.2}, has {:.2}", amount, available));
        }

        Ok(())
    }
}
```

---

## `crates/domains/prelude/Cargo.toml`

```toml
[package]
name = "domains_prelude"
version = "0.1.0"
edition = "2024"

[dependencies]
domains_banking = { path = "../banking" }
domains_production = { path = "../production" }
domains_trading = { path = "../trading" }
domains_consumption = { path = "../consumption" }
```

---

## `crates/domains/prelude/src/lib.rs`

```rust
pub use domains_banking::{
    BankingDomain, 
    BankingOperations, 
    BankingValidator, 
    BankingResult,
    BankingAgent, 
};

pub use domains_consumption::{
    ConsumptionDomain, 
    ConsumptionOperations, 
    ConsumptionValidator, 
    ConsumptionResult,
};

pub use domains_production::{
    ProductionDomain, 
    ProductionOperations, 
    ProductionValidator, 
    ProductionResult,
};

pub use domains_trading::{
    TradingDomain, 
    TradingOperations, 
    TradingValidator, 
    TradingResult,
};
```

---

## `crates/domains/production/Cargo.toml`

```toml
[package]
name = "domains_production"
version = "0.1.0"
edition = "2024"

[dependencies]
sim_prelude = { path = "../../core/sim_prelude" }
serde = { workspace = true }
uuid = { workspace = true }
rand = { workspace = true }
```

---

## `crates/domains/production/src/behavior.rs`

```rust
//! uproduction behavior models

// TODO: Implement production behavior models


```

---

## `crates/domains/production/src/domain.rs`

```rust
use sim_prelude::*;
use crate::{ProductionOperations, ProductionValidator};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionDomain {
    validator: ProductionValidator,
    operations: ProductionOperations,
}

#[derive(Debug, Clone)]
pub struct ProductionResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl ProductionDomain {
    pub fn new() -> Self {
        Self {
            validator: ProductionValidator::new(),
            operations: ProductionOperations::new(),
        }
    }

    pub fn can_handle(&self, action: &ProductionAction) -> bool {
        matches!(action, ProductionAction::Hire { .. } | ProductionAction::Produce { .. } | ProductionAction::PayWages { .. })
    }

    pub fn validate(&self, action: &ProductionAction, state: &SimState) -> Result<(), String> {
        self.validator.validate(action, state)
    }

    pub fn execute(&self, action: &ProductionAction, state: &SimState) -> ProductionResult {
        if let Err(error) = self.validate(action, state) {
            return ProductionResult { success: false, effects: vec![], errors: vec![error] };
        }

        match action {
            ProductionAction::Hire { agent_id, count } => self.operations.execute_hire(*agent_id, *count),
            ProductionAction::Produce { agent_id, recipe_id, batches } => {
                self.operations.execute_produce(*agent_id, *recipe_id, *batches, state)
            }
            ProductionAction::PayWages { .. } => {
                // Wage payments are financial transfers, handled by the BankingDomain.
                // This action is converted into a BankingAction by the engine.
                // Therefore, this domain produces no effects for it directly.
                ProductionResult { success: true, effects: vec![], errors: vec![] }
            }
        }
    }
}

impl Default for ProductionDomain {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## `crates/domains/production/src/lib.rs`

```rust
//! production domain implementation

pub mod domain;
pub mod operations;
pub mod validation;
pub mod behavior;

pub use domain::*;
pub use operations::*;
pub use validation::*;


```

---

## `crates/domains/production/src/operations.rs`

```rust
use sim_prelude::*;
use crate::ProductionResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionOperations;

impl ProductionOperations {
    pub fn new() -> Self {
        Self
    }

    pub fn execute_hire(&self, firm_id: AgentId, count: u32) -> ProductionResult {
        // In a more complex model, this would find unemployed agents.
        // For now, it's a simple effect that a firm behavior model can react to.
        let effects = vec![StateEffect::Agent(AgentEffect::Hire { firm: firm_id, count })];
        ProductionResult { success: true, effects, errors: vec![] }
    }

    pub fn execute_produce(
        &self,
        firm_id: AgentId,
        recipe_id: RecipeId,
        batches: u32,
        state: &SimState,
    ) -> ProductionResult {
        let recipe = match state.financial_system.goods.recipes.get(&recipe_id) {
            Some(r) => r,
            None => {
                return ProductionResult {
                    success: false,
                    effects: vec![],
                    errors: vec![format!("Recipe {:?} not found", recipe_id)],
                }
            }
        };

        let mut effects = vec![];
        let total_batches = batches as f64;

        // 1. Consume inputs
        for (input_good, required_qty) in &recipe.inputs {
            effects.push(StateEffect::Inventory(InventoryEffect::RemoveInventory {
                owner: firm_id,
                good_id: *input_good,
                quantity: *required_qty * total_batches,
            }));
        }

        // 2. Add output
        let (output_good, output_qty) = &recipe.output;
        let total_output = output_qty * total_batches * recipe.efficiency;
        // A simple cost model: for now, unit cost is 0 as we don't model input costs perfectly yet.
        effects.push(StateEffect::Inventory(InventoryEffect::AddInventory {
            owner: firm_id,
            good_id: *output_good,
            quantity: total_output,
            unit_cost: 0.0,
        }));

        ProductionResult { success: true, effects, errors: vec![] }
    }
}
```

---

## `crates/domains/production/src/validation.rs`

```rust
use sim_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionValidator;

impl ProductionValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, action: &ProductionAction, state: &SimState) -> Result<(), String> {
        match action {
            ProductionAction::Hire { agent_id, count } => self.validate_hire(*agent_id, *count, state),
            ProductionAction::Produce { agent_id, recipe_id, batches } => {
                self.validate_produce(*agent_id, *recipe_id, *batches, state)
            }
            ProductionAction::PayWages { agent_id, amount, .. } => {
                Validator::positive_amount(*amount)?;
                if state.financial_system.get_liquid_assets(agent_id) < *amount {
                    return Err("Insufficient funds for wages".to_string());
                }
                Ok(())
            }
        }
    }

    fn validate_hire(&self, firm_id: AgentId, count: u32, state: &SimState) -> Result<(), String> {
        Validator::positive_integer(count, "hire count")?;
        if !state.agents.firms.iter().any(|f| f.id == firm_id) {
            return Err(format!("Firm {:?} not found", firm_id));
        }
        // Could add a check for liquidity to pay wages
        Ok(())
    }

    fn validate_produce(
        &self,
        firm_id: AgentId,
        recipe_id: RecipeId,
        batches: u32,
        state: &SimState,
    ) -> Result<(), String> {
        Validator::positive_integer(batches, "production batches")?;
        let firm = state.agents.firms.iter().find(|f| f.id == firm_id).ok_or(format!("Firm {:?} not found", firm_id))?;
        let recipe = state.financial_system.goods.recipes.get(&recipe_id).ok_or(format!("Recipe {:?} not found", recipe_id))?;

        let bs = state.financial_system.balance_sheets.get(&firm_id).ok_or("Firm has no balance sheet")?;
        let inventory = bs.get_inventory().ok_or("Firm has no inventory")?;

        for (input_good, required_qty) in &recipe.inputs {
            let available = inventory.get(input_good).map_or(0.0, |item| item.quantity);
            let total_needed = *required_qty * batches as f64;
            if available < total_needed {
                return Err(format!("Insufficient input {:?}: have {:.2}, need {:.2}", input_good, available, total_needed));
            }
        }

        if firm.employees.is_empty() {
            return Err("Firm has no employees to produce".to_string());
        }
        Ok(())
    }
}
```

---

## `crates/domains/trading/Cargo.toml`

```toml
[package]
name = "domains_trading"
version = "0.1.0"
edition = "2024"

[dependencies]
sim_prelude = { path = "../../core/sim_prelude" }
serde = { workspace = true }
uuid = { workspace = true }
rand = { workspace = true }
```

---

## `crates/domains/trading/src/behavior.rs`

```rust
//! utrading behavior models

// TODO: Implement trading behavior models


```

---

## `crates/domains/trading/src/domain.rs`

```rust
use sim_prelude::*;
use crate::{TradingOperations, TradingValidator};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradingDomain {
    validator: TradingValidator,
    operations: TradingOperations,
}

#[derive(Debug, Clone)]
pub struct TradingResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl TradingDomain {
    pub fn new() -> Self {
        Self {
            validator: TradingValidator::new(),
            operations: TradingOperations::new(),
        }
    }

    pub fn can_handle(&self, action: &TradingAction) -> bool {
        matches!(action, TradingAction::PostBid { .. } | TradingAction::PostAsk { .. })
    }

    pub fn validate(&self, action: &TradingAction, state: &SimState) -> Result<(), String> {
        self.validator.validate(action, state)
    }

    pub fn execute(&self, action: &TradingAction, state: &SimState) -> TradingResult {
        if let Err(error) = self.validate(action, state) {
            return TradingResult { success: false, effects: vec![], errors: vec![error] };
        }

        match action {
            TradingAction::PostBid { agent_id, market_id, quantity, price } => {
                self.operations.execute_post_bid(*agent_id, market_id.clone(), *quantity, *price)
            }
            TradingAction::PostAsk { agent_id, market_id, quantity, price } => {
                self.operations.execute_post_ask(*agent_id, market_id.clone(), *quantity, *price)
            }
        }
    }
}

impl Default for TradingDomain {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## `crates/domains/trading/src/lib.rs`

```rust
//! trading domain implementation

pub mod domain;
pub mod operations;
pub mod validation;
pub mod behavior;

pub use domain::*;
pub use operations::*;
pub use validation::*;


```

---

## `crates/domains/trading/src/operations.rs`

```rust
use sim_prelude::*;
use crate::TradingResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradingOperations;

impl TradingOperations {
    pub fn new() -> Self {
        Self
    }

    pub fn execute_post_bid(
        &self,
        agent_id: AgentId,
        market_id: MarketId,
        quantity: f64,
        price: f64,
    ) -> TradingResult {
        let effects = vec![StateEffect::Market(MarketEffect::PlaceOrderInBook {
            market_id,
            order: Order::Bid(Bid { agent_id, quantity, price }),
        })];

        TradingResult { success: true, effects, errors: vec![] }
    }

    pub fn execute_post_ask(
        &self,
        agent_id: AgentId,
        market_id: MarketId,
        quantity: f64,
        price: f64,
    ) -> TradingResult {
        let effects = vec![StateEffect::Market(MarketEffect::PlaceOrderInBook {
            market_id,
            order: Order::Ask(Ask { agent_id, quantity, price }),
        })];

        TradingResult { success: true, effects, errors: vec![] }
    }
}
```

---

## `crates/domains/trading/src/validation.rs`

```rust
use sim_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradingValidator;

impl TradingValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, action: &TradingAction, state: &SimState) -> Result<(), String> {
        match action {
            TradingAction::PostBid { agent_id, quantity, price, .. } => {
                self.validate_post_bid(*agent_id, *quantity, *price, state)
            }
            TradingAction::PostAsk { agent_id, market_id, quantity, .. } => {
                self.validate_post_ask(*agent_id, market_id, *quantity, state)
            }
        }
    }

    fn validate_post_bid(&self, agent_id: AgentId, quantity: f64, price: f64, state: &SimState) -> Result<(), String> {
        Validator::positive_amount(quantity)?;
        Validator::positive_amount(price)?;

        if !state.financial_system.balance_sheets.contains_key(&agent_id) {
            return Err(format!("Bidding agent {:?} not found", agent_id));
        }

        let required_cash = quantity * price;
        let available_cash = state.financial_system.get_liquid_assets(&agent_id);

        if available_cash < required_cash {
            return Err(format!(
                "Insufficient funds for bid: agent {:?} needs ${:.2}, has ${:.2}",
                agent_id, required_cash, available_cash
            ));
        }

        Ok(())
    }

    fn validate_post_ask(
        &self,
        agent_id: AgentId,
        market_id: &MarketId,
        quantity: f64,
        state: &SimState,
    ) -> Result<(), String> {
        Validator::positive_amount(quantity)?;

        if !state.financial_system.balance_sheets.contains_key(&agent_id) {
            return Err(format!("Asking agent {:?} not found", agent_id));
        }

        if let MarketId::Goods(good_id) = market_id {
            let bs = state.financial_system.balance_sheets.get(&agent_id).unwrap();
            let available_inventory = bs.get_inventory().and_then(|inv| inv.get(good_id)).map_or(0.0, |item| item.quantity);

            if available_inventory < quantity {
                return Err(format!(
                    "Insufficient inventory for ask: agent {:?} needs {:.2}, has {:.2}",
                    agent_id, quantity, available_inventory
                ));
            }
        }
        // Note: Validation for financial instruments would go here

        Ok(())
    }
}
```

---

## `crates/engine/Cargo.toml`

```toml
[package]
name = "engine"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "cli"
path = "cli/main.rs"

[dependencies]
# Core simulation crates
sim_prelude = { path = "../core/sim_prelude" }
# Domain crates
domains_prelude = { path = "../domains/prelude" }
# Workspace dependencies
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
axum = { workspace = true }
tower-http = { workspace = true} 
chrono = { workspace = true }
uuid = { workspace = true }
rand = { workspace = true }
toml = { workspace = true }
thiserror = { workspace = true }
crossbeam-channel = { workspace = true }
fake = { workspace = true }
typetag = { workspace = true }
serde_with = { workspace = true }
rand_chacha = "0.9.0"
rand_core = "0.9.3"
async-nats = { version = "0.42.0", features = ["websockets"] }
futures = "0.3.31"

```

---

## `crates/engine/cli/bridge.rs`

```rust
use crate::{routes, AppState};
use async_nats::connect;
use futures::stream::StreamExt;
use std::sync::Arc;

pub async fn run_nats_bridge(state: Arc<AppState>) -> Result<(), Box<dyn std::error::Error>> {
    const NATS_URL: &str = "ws://127.0.0.1:8070";

    let client = connect(NATS_URL).await?;
    println!("[NATS] Connected successfully!");

    let mut subscriber = client.subscribe("sim.control.>").await?;
    println!("\n[NATS] Subscribed to 'sim.control.>'");
    println!();
    println!("[NATS] Available commands:\n");
    println!(" > nats req sim.control.init - [Initialize simulation]");
    println!(" > nats req sim.control.tick - [Advance simulation by one tick]");
    println!(" > nats req sim.control.query.state - [Request current simulation state]");
    println!();

    while let Some(msg) = subscriber.next().await {
        let client_clone = client.clone();
        let state_clone = state.clone(); // Clone the Arc, not the state itself
        tokio::spawn(async move {
            routes::handle_message(msg, client_clone, state_clone).await;
        });
    }

    Ok(())
}
```

---

## `crates/engine/cli/main.rs`

```rust
use crate::bridge::run_nats_bridge;
use engine::{SimulationEngine, Scenario};
use std::sync::{Arc, Mutex};

mod bridge;
mod routes;

pub const SCENARIO_TOML: &str = include_str!("../../../config/config.toml");

pub struct AppState {
    sim_engine: Mutex<Option<SimulationEngine>>,
    scenario: Scenario,
}

#[tokio::main]
async fn main() {
    let scenario = Scenario::from_toml_str(SCENARIO_TOML)
        .expect("Failed to parse scenario TOML");

    let app_state = Arc::new(AppState {
        sim_engine: Mutex::new(None),
        scenario,
    });
    
    run_nats_bridge(app_state)
        .await
        .expect("[NATS] Failed to run NATS bridge");
}
```

---

## `crates/engine/cli/routes.rs`

```rust
use crate::AppState;
use async_nats::{Client, Message};
use engine::*;
use rand::rngs::ThreadRng;
use serde_json::json;
use std::sync::Arc;

pub async fn handle_message(msg: Message, client: Client, state: Arc<AppState>) {
    println!("[NATS] Received message on '{}'", msg.subject);

    let response = match msg.subject.as_ref() {
        "sim.control.init" => handle_init_sim(&state),
        "sim.control.tick" => handle_tick(&state),
        "sim.control.query.state" => handle_req_state(&state),
        _ => {
            let error_msg = format!("[NATS] No handler for subject: {}", msg.subject);
            println!("{}", error_msg);
            Err(error_msg)
        }
    };

    if let Some(reply) = msg.reply {
        let payload = match response {
            Ok(data) => data,
            Err(e) => json!({ "status": "error", "message": e }).to_string(),
        };
        client.publish(reply, payload.into()).await.ok();
    }
}

fn handle_init_sim(state: &Arc<AppState>) -> Result<String, String> {
    println!("[SIMCTL] Received INIT command.");
    let mut engine_guard = state.sim_engine.lock().unwrap();
    
    let sim_state = state.scenario.initialize_state();
    *engine_guard = Some(SimulationEngine::new(sim_state));

    println!("[SIMCTL] Simulation Initialized.");
    Ok(json!({ "status": "ok", "message": "Simulation initialized successfully." }).to_string())
}

fn handle_tick(state: &Arc<AppState>) -> Result<String, String> {
    println!("[SIMCTL] Received TICK command.");
    let mut engine_guard = state.sim_engine.lock().unwrap();

    if let Some(engine) = engine_guard.as_mut() {
        let mut rng = ThreadRng::default();
        let result = engine.tick(&mut rng);
        println!("[SIMCTL] Tick {} completed.", result.tick_number);
        Ok(serde_json::to_string(&result).map_err(|e| e.to_string())?)
    } else {
        Err("Simulation not initialized. Send 'init' command first.".to_string())
    }
}

fn handle_req_state(state: &Arc<AppState>) -> Result<String, String> {
    println!("[SIMCTL] Received QUERY STATE command");
    let engine_guard = state.sim_engine.lock().unwrap();

    if let Some(engine) = engine_guard.as_ref() {
        let state_json = serde_json::to_string(&engine.state).map_err(|e| e.to_string())?;
        Ok(state_json)
    } else {
        Err("Simulation not initialized. Send 'init' command first.".to_string())
    }
}
```

---

## `crates/engine/src/api.rs`

```rust
//! HTTP API handlers

// TODO: Implement API handlers


```

---

## `crates/engine/src/executor.rs`

```rust
use serde::{Deserialize, Serialize};
use sim_prelude::*;
use crate::registry::DomainRegistry;
use rand::RngCore;

pub struct SimulationEngine {
    pub state: SimState,
    pub domain_registry: DomainRegistry,
}

impl SimulationEngine {
    pub fn new(state: SimState) -> Self {
        Self { state, domain_registry: DomainRegistry::new() }
    }

    pub fn tick(&mut self, rng: &mut dyn RngCore) -> TickResult {
        let decisions = self.collect_decisions(rng);

        let actions = self.convert_decisions_to_actions(&decisions);

        let effects = self.execute_actions(&actions);
        if let Err(e) = self.state.apply_effects(&effects) {
            println!("[ERROR] applying effects: {}", e);
        }

        let trades = self.state.financial_system.exchange.clear_markets();
        let settlement_effects = self.settle_trades(&trades);
        if let Err(e) = self.state.apply_effects(&settlement_effects) {
            println!("[ERROR] applying settlement effects: {}", e);
        }

        self.state.advance_time();

        TickResult {
            tick_number: self.state.ticknum,
            decisions_count: decisions.len(),
            actions_count: actions.len(),
            effects_count: effects.len() + settlement_effects.len(),
        }
    }

    fn collect_decisions(&self, rng: &mut dyn RngCore) -> Vec<AgentDecision> {
        let mut all_decisions = Vec::new();
        let fs = &self.state.financial_system;

        let bank_model = BasicBankDecisionModel::default();
        for bank in self.state.agents.banks.values() {
            let decisions = bank_model.decide(bank, fs, rng);
            for decision in decisions {
                all_decisions.push(AgentDecision::Bank { agent_id: bank.id, decision });
            }
        }

        let consumer_model = BasicConsumerDecisionModel::default();
        for consumer in &self.state.agents.consumers {
            let decisions = consumer_model.decide(consumer, fs, rng);
            for decision in decisions {
                all_decisions.push(AgentDecision::Consumer { agent_id: consumer.id, decision });
            }
        }

        let firm_model = BasicFirmDecisionModel::default();
        for firm in &self.state.agents.firms {
            let decisions = firm_model.decide(firm, fs, rng);
            for decision in decisions {
                all_decisions.push(AgentDecision::Firm { agent_id: firm.id, decision });
            }
        }

        all_decisions
    }

    fn convert_decisions_to_actions(&self, decisions: &[AgentDecision]) -> Vec<SimAction> {
        let mut actions = Vec::new();

        for agent_decision in decisions {
            match agent_decision {
                AgentDecision::Bank { decision, .. } => match decision {
                    BankDecision::LendOvernight { amount_dollars, min_annual_rate_bps } => {
                        let daily_rate = self.state.financial_system.exchange.financial_markets
                            [&FinancialMarketId::SecuredOvernightFinancing]
                            .market_id
                            .annual_bps_to_daily_rate(*min_annual_rate_bps);
                        let price = 1.0 / (1.0 + daily_rate);
                        actions.push(SimAction::Trading(TradingAction::PostAsk {
                            agent_id: agent_decision.agent_id(),
                            market_id: MarketId::Financial(FinancialMarketId::SecuredOvernightFinancing),
                            quantity: *amount_dollars,
                            price,
                        }));
                    }
                    BankDecision::BorrowOvernight { amount_dollars, max_annual_rate_bps } => {
                        let daily_rate = self.state.financial_system.exchange.financial_markets
                            [&FinancialMarketId::SecuredOvernightFinancing]
                            .market_id
                            .annual_bps_to_daily_rate(*max_annual_rate_bps);
                        let price = 1.0 / (1.0 + daily_rate);
                        actions.push(SimAction::Trading(TradingAction::PostBid {
                            agent_id: agent_decision.agent_id(),
                            market_id: MarketId::Financial(FinancialMarketId::SecuredOvernightFinancing),
                            quantity: *amount_dollars,
                            price,
                        }));
                    }
                    _ => {}
                },
                AgentDecision::Consumer { decision, .. } => match decision {
                    ConsumerDecision::Spend { seller_id, amount, good_id, .. } => {
                        actions.push(SimAction::Consumption(ConsumptionAction::Purchase {
                            agent_id: agent_decision.agent_id(),
                            seller: *seller_id,
                            good_id: *good_id,
                            amount: *amount,
                        }));
                    }
                    ConsumerDecision::Save { agent_id, amount } => {
                        if let Some(consumer) = self.state.agents.consumers.iter().find(|c| c.id == *agent_id) {
                            actions.push(SimAction::Banking(BankingAction::Deposit {
                                agent_id: *agent_id,
                                bank: consumer.bank_id,
                                amount: *amount,
                            }));
                        }
                    }
                    _ => {}
                },
                AgentDecision::Firm { decision, .. } => match decision {
                    FirmDecision::Produce { recipe_id, batches } => {
                        actions.push(SimAction::Production(ProductionAction::Produce {
                            agent_id: agent_decision.agent_id(),
                            recipe_id: *recipe_id,
                            batches: *batches,
                        }));
                    }
                    FirmDecision::Hire { count } => {
                        actions.push(SimAction::Production(ProductionAction::Hire {
                            agent_id: agent_decision.agent_id(),
                            count: *count,
                        }));
                    }
                    FirmDecision::PayWages { employee, amount } => {
                        actions.push(SimAction::Banking(BankingAction::PayWages {
                            agent_id: agent_decision.agent_id(),
                            employee: *employee,
                            amount: *amount,
                        }));
                    }
                    FirmDecision::SellInventory { good_id, quantity } => {
                        let price = 100.0;
                        actions.push(SimAction::Trading(TradingAction::PostAsk {
                            agent_id: agent_decision.agent_id(),
                            market_id: MarketId::Goods(*good_id),
                            quantity: *quantity,
                            price,
                        }));
                    }
                    _ => {}
                },
            }
        }
        actions
    }

    fn execute_actions(&self, actions: &[SimAction]) -> Vec<StateEffect> {
        let mut all_effects = Vec::new();
        for action in actions {
            let effects = self.domain_registry.execute(action, &self.state);
            all_effects.extend(effects);
        }
        all_effects
    }

    fn settle_trades(&self, trades: &[Trade]) -> Vec<StateEffect> {
        let mut effects = Vec::new();
        for trade in trades {
            effects.push(StateEffect::Market(MarketEffect::ExecuteTrade(trade.clone())));
        }
        effects
    }
}

#[derive(Debug, Clone)]
pub enum AgentDecision {
    Bank { agent_id: AgentId, decision: BankDecision },
    Consumer { agent_id: AgentId, decision: ConsumerDecision },
    Firm { agent_id: AgentId, decision: FirmDecision },
}

impl AgentDecision {
    pub fn agent_id(&self) -> AgentId {
        match self {
            AgentDecision::Bank { agent_id, .. } => *agent_id,
            AgentDecision::Consumer { agent_id, .. } => *agent_id,
            AgentDecision::Firm { agent_id, .. } => *agent_id,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TickResult {
    pub tick_number: u32,
    pub decisions_count: usize,
    pub actions_count: usize,
    pub effects_count: usize,
}
```

---

## `crates/engine/src/factory.rs`

```rust
use chrono::Datelike;
use rand::prelude::*;
use sim_prelude::*;
use std::str::FromStr;
use crate::scenario::{BankConfig, ConsumerConfig, FirmConfig};

pub struct AgentFactory<'a> {
    pub state: &'a mut SimState,
    pub rng: &'a mut ThreadRng,
}

impl<'a> AgentFactory<'a> {
    pub fn new(state: &'a mut SimState, rng: &'a mut ThreadRng) -> Self {
        Self { state, rng }
    }

    pub fn create_bank(&mut self, config: &BankConfig, cb_id: AgentId) -> Bank {
        let bank = Bank::new(config.name.clone(), 200.0, -70.0);
        self.state.financial_system.balance_sheets.insert(bank.id, BalanceSheet::new(bank.id));

        let reserves = reserves!(bank.id, cb_id, config.initial_reserves, self.state.current_date);
        self.state.financial_system.create_instrument(reserves).unwrap();

        for bond_conf in &config.initial_bonds {
            let tenor = Tenor::from_str(&bond_conf.tenor).unwrap();
            let years_to_maturity = match tenor {
                Tenor::T2Y => 2,
                Tenor::T5Y => 5,
                Tenor::T10Y => 10,
                Tenor::T30Y => 30,
            };
            let maturity_date = self
                .state
                .current_date
                .with_year(self.state.current_date.year() + years_to_maturity)
                .expect("Failed to calculate maturity date");

            let bond = bond!(
                bank.id,
                cb_id,
                bond_conf.face_value,
                0.04,
                maturity_date,
                bond_conf.face_value,
                BondType::Government,
                2,
                self.state.current_date
            );
            self.state.financial_system.create_instrument(bond).unwrap();
        }

        self.state.agents.banks.insert(bank.id, bank.clone());
        bank
    }

    pub fn create_consumer(&mut self, config: &ConsumerConfig, bank_id: AgentId, cb_id: AgentId) -> Consumer {
        let personality = *vec![PersonalityArchetype::Balanced, PersonalityArchetype::Saver, PersonalityArchetype::Spender]
            .choose(self.rng)
            .unwrap();
        let mut consumer = Consumer::new(self.rng.random_range(25..65), bank_id, personality);
        consumer.income = config.income;

        self.state.financial_system.balance_sheets.insert(consumer.id, BalanceSheet::new(consumer.id));
        let cash = cash!(consumer.id, config.initial_cash, cb_id, self.state.current_date);
        self.state.financial_system.create_instrument(cash).unwrap();

        self.state.agents.consumers.push(consumer.clone());
        consumer
    }

    pub fn create_firm(&mut self, config: &FirmConfig, bank_id: AgentId, cb_id: AgentId) -> Firm {
        let recipe_id = self.state.financial_system.goods.get_recipe_id_by_name(&config.recipe_name);
        let firm = Firm::new(bank_id, config.name.clone(), recipe_id);

        self.state.financial_system.balance_sheets.insert(firm.id, BalanceSheet::new(firm.id));
        let cash = cash!(firm.id, config.initial_cash, cb_id, self.state.current_date);
        self.state.financial_system.create_instrument(cash).unwrap();

        let inventory_to_add: Vec<_> = config
            .initial_inventory
            .iter()
            .map(|inv_conf| {
                let good_id = self.state.financial_system.goods.get_good_id_by_slug(&inv_conf.good_slug).unwrap();
                (good_id, inv_conf.quantity, inv_conf.unit_cost)
            })
            .collect();

        let bs = self.state.financial_system.balance_sheets.get_mut(&firm.id).unwrap();
        for (good_id, quantity, unit_cost) in inventory_to_add {
            bs.add_to_inventory(&good_id, quantity, unit_cost);
        }

        self.state.agents.firms.push(firm.clone());
        firm
    }
}
```

---

## `crates/engine/src/lib.rs`

```rust
//! Simulation engine - orchestrates the Decision → Action → Effect pipeline

pub mod executor;
pub mod scheduler;
pub mod registry;
pub mod state_manager;
pub mod scenario;
pub mod api;
pub mod factory;

pub use executor::*;
pub use registry::*;
pub use factory::*;
pub use scenario::*;
```

---

## `crates/engine/src/registry.rs`

```rust
use sim_prelude::*;
use domains_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainRegistry {
    banking: BankingDomain,
    production: ProductionDomain,
    trading: TradingDomain,
    consumption: ConsumptionDomain,
}

impl DomainRegistry {
    pub fn new() -> Self {
        Self {
            banking: BankingDomain::new(),
            production: ProductionDomain::new(),
            trading: TradingDomain::new(),
            consumption: ConsumptionDomain::new(),
        }
    }

    pub fn execute(&self, action: &SimAction, state: &SimState) -> Vec<StateEffect> {
        match action {
            SimAction::Banking(action) => {
                if self.banking.can_handle(action) {
                    let result = self.banking.execute(action, state);
                    if !result.success {
                        println!("Banking action failed: {:?}", result.errors);
                    }
                    result.effects
                } else {
                    println!("Banking domain cannot handle action: {:?}", action);
                    vec![]
                }
            }
            SimAction::Production(action) => {
                if self.production.can_handle(action) {
                    let result = self.production.execute(action, state);
                    if !result.success {
                        println!("Production action failed: {:?}", result.errors);
                    }
                    result.effects
                } else {
                    println!("Production domain cannot handle action: {:?}", action);
                    vec![]
                }
            }
            SimAction::Trading(action) => {
                if self.trading.can_handle(action) {
                    let result = self.trading.execute(action, state);
                    if !result.success {
                        println!("Trading action failed: {:?}", result.errors);
                    }
                    result.effects
                } else {
                    println!("Trading domain cannot handle action: {:?}", action);
                    vec![]
                }
            }
            SimAction::Consumption(action) => {
                if self.consumption.can_handle(action) {
                    let result = self.consumption.execute(action, state);
                    if !result.success {
                        println!("Consumption action failed: {:?}", result.errors);
                    }
                    result.effects
                } else {
                    println!("Consumption domain cannot handle action: {:?}", action);
                    vec![]
                }
            }
        }
    }
}

impl Default for DomainRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## `crates/engine/src/scenario.rs`

```rust
use crate::factory::AgentFactory;
use serde::Deserialize;
use sim_prelude::*;
use std::{collections::HashMap, str::FromStr};
use uuid::Uuid;

const _SCENARIO_NAMESPACE: Uuid = uuid::uuid!("6E62B743-2623-404B-84C8-45F48A85189A");

#[derive(Debug, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub description: String,
    config: ScenarioConfig,
    banks: Vec<BankConfig>,
    firms: Vec<FirmConfig>,
    consumers: Vec<ConsumerConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioConfig {
    iterations: u32,
    treasury_tenors_to_register: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankConfig {
    pub id: String,
    pub name: String,
    pub initial_reserves: f64,
    pub initial_bonds: Vec<BondConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmConfig {
    pub id: String,
    pub name: String,
    pub bank_id: String,
    pub recipe_name: String,
    pub initial_cash: f64,
    pub initial_inventory: Vec<InventoryConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsumerConfig {
    pub id: String,
    pub bank_id: String,
    pub initial_cash: f64,
    pub income: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BondConfig {
    pub tenor: String,
    pub face_value: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryConfig {
    pub good_slug: String,
    pub quantity: f64,
    pub unit_cost: f64,
}

impl Scenario {
    pub fn from_toml_str(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    pub fn initialize_state(&self) -> SimState {
        let mut state = SimState::default();
        state.config.iterations = self.config.iterations;
        state.financial_system.goods = goods::CATALOGUE.clone();

        let cb_id = state.financial_system.central_bank.id;
        let mut rng = rand::rng();
        let mut factory = AgentFactory::new(&mut state, &mut rng);

        let mut agent_ids: HashMap<String, AgentId> = HashMap::new();

        for bank_conf in &self.banks {
            let bank = factory.create_bank(bank_conf, cb_id);
            agent_ids.insert(bank_conf.id.clone(), bank.id);
        }

        for consumer_conf in &self.consumers {
            let bank_id = *agent_ids.get(&consumer_conf.bank_id).expect("Bank not found for consumer");
            let consumer = factory.create_consumer(consumer_conf, bank_id, cb_id);
            agent_ids.insert(consumer_conf.id.clone(), consumer.id);
        }

        for firm_conf in &self.firms {
            let bank_id = *agent_ids.get(&firm_conf.bank_id).expect("Bank not found for firm");
            let firm = factory.create_firm(firm_conf, bank_id, cb_id);
            agent_ids.insert(firm_conf.id.clone(), firm.id);
        }

        let goods_ref = &state.financial_system.goods;
        state.financial_system.exchange.register_goods_market(good_id!("petrol"), goods_ref);
        for tenor_str in &self.config.treasury_tenors_to_register {
            let tenor = Tenor::from_str(tenor_str).unwrap();
            state.financial_system.exchange.register_financial_market(FinancialMarketId::Treasury { tenor });
        }
        state.financial_system.exchange.register_financial_market(FinancialMarketId::SecuredOvernightFinancing);

        state
    }
}
```

---

## `crates/engine/src/scheduler.rs`

```rust
//! Tick scheduling and timing

// TODO: Implement scheduling logic


```

---

## `crates/engine/src/state_manager.rs`

```rust
//! State management and persistence

// TODO: Implement state management


```

---

