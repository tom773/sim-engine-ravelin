Of course. Here is a README for the project.

---

# Economic Simulation Engine (v3)

This is a sophisticated agent-based model (ABM) for simulating complex economic and financial systems, written in Rust. It models the interactions between various economic agents (consumers, firms, banks, government) within a realistic financial infrastructure.

The engine is driven by a `SimulationEngine` struct that executes discrete time steps (`ticks`). Scenarios are loaded from TOML files to configure agents, goods, production recipes, and initial economic conditions.

## Core Simulated Systems

The simulation is built upon a foundation of modern financial infrastructure, ensuring a clear and robust separation of concerns:

* **Central Securities Depository (CSD):** Acts as the single source of truth for the ownership and settlement of all securities (e.g., bonds, equities).
* **Real-Time Gross Settlement (RTGS):** Manages a payment queue to process all inter-agent cash transfers, including trade settlements and wage payments.
* **Exchange:** Provides a central venue for matching orders in various markets:
    * Financial Markets (bonds, etc.)
    * Goods Markets
    * Labour Markets
* **Agent Balance Sheets:** Track only cash-equivalent instruments (deposits, reserves) and real assets (inventory). **Securities are never tracked on balance sheets.**

## System Architecture

The engine uses a domain-driven, event-based architecture to manage complexity.

### The Simulation Loop: `Intention` -> `Action` -> `Effect`

The core logic of the simulation follows a distinct three-stage pipeline each tick:

1.  **Intention:** Agents' `DecisionModel`s assess the simulation state and generate high-level goals (e.g., `MarketMakeTreasuries`).
2.  **Action:** **Domains** resolve these intentions into concrete, executable `SimAction`s (e.g., `PostMarketOrder`).
3.  **Effect:** The engine executes actions, which produce primitive `StateEffect`s (e.g., `PlaceOrderInBook`). These effects are the only things that can mutate the simulation state.

### Domain-Driven Design

The logic is separated into two types of domains:

* **Agent Domains** (`Banking`, `Production`, `Consumption`, etc.): Contain the decision-making logic and business rules specific to each agent type. They are responsible for resolving intentions into actions.
* **Transaction Domain:** A specialized domain that handles the execution of all inter-agent interactions, such as posting orders, initiating payments, and settling trades. It acts as a gateway to the core financial systems (CSD, RTGS, Exchange).

### Tick Scheduler

A Directed Acyclic Graph (DAG) defines the precise execution order of all processes within a single tick, from agent upkeep and decision-making to market clearing and trade settlement. This ensures that dependencies are respected and the simulation proceeds in a logical, deterministic order.

## Key Architectural Principles

To maintain consistency and prevent common simulation bugs like double-counting, the engine strictly enforces several invariants:

* **Single Source of Truth:** Each piece of financial data has one and only one home.
    * Securities live **only** in the CSD.
    * Payments are processed **only** by the RTGS.
    * Market orders are managed **only** by the Exchange.
* **No Direct State Modification:** Domains cannot directly change the state. They can only produce `StateEffect`s, which are applied by the engine in a controlled manner.
* **Delivery vs. Payment (DvP):** Securities settlement is tightly coupled with the RTGS payment system to ensure that securities are only transferred after the cash leg has successfully settled.