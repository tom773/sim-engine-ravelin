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
### Roadmap

#### 1: Deepening the Financial System

Your current financial system is good, but "professional" means modeling the nitty-gritty of credit, risk, and asset pricing.

1.  **A Formal Credit & Lending System:**
    *   **Loan Applications & Credit Scoring:** Instead of firms/consumers just getting money, they must *apply* for loans. Banks should run a credit scoring model (using agent history, debt-to-income, net worth) to approve/deny or set interest rates.
    *   **Collateral & Secured Lending:** Introduce collateral. Firms can pledge their real assets (equipment, buildings) for loans. This lowers the bank's risk and the interest rate. If they default, the bank seizes the asset.
    *   **Loan Covenants:** Loans can come with conditions, e.g., "the firm's net worth must not fall below X." Violating these can trigger a default.

2.  **End-to-End Asset Markets & Speculation:**
    *   **Full Order Book Matching:** Implement the `match_orders` function in your markets. When a high bid meets a low ask, a trade occurs, a market price is set, and assets/cash are exchanged.
    *   **Bonds & Yield Curve:** Introduce government bonds of different maturities (e.g., 2-year, 10-year, 30-year). The different interest rates on these form the **yield curve**, a critical economic indicator. Agents should be able to buy and sell these bonds.
    *   **Asset Bubbles & Crashes:** With a real market, agents can speculate. They might buy an asset not for its fundamental value, but because they expect its price to rise. This is how bubbles form. When expectations shift, they can crash.

3.  **Financial Contagion & Systemic Risk:**
    *   **Interbank Lending Network:** The SOFR market you've started is key. Model the specific network of who owes whom in the banking system.
    *   **Default Cascades:** Now, if a major firm defaults on its loan to Bank A, Bank A might not have enough money to pay its own debts to Bank B. This is a **default cascade**. Your simulation could model this and measure systemic risk.
    *   **Bank Runs:** If a consumer's `PersonalityArchetype` is `AnxiousAndy` and he hears a rumor that his bank is in trouble, he might start a bank run by withdrawing all his money, causing others to follow suit.

#### 2: Introducing the Government Sector

A modern economy is incomplete without a government. This introduces a powerful new agent with unique abilities.

1.  **Fiscal Policy (Taxes & Spending):**
    *   **Taxation:** Add income taxes for consumers, corporate taxes for firms, and potentially sales taxes on goods. This removes money from the private sector.
    *   **Government Spending:** The government uses tax revenue to spend money on things that don't have a direct market: infrastructure, defense, and social programs (which becomes income for some agents).
    *   **Automatic Stabilizers:** Implement unemployment benefits. When an agent loses their job, the government gives them a small income. This automatically cushions the economy during a recession.

2.  **Government Debt & Monetary Policy Interaction:**
    *   **Issuing Government Bonds:** If the government spends more than it taxes (a deficit), it funds this by issuing the government bonds mentioned in Tier 1.
    *   **Quantitative Easing (QE):** The `CentralBank` can now do more than set rates. It can *create money* to buy government bonds (or other assets) from commercial banks, injecting massive liquidity into the system. This is a core feature of post-2008 economics.

#### 3: Enhancing the Real Economy (Firms & Production)

Firms are the engine of the economy. Let's make them more sophisticated.

1.  **Capital Investment (CapEx) & Productivity:**
    *   **Capital Goods:** Introduce a new category of goods: `Capital`. These are things like machines, factories, and software.
    *   **Investment Decisions:** Firms should make long-term decisions to *invest* in capital by taking out loans. This capital increases their `productivity` over time, allowing them to produce more with fewer employees. This creates a fundamental growth dynamic.

2.  **Multi-stage Production & Supply Chains:**
    *   Instead of `oil -> petrol`, model a full chain: `iron ore -> steel -> car parts -> cars`.
    *   This creates a network of firms that are customers and suppliers to each other. A shock to the steel producer now ripples through the entire car industry. This is crucial for modeling supply chain disruptions.

3.  **Inventory Management & Business Cycles:**
    *   Firms shouldn't just produce based on a simple rule. They should manage their inventory levels based on their expectations of future sales.
    *   If they expect a boom, they build up inventory. If they expect a recession, they sell it off and cut production. This behavior (the "inventory cycle") is a major driver of short-term business cycles.

#### 4: Modeling a Realistic Labor Market

1.  **Endogenous Wages & Unemployment:**
    *   **Labor Market:** Create a true "market" for labor. Wages should not be fixed but should rise and fall based on the supply (consumers looking for work) and demand (firms hiring).
    *   **Unemployment Pool:** Create an explicit pool of unemployed agents. This allows you to track the **unemployment rate**, one of the most important outputs of any macroeconomic model.
    *   **Job Search:** Unemployed agents must actively "search" for jobs. Firms "post" job openings.

2.  **Skill Levels & Education:**
    *   Introduce different labor types: e.g., `unskilled`, `skilled`, `highly_skilled`.
    *   Firms' production recipes might require specific skill mixes. This can lead to skill shortages and wage differentials.
    *   Consumers can make a long-term decision to "invest in education" (forego income now) to become skilled, increasing their future earning potential.

#### 5: Expanding Agent Intelligence

Your agent personalities are a great start. The next level is making them learn and adapt.

1.  **Learning & Adaptive Expectations:**
    *   Agents shouldn't have fixed parameters. They should *learn* and *update* their beliefs.
    *   If inflation has been high for the last year, agents should start to *expect* high inflation and demand higher wages. This is a core concept in modern macroeconomics.
    *   This can be implemented with simple adaptive rules or more advanced techniques like reinforcement learning.

2.  **Network Effects & Social Influence:**
    *   Agents don't make decisions in a vacuum. A consumer's decision to buy something could be influenced by what their "friends" in the simulation are buying. This can be modeled with a social network graph.
