# Ravelin Analytics - Economic Simulation Engine 🚀

A Rust-based economic simulation engine that models consumer spending behavior using real-world data! This is the core simulation component extracted from the larger Ravelin Analytics platform.

## What is this?

This engine simulates economic interactions between consumers, firms, and banks. It uses an Agent-Based Modelling approach, where each entity is an agent that will learn based on what happened to it the previous tick, and make a decision to improve its situation. It uses machine learning models trained on actual Consumer Expenditure Survey data from the Bureau of Labor Statistics to make realistic spending decisions. Firms will use Cobb-Douglas production functions to try and reach optimal production. Banks don't really make decisions, but there is a complex financial system built into the simulation that models the plumbing of an economy.

## Features

- **ML-Powered Consumer Behavior**: Train decision models on real BLS consumer expenditure data
- **Multi-Agent Simulation**: Consumers, firms, banks, and a central bank all interacting
- **Financial System Modeling**: Balance sheets, various financial instruments (loans, deposits, bonds), and realistic banking operations
- **REST API**: Run simulations via a simple HTTP interface
- **Real-time State Management**: Watch the economy evolve tick by tick

## Tech Stack

- **Rust** (because we need that blazing fast performance ⚡)
- **Machine Learning**: LightGBM for regression, custom feature engineering
- **Web Framework**: Axum for the REST API
- **Async Runtime**: Tokio for handling concurrent operations
- **Data Processing**: Polars for fast CSV processing, ndarray for ML operations

## Project Structure

```
crates/
├── core/         # Core business logic (placeholder for now)
├── engine/       # The simulation engine itself
├── ml/           # Machine learning model training and inference
└── shared/       # Shared types used across all crates
```

## Getting Started

### Prerequisites

- Rust (latest stable version)
- About 500MB free space for the BLS data files

### Training the ML Model

First, let's get a consumer decision model trained up!

1. Clone the repo:
   ```bash
   git clone git@github.com:tom773/sim-engine-ravelin.git
   cd sim-engine-ravelin
   ```

2. Download Consumer Expenditure data:
   - Go to the [BLS Public Use Microdata Files](https://www.bls.gov/cex/pumd_data.htm)
   - Find an Interview Survey ZIP file (any recent year works)
   - Download and extract the CSV files:
   ```bash
   mkdir -p ./crates/ml/data
   # Download the ZIP file and extract CSVs to ./crates/ml/data/
   # Make sure the .csv files are directly in this folder!
   ```

3. Train the model:
   ```bash
   cargo run --bin ml -- train
   ```
   This creates `ce_trained_model.bin` in your project root.

4. Test it out:
   ```bash
   cargo run --bin ml -- predict    # See some predictions
   cargo run --bin ml -- validate   # Run validation tests
   ```

### Running the Simulation

Fire up the simulation API:

```bash
cargo run --bin cli
```

This starts a REST API on `http://localhost:8070`. 

#### API Endpoints

- `GET /health` - Check if the server is alive
- `GET /state` - Get the current simulation state
- `GET /clear` - Reset the simulation
- `GET /make_bank` - Create some test banks with bonds
- `GET /do_tx` - Execute a test transaction

### Example: Watch an Economy Run

```bash
# Terminal 1: Start the server
cargo run --bin cli

# Terminal 2: Create some banks
curl http://localhost:8070/make_bank

# Watch the state
curl http://localhost:8070/state | jq
```

## How It Works

The simulation follows this flow:

1. **Initialization**: Creates a financial system with a central bank, commercial banks, consumers, and firms
2. **Consumer Decisions**: Each tick, consumers use the ML model to decide how much to spend based on their income, age, family size, etc.
3. **Financial Transactions**: Money flows between agents through the banking system
4. **Balance Sheet Updates**: All transactions update the relevant balance sheets in real-time

The ML model is trained on actual consumer spending patterns, so the simulated consumers behave somewhat realistically!

## Current Limitations

- The simulation loop in the CLI is still WIP (you'll see it tries to run continuously but needs some work)
- No persistence yet - everything is in-memory 
- Limited behavior (consumers, firms, banks, etc... don't actually do anything)
- The transaction processing is simplified for now

## Future Plans

- Expand the types of decision for firms and consumers.
- Link these with a SimAction that will actually execute the decision (action) such as applying changes to balance sheets, or executing an interbank transaction.
- Align consumer agent features set with ML model's feature set.
- Allow the sim action API to be accessed from the Axum API, independant of a tick-based sim run. Purspose is to build the world out and watch it grow.
- Market clearing mechanisms. All transactions should be checked and settled within the financial system, with double entry accounting taking place.
- Way down the line: Model distance. Each agent should have a location. What happens to the coffee shops if a big firm goes bust in a certain region? 

## Development Notes

This is extracted from a larger project, so you might see references to an `api` crate that's not included here. The full platform includes a SvelteKit frontend and additional API services for enterprise risk analysis.

The code is structured to be extensible - adding new agent types or financial instruments is pretty straightforward thanks to Rust's trait system.

