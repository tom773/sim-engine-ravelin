# Economic Simulation (Agent-Based)

An extensible economic simulation that models a real economy with interbank and credit markets, goods/services markets, labour, fiscal agents, and robust settlement processes. The system is built on Agent-Based Modelling (ABM): each agent selects actions given its current state, observes outcomes on the next tick, then updates beliefs and knowledge.

---

## Highlights

* **Multi-market microstructure**

  * Interbank (federal funds–style) lending and Treasuries
  * Goods & services exchange with order books and history
  * Labour matching and wage payments
  * Settlement for interest accrual, coupon payments, and transfers
* **Agent Decision Models** via a unified `DecisionModel` trait; the engine queries every agent each tick and executes their proposed actions. &#x20;
* **Pluggable Domains** (Banking, Trading, Production, Consumption, Fiscal, Labour, Settlement) registered dynamically through an inventory-based `DomainRegistry`.&#x20;
* **HTTP control/API** for headless runs, dashboards, and integrations (Axum on port **8060**).&#x20;

---

## Repository Layout

```
config/
  config.toml      # Scenario: agents, iterations, treasuries, etc.
  goods.toml       # Goods catalogue and production recipes

crates/
  engine/          # Simulation loop, scenario loading, HTTP server
  domains/         # Banking, Trading, Production, Consumption, Fiscal, Labour, Settlement
  sim_core/        # Types, actions/effects, markets, decision traits
  sim_macros/      # (proc-macros for domain registration)
```

* The engine crate documents the tick lifecycle (collect decisions → execute → clear markets → settle → advance time).&#x20;
* Domains are discovered and routed by `DomainRegistry`.&#x20;

---

## Quickstart

### 1) Build & run the HTTP server

```bash
# From the workspace root
cargo run -p engine --bin cli
```

* The server binds to `0.0.0.0:8060` and exposes a simple health endpoint. &#x20;
* The scenario TOML is embedded at build time.&#x20;

### 2) Initialise and tick

```bash
# Initialise the simulation engine (loads the compiled-in scenario)
curl -X POST http://localhost:8060/init

# Advance one tick
curl -X POST http://localhost:8060/sim/control/tick
```

Key endpoints (see full list below):

* `POST /init` – load scenario and create engine instance.&#x20;
* `POST /sim/control/tick` – advance one tick.&#x20;
* `GET /sim/analysis/stats` – macro/market stats snapshot.&#x20;
* `GET /healthz` and `GET /health` – health checks. &#x20;

---

## HTTP API (overview)

The router mounts the following (non-exhaustive) endpoints:

* **Simulation control & status**

  * `GET /healthz` – structured health (epoch & init status)
  * `POST /init` – construct engine from scenario
  * `POST /sim/control/tick` – advance one tick
  * `GET /sim/analysis/stats` – macro/market statistics 
  * `GET /agents/{agent_type}` and `/agents/{agent_type}/summary` – listings/summaries


* **Markets**

  * `GET /api/markets/overview` – high-level overview
  * Goods: catalogue, overview, orderbook, history at
    `/api/markets/goods/cat`, `/goods/overview`, `/goods/{good_id}/orderbook`, `/goods/{good_id}/history`
  * Financial: overview, orderbook, history at
    `/api/markets/financial/overview`, `/financial/{instrument_id}/orderbook`, `/financial/{instrument_id}/history`

## How the Simulation Works

### 1) Tick lifecycle

Each tick the engine:

1. Processes financial updates (interest accrual, etc.)
2. Queries every agent’s `DecisionModel` to collect `SimAction`s
3. Validates & executes actions via `DomainRegistry` → `StateEffect`s
4. Applies effects to state
5. Clears markets to produce trades
6. Generates and applies settlement effects
7. Advances simulation time


### 2) Agent behaviour (selected examples)

* **Banks** (`BasicBankDecisionModel`)

  * **Reserve management:** compare current reserves to a buffer above required reserves; post overnight interbank **bids** (shortfall) or **asks** (surplus) around the policy-rate floor/ceiling; price converted from bps to a daily rate. &#x20;
  * **Treasury market making:** (also implemented) uses portfolio state to post quotes in Treasury markets.&#x20;

* **Firms** (`BasicFirmDecisionModel`)

  * Hire up to a target headcount; when inputs and staff are available, **produce** according to a recipe; post **asks** in the goods market at a markup over unit cost; pay weekly wages. &#x20;

* **Consumers** and **Labour**

  * Consumption domain executes purchases/consumption and can split demand across best available asks; the labour domain posts offers/applications and clears a simple matching market. &#x20;

### 3) Settlement

Periodic processes maintain financial plumbing:

* **AccrueInterest** and **PayInterest** on instruments
* **ProcessCouponPayment** for bonds
  These translate into concrete financial `StateEffect`s (including resets of accrued interest after payment).  &#x20;

## Next
* Consolidate trade settlement logic
* Ensure trades are tracked, and accessible by the API
#### * Implement labour market - currently a stub as it requires different plumbing to goods & financial markets
* Balance starting config to allow functioning in overnight funding markets
#### * Decision models should access market stats for decision making
#### * Consumer & Corporate Debt Markets (Mortgages, Corporate Bond issues)
* More expansive derivative instrument system - swaps, structured products, MBS, ABS, etc...
