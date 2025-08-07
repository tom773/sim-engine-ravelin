# Economic Simulation Engine

A comprehensive agent-based economic simulation system built in Rust that models the interactions between various economic actors in a virtual economy.

## Overview

This codebase implements a sophisticated economic simulation where autonomous agents (banks, consumers, firms, and government entities) make decisions and interact through realistic financial markets and economic mechanisms. The simulation models real-world economic concepts including banking operations, production cycles, consumption patterns, fiscal policy, and financial market dynamics.

## Architecture

The system follows a modular, domain-driven architecture with clear separation of concerns across multiple layers:

### Core Architecture Layers

```
┌─────────────────┐
│     Engine      │  ← Orchestrates simulation loop
├─────────────────┤
│    Domains      │  ← Specialized economic sector handlers  
├─────────────────┤
│ Actions/Effects │  ← Command/Result pattern for state changes
├─────────────────┤
│   Sim Types     │  ← Core data structures and state
└─────────────────┘
```

### Key Architectural Patterns

- **Action/Effect Pattern**: Agents declare intentions via `SimAction`s, which are validated and executed to produce `StateEffect`s
- **Domain-Driven Design**: Economic sectors (banking, production, etc.) are encapsulated in specialized domain handlers
- **Decision Models**: Agent behavior is defined through pluggable `DecisionModel` implementations
- **Immutable State Transitions**: All state changes go through explicit, auditable effects

## Core Components

### 1. Simulation State (`sim_types`)
The foundation layer containing all data structures:
- **Agents**: Banks, Consumers, Firms, Government, Central Bank
- **Financial System**: Instruments (bonds, deposits, loans), balance sheets, markets
- **Economic Entities**: Goods, production recipes, inventory, trade orders
- **Time & Policy**: Fiscal policy, monetary policy, time progression

### 2. Actions & Effects (`sim_actions`, `sim_effects`)
The command layer implementing the action/effect pattern:
- **Actions**: Express agent intentions (deposit money, produce goods, trade bonds)
- **Effects**: Represent validated state changes to be applied
- **Validation**: Ensures actions are feasible given current state

### 3. Decision Models (`sim_decisions`)
The intelligence layer defining agent behavior:
- **DecisionModel Trait**: Universal interface for agent AI
- **Behavioral Models**: Basic rule-based models for each agent type
- **ML Integration**: Framework for machine learning-based decision making

### 4. Economic Domains (`domains/`)
Specialized handlers for different economic sectors:

#### Banking Domain
- Processes deposits, withdrawals, transfers
- Manages bank reserves and liquidity
- Handles interbank lending markets

#### Production Domain  
- Validates and executes production processes
- Manages hiring and inventory consumption
- Implements production recipes and efficiency

#### Trading Domain
- Handles market order placement and validation
- Settles executed trades with asset/payment transfers
- Manages order books across multiple markets

#### Consumption Domain
- Processes consumer purchases and consumption
- Validates purchasing power and inventory
- Models consumer spending behavior

#### Fiscal Domain
- Executes government taxation and spending
- Manages debt issuance and fiscal policy
- Handles transfer payments and public investment

#### Settlement Domain
- Processes time-based financial events
- Handles interest accrual and coupon payments
- Manages periodic financial settlements

### 5. Simulation Engine (`engine`)
The orchestration layer managing the simulation lifecycle:

#### Core Simulation Loop
Each simulation "tick" performs:
1. **Financial Updates**: Process interest, coupon payments
2. **Decision Collection**: Query all agents for desired actions  
3. **Action Execution**: Validate and execute actions via domains
4. **Effect Application**: Apply validated changes to state
5. **Market Clearing**: Match orders and generate trades
6. **Trade Settlement**: Execute financial transfers for trades
7. **Time Advancement**: Progress the simulation clock

#### Supporting Components
- **Domain Registry**: Routes actions to appropriate domain handlers
- **Agent Factory**: Creates agents and initial conditions from scenarios  
- **Scenario System**: Configures initial simulation state via TOML
- **Remote Interface**: NATS-based API for external control

## Economic Features

### Financial System
- **Realistic Instruments**: Cash, demand deposits, savings, bonds, loans
- **Balance Sheets**: Double-entry accounting for all agents
- **Interest Mechanics**: Daily accrual, periodic payments
- **Credit Systems**: Bank lending with reserves and liquidity constraints

### Market Mechanisms  
- **Order Books**: Bid/ask matching for goods and financial instruments
- **Price Discovery**: Market-driven pricing through order matching
- **Multiple Markets**: Separate markets for goods, bonds, overnight lending
- **Trade Settlement**: Automatic asset and payment transfers

### Agent Behaviors
- **Banks**: Reserve management, bond trading, market making
- **Consumers**: Income-based spending, saving decisions, consumption
- **Firms**: Production planning, hiring, inventory management, pricing
- **Government**: Tax collection, spending, debt issuance, fiscal policy

### Time-Based Processes
- **Interest Accrual**: Daily compound interest calculations
- **Coupon Payments**: Periodic bond coupon distributions  
- **Tax Collection**: Scheduled government revenue collection
- **Production Cycles**: Multi-period manufacturing processes

## Crate Organization

The codebase is organized into focused, single-responsibility crates:

### Core Crates (`core/`)
- `sim_types`: Foundational data structures
- `sim_actions`: Action definitions and validation
- `sim_effects`: Effect definitions and application
- `sim_decisions`: Decision model framework
- `sim_prelude`: Convenience re-exports

### Domain Crates (`domains/`)
- `banking`: Banking operations and bank AI
- `production`: Manufacturing and firm behavior  
- `trading`: Market operations and trade settlement
- `consumption`: Consumer behavior and purchases
- `fiscal`: Government operations and policy
- `settlement`: Financial settlement processes
- `prelude`: Domain handler re-exports

### Engine Crate (`engine/`)
- Simulation orchestration and main loop
- Scenario configuration and agent factory
- Domain registry and action routing
- CLI interface with NATS messaging

## Design Philosophy

### Modularity
Each economic sector is encapsulated in its own domain with clear interfaces, enabling independent development and testing of different economic mechanisms.

### Extensibility  
The action/effect pattern and decision model framework make it straightforward to add new agent types, economic mechanisms, or behavioral models without modifying existing code.

### Realism
The simulation models real economic concepts like double-entry accounting, interest accrual, market clearing, and multi-agent interactions rather than simplified abstractions.

### Auditability
All state changes flow through explicit, typed effects, making the simulation's behavior fully traceable and debuggable.

### Performance
The Rust implementation provides memory safety and performance suitable for large-scale simulations with many agents and complex interactions.

This architecture creates a flexible, realistic economic simulation platform suitable for research, policy analysis, and economic modeling applications.