# Ravelin Analytics - Economic Simulation Engine

This engine simulates economic interactions between consumers, firms, and banks. It uses an Agent-Based Modelling approach, where each entity is an agent that will learn based on what happened to it the previous tick, and make a decision to improve its situation. 

It uses machine learning models trained on actual Consumer Expenditure Survey data from the Bureau of Labor Statistics to make realistic spending decisions. Firms will use Cobb-Douglas production functions to try and reach optimal production. 

Banks don't really make decisions yet, but there is a complex financial system built into the simulation that models the plumbing of an economy. 

## Project Structure

```
crates/
├── core/         # Core business logic (placeholder for now)
├── engine/       # The simulation engine itself
├── ml/           # Machine learning model training and inference
└── shared/       # Shared types used across all crates
```

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

## Future Plans

- Expand the types of decision for firms and consumers.
- Link these with a SimAction that will actually execute the decision (action) such as applying changes to balance sheets, or executing an interbank transaction.
- Align consumer agent features set with ML model's feature set.
- Allow the sim action API to be accessed from the Axum API, independant of a tick-based sim run. Purspose is to build the world out and watch it grow.
- Market clearing mechanisms. All transactions should be checked and settled within the financial system, with double entry accounting taking place.
- Way down the line: Model distance. Each agent should have a location. What happens to the coffee shops if a big firm goes bust in a certain region? 
