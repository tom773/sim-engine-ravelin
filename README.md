# Economics Simulation — Architecture & Internals

This project is a domain-driven agent-based macro/markets simulator. Agents (banks, firms, consumers, government) run decision models that emit **actions**. Domain handlers validate and translate actions into **effects**, and the engine applies those effects to mutate the global **state**.

## Workspace layout

```
.
├─ config/                 # Scenario + goods/recipes definitions
├─ crates/
│  ├─ sim_core/            # Actions, effects, types, state, utils
│  ├─ domains/             # Domain logic (banking, consumption, etc.)
│  ├─ engine/              # Orchestration + CLI
│  ├─ sim_macros/          # Proc macros (e.g., derive SimDomain)
│  └─ ml/                  # Offline analysis / training helpers
├─ Cargo.toml              # Workspace members & shared deps
└─ .rustfmt.toml
```

Workspace members are declared in the root `Cargo.toml` (engine, sim\_core, domains, sim\_macros, ml).&#x20;

### Configs

* `config/config.toml` defines a named scenario, number of iterations, treasury tenors, initial banks/firms/consumers, and starting positions (reserves, bonds, cash, income).  &#x20;
* `config/goods.toml` declares tradable goods, CPI weights, and production recipes (e.g., “Oil Refining” produces petrol from oil with specified efficiency, labour, and capital).  &#x20;

## Core concepts (`crates/sim_core`)

* **Actions**: typed intents emitted by agents and domains. Modules exist for banking, consumption, fiscal, labour, production, settlement, trading, plus validation helpers.&#x20;
* **Effects**: concrete state transitions grouped by concern (agent/equipment, financial instruments, inventory, market, application).&#x20;
* **Types**: agents, balance sheets, instruments (cash/deposits/bonds/reserves…), goods/markets/time/policy, and shared traits/macros.&#x20;

> ⚙️ Effects are later applied to the `SimState` via `state.apply_effects(&effects)`, as seen in domain unit tests.&#x20;

## Domain pattern (`crates/domains`)

Each domain cleanly separates:

* **Behavior** (“decision models”) → *what an agent decides to do this tick*.
* **Domain execution** → *how an action is validated and translated into effects*.

This separation is documented directly in each `mod.rs` (e.g., Banking and Consumption). &#x20;

Domains use a `#[derive(SimDomain)]` macro to generate boilerplate impls. For example, `BankingDomain` derives `SimDomain`.&#x20;

### Banking domain

**Behavior**: `BasicBankDecisionModel` runs two strategies each tick:

1. **Reserve management**: computes required reserves from deposits and a buffer; posts bids/asks in the secured overnight market around a target (floor/ceiling) policy rate translated into a daily price. &#x20;
2. **Treasury market making**: scans existing government bond holdings, computes bid/ask yields (policy rate ± spread + term premium), converts to prices, and posts quotes by tenor.   &#x20;

**Execution**: `BankingDomain::execute` routes to validated handlers for `Deposit`, `Withdraw`, `Transfer`, `PayWages`, `UpdateReserves` (stub), or `InjectLiquidity`. &#x20;

Key flows:

* **Deposit**: creates a demand deposit (rate derived from policy + bank spread), debits depositor cash, credits the bank’s reserves at the central bank. Effects are emitted as instrumental creations/updates/removals.  &#x20;
* **Withdraw/Transfer**: composite logic spends cash first, then deposits; moves reserves from payer-bank to payee-bank, credits payee with cash or a new deposit depending on counterparty type.     &#x20;
* **InjectLiquidity**: a fiscal-style cash drop to consumers (creates cash instruments).&#x20;

Unit tests demonstrate the engine side applying effects to state:

* Uses cash first if sufficient; deposits untouched.&#x20;
* Composite cash+deposit transfer and reserve movement across banks.&#x20;

### Consumption domain

**Behavior**:

* `BasicConsumerDecisionModel`: sets a propensity to consume vs. save from income+liquid assets; buys petrol at best and deposits the remainder.  &#x20;
* `CESConsumerDecisionModel`: macro-CES shares across goods using current market prices; MPC adjusts with real rate (policy minus expected inflation); handles job applications if unemployed; saves remainder.     &#x20;

**Execution**:

* `Purchase`: validates funds and seller inventory; routes payment via `BankingDomain::execute_transfer`; then removes seller inventory and credits buyer inventory at the transacted price.   &#x20;
* `PurchaseAtBest`: scans/partitions asks (lowest first), places bids sized by remaining notional, and leaves order placement to the market.  &#x20;
* `Consume`: removes inventory only.&#x20;

> The domain’s “can handle”, “validate”, and “execute” phases are explicit and mirror the Banking domain’s structure.  &#x20;

### Other domains

The file list shows additional domain areas—fiscal, labour, production, settlement, trading—implemented under `crates/domains/src/...`. (See specific files such as `fiscal/behaviour.rs`, `labor/domain.rs`, etc.)&#x20;

## The Action → Effect → State pipeline

1. **Decision phase** (agent AI): Agents implement `DecisionModel::decide(&dyn Any, &SimState, &mut Rng)` and return a `Vec<SimAction>`. Examples include banks posting bids/asks or consumers purchasing/saving. &#x20;

2. **Validation & execution** (domain services): Each domain’s `execute` first runs `validate(...)` and, if successful, produces a list of `StateEffect`s describing financial, inventory, and market changes. (See Banking and Consumption domains.)   &#x20;

3. **Application** (state mutation): The engine (or tests) calls `state.apply_effects(&effects)`; effects like `CreateInstrument`, `UpdateInstrument`, `RemoveInstrument`, `AddInventory`, `RemoveInventory`, and `Market::PlaceOrderInBook` are applied to balance sheets, inventories, markets, and ledgers.    &#x20;

## Engine & CLI (`crates/engine`)

The engine crate exposes a CLI and modules named `executor.rs`, `factory.rs`, `registry.rs`, and `scenario.rs`, plus HTTP routes for a bridge. This crate is responsible for wiring scenarios, domains, and the simulation loop. (See file list.)&#x20;

## Macros (`crates/sim_macros`)

Derive macros are provided here and are used by domains (e.g., `#[derive(SimDomain)]` on `BankingDomain`). &#x20;

## ML & analysis (`crates/ml`)

An auxiliary crate for data processing and model training pipelines (e.g., `process.rs`, `train.rs`, with `polars`, `linfa`, `lightgbm3` in workspace deps). (See file list and workspace deps.) &#x20;

## Coding patterns & conventions

* **Domain separation**: behavior vs. execution/validation (clear cross-domain pattern). &#x20;
* **Effect-first state changes**: domains never mutate the state directly; they return `StateEffect`s. Tests then `apply_effects`.&#x20;
* **Financial plumbing**: cash/deposits/reserves are instrumented; transfers split across cash then deposits; reserve settlement moves payer-bank → payee-bank reserves. &#x20;
* **Market microstructure**: order book participation via explicit `MarketEffect::PlaceOrderInBook` with price/quantity logic.&#x20;

## File-by-file purpose (selected)

* `crates/sim_core/src/actions/*`: Action enums and validators by domain.&#x20;
* `crates/sim_core/src/effects/*`: Effect enums (financial/inventory/market/etc.).&#x20;
* `crates/sim_core/src/types/*`: Agent structs, balance-sheet/instrument types, goods/markets/time/policy, traits.&#x20;
* `crates/domains/src/banking/behavior.rs`: Bank decision model (reserves, treasuries).  &#x20;
* `crates/domains/src/banking/domain.rs`: Validates/executes deposits, withdrawals, transfers, liquidity injections.&#x20;
* `crates/domains/src/consumption/behavior.rs`: Consumer decision models (Basic, CES, Parametric MPC).  &#x20;
* `crates/domains/src/consumption/domain.rs`: Validates/executes purchases (direct/best) and consumption; routes payments via banking. &#x20;
* `crates/engine/cli/*`: CLI entrypoints and routes; `engine/src/*`: (executor/factory/registry/scenario).&#x20;
* `config/*.toml`: scenario setup and goods/recipes. &#x20;

## How actions become state mutations (worked examples)

* **Consumer buys petrol**

  1. Behavior: `BasicConsumerDecisionModel` computes spend/save and emits `ConsumptionAction::PurchaseAtBest { max_notional }`. &#x20;
  2. Domain: `execute_purchase_at_best` sorts asks, places `Bid` effects with prices/quantities sized to remaining notional.  &#x20;
  3. Engine: applies `MarketEffect` to order book; later matches determine cash/inventory effects (via market/settlement domains).

* **Direct purchase (buyer ↔ seller)**

  1. Domain validates seller inventory and buyer liquidity, then routes payment through Banking, then emits inventory transfer effects.   &#x20;

* **Bank transfer (`create_transfer_effects`)**
  Splits payment into **cash** (debit payer cash; credit payee cash or payee-bank reserves if paying a bank) and then **deposits** (debit payer deposit, move payer-bank reserves to payee-bank, and credit payee deposit if payee is non-bank). Each step is a `FinancialEffect`.    &#x20;

## Extending the sim

* Add a domain: create `{domain}/behavior.rs` + `{domain}/domain.rs`, derive `SimDomain`, and implement `can_handle/validate/execute`. (See Banking/Consumption patterns.) &#x20;
* Add a decision model: implement `DecisionModel::decide` for your agent type and register it in agent construction. (See `BasicBankDecisionModel` / CES model.) &#x20;

## Notes

* Formatting is standardized via `.rustfmt.toml` (compressed fn params, max width 120, etc.).&#x20;
* The workspace pins common crates for reproducibility across `domains`, `engine`, `sim_core`, `sim_macros`, and `ml`.&#x20;

---
