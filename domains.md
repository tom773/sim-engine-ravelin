# Included Files

- `crates/domains/src/banking/behavior.rs`
- `crates/domains/src/banking/domain.rs`
- `crates/domains/src/banking/mod.rs`
- `crates/domains/src/consumption/behavior.rs`
- `crates/domains/src/consumption/domain.rs`
- `crates/domains/src/consumption/mod.rs`
- `crates/domains/src/fiscal/behaviour.rs`
- `crates/domains/src/fiscal/domain.rs`
- `crates/domains/src/fiscal/mod.rs`
- `crates/domains/src/labour/mod.rs`
- `crates/domains/src/lib.rs`
- `crates/domains/src/prelude/mod.rs`
- `crates/domains/src/production/behavior.rs`
- `crates/domains/src/production/domain.rs`
- `crates/domains/src/production/mod.rs`
- `crates/domains/src/settlement/domain.rs`
- `crates/domains/src/settlement/mod.rs`
- `crates/domains/src/trading/domain.rs`
- `crates/domains/src/trading/mod.rs`

---

## `crates/domains/src/banking/behavior.rs`

```rust
use rand::RngCore;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::any::Any;

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct BasicBankDecisionModel;

#[typetag::serde]
impl DecisionModel for BasicBankDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let bank = match agent.downcast_ref::<Bank>() {
            Some(b) => b,
            None => return vec![],
        };
        
        let mut intentions = Vec::new();
        let fs = &state.financial_system;

        self.assess_liquidity_needs(bank, fs, &mut intentions);
        self.consider_treasury_market_making(bank, fs, &mut intentions, rng);
        self.evaluate_lending_opportunities(bank, fs, &mut intentions);

        intentions
    }
}

impl BasicBankDecisionModel {
    fn assess_liquidity_needs(&self, bank: &Bank, fs: &FinancialSystem, intentions: &mut Vec<SimIntention>) {
        let total_deposits = fs.get_total_liabilities(&bank.id);
        let required_reserves = total_deposits * fs.central_bank.reserve_requirement;
        let desired_buffer = total_deposits * 0.02;
        let target_reserve_level = required_reserves + desired_buffer;

        let current_reserves = fs.get_bank_reserves(&bank.id).unwrap_or(0.0);
        let reserve_surplus_or_shortfall = current_reserves - target_reserve_level;

        let policy_rate_bps = fs.central_bank.policy_rate_bps;
        let acceptable_rate_range = 25.0;
        let target_rate_bps = policy_rate_bps + (acceptable_rate_range / 2.0);

        if reserve_surplus_or_shortfall < -1.0 {
            let amount_needed = -reserve_surplus_or_shortfall;
            intentions.push(SimIntention::BorrowReserves {
                agent_id: bank.id,
                amount: amount_needed,
                target_rate_bps,
            });
        } else if reserve_surplus_or_shortfall > 1.0 {
            let amount_to_lend = reserve_surplus_or_shortfall * 0.75;
            if amount_to_lend > 100.0 {
                intentions.push(SimIntention::LendExcessReserves {
                    agent_id: bank.id,
                    amount: amount_to_lend,
                    target_rate_bps,
                });
            }
        }
    }

    fn consider_treasury_market_making(&self, bank: &Bank, fs: &FinancialSystem, intentions: &mut Vec<SimIntention>, rng: &mut dyn RngCore) {
        let liquidity = fs.get_liquid_assets(&bank.id);
        if liquidity < 10000.0 {
            return;
        }

        let quantity_per_tenor = 5.0;

        for (market_id, _market) in &fs.exchange.financial_markets {
            if let FinancialMarketId::Treasury { tenor } = market_id {
                if self.should_make_market_for_tenor(tenor, bank, fs) {
                    let (bid_yield, ask_yield) = self.calculate_yield_quotes(*tenor, fs, rng);
                    
                    intentions.push(SimIntention::MarketMakeTreasuries {
                        agent_id: bank.id,
                        tenor: *tenor,
                        quantity: quantity_per_tenor,
                        bid_yield_bps: bid_yield,
                        ask_yield_bps: ask_yield,
                    });
                }
            }
        }
    }

    fn should_make_market_for_tenor(&self, tenor: &Tenor, _bank: &Bank, _fs: &FinancialSystem) -> bool {
        match tenor {
            Tenor::T2Y | Tenor::T5Y | Tenor::T10Y => true,
            Tenor::T30Y => false,
        }
    }

    fn calculate_yield_quotes(&self, tenor: Tenor, fs: &FinancialSystem, rng: &mut dyn RngCore) -> (BasisPoints, BasisPoints) {
        let policy_rate_bps = fs.central_bank.policy_rate_bps;
        
        let term_premium = match tenor {
            Tenor::T2Y => 15.0,
            Tenor::T5Y => 35.0,
            Tenor::T10Y => 50.0,
            Tenor::T30Y => 65.0,
        };

        let bid_ask_spread_bps = rng.random_range(15.0..30.0);
        
        let base_yield = policy_rate_bps + term_premium;
        let bid_yield_bps = base_yield + (bid_ask_spread_bps / 2.0);
        let ask_yield_bps = base_yield - (bid_ask_spread_bps / 2.0);

        (bid_yield_bps, ask_yield_bps)
    }

    fn evaluate_lending_opportunities(&self, bank: &Bank, fs: &FinancialSystem, _intentions: &mut Vec<SimIntention>) {
        let available_capital = fs.get_liquid_assets(&bank.id) - 5000.0;
        
        if available_capital > 1000.0 {
        }
    }
}
```

---

## `crates/domains/src/banking/domain.rs`

```rust
use serde::{Deserialize, Serialize};
use sim_core::*;
use crate::{Any, Domain, DomainResult, DomainValidator, ResolutionContext, ResolutionResult, ResolutionPhase};
extern crate inventory;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankingDomain {}

impl BankingDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for BankingDomain {
    fn name(&self) -> &'static str { 
        "Banking" 
    }

    fn resolve_intention(&self, intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::DepositFunds { agent_id, bank, amount } => {
                vec![SimAction::Banking(BankingAction::Deposit { 
                    agent_id: *agent_id, bank: *bank, amount: *amount 
                })]
            },
            
            SimIntention::WithdrawFunds { agent_id, bank, amount } => {
                vec![SimAction::Banking(BankingAction::Withdraw { 
                    agent_id: *agent_id, bank: *bank, amount: *amount 
                })]
            },
            
            SimIntention::PayWages { employer, employee, amount } => {
                vec![SimAction::Banking(BankingAction::PayWages { 
                    agent_id: *employer, employee: *employee, amount: *amount 
                })]
            },
            
            SimIntention::CollectTaxes { government_id, target, amount } => {
                vec![SimAction::Banking(BankingAction::Transfer { 
                    from: *target, to: *government_id, amount: *amount 
                })]
            },
            
            SimIntention::InjectLiquidity => {
                vec![SimAction::Banking(BankingAction::InjectLiquidity)]
            },
            
            SimIntention::LendExcessReserves { agent_id, amount, target_rate_bps } => {
                self.resolve_reserve_lending(*agent_id, *amount, *target_rate_bps)
            },

            SimIntention::BorrowReserves { agent_id, amount, target_rate_bps } => {
                self.resolve_reserve_borrowing(*agent_id, *amount, *target_rate_bps)
            },
            
            _ => return None,
        };
        
        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::DepositFunds { .. } |
            SimIntention::WithdrawFunds { .. } |
            SimIntention::PayWages { .. } |
            SimIntention::CollectTaxes { .. } |
            SimIntention::InjectLiquidity => Some(ResolutionPhase::Independent),
            
            SimIntention::LendExcessReserves { .. } |
            SimIntention::BorrowReserves { .. } => Some(ResolutionPhase::Market),
            
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let banking_action = match action {
            SimAction::Banking(action) => action,
            _ => return DomainResult::failure(vec!["Not a banking action".to_string()]),
        };

        if let Err(error) = self.validate(banking_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match banking_action {
            BankingAction::Deposit { agent_id, bank, amount } => {
                self.execute_deposit(*agent_id, *bank, *amount)
            },
            BankingAction::Withdraw { agent_id, bank, amount } => {
                self.execute_withdraw(*agent_id, *bank, *amount)
            },
            BankingAction::Transfer { from, to, amount } => {
                self.execute_transfer(*from, *to, *amount)
            },
            BankingAction::PayWages { agent_id, employee, amount } => {
                self.execute_pay_wages(*agent_id, *employee, *amount)
            },
            BankingAction::UpdateReserves { bank: _, amount_change: _ } => {
                DomainResult::failure(vec!["Reserve updates not yet implemented".to_string()])
            },
            BankingAction::InjectLiquidity => {
                self.execute_inject_liquidity(state)
            },
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl BankingDomain {
    fn resolve_reserve_lending(&self, agent_id: AgentId, amount: f64, target_rate_bps: BasisPoints) -> Vec<SimAction> {
        let market_id = FinancialMarketId::FederalFundsOvernight;
        let daily_rate = market_id.annual_bps_to_daily_rate(target_rate_bps);
        let price = 1.0 / (1.0 + daily_rate);

        vec![SimAction::Trading(TradingAction::PostAsk {
            agent_id,
            market_id: MarketId::Financial(market_id),
            quantity: amount,
            price,
        })]
    }

    fn resolve_reserve_borrowing(&self, agent_id: AgentId, amount: f64, target_rate_bps: BasisPoints) -> Vec<SimAction> {
        let market_id = FinancialMarketId::FederalFundsOvernight;
        let daily_rate = market_id.annual_bps_to_daily_rate(target_rate_bps);
        let price = 1.0 / (1.0 + daily_rate);

        vec![SimAction::Trading(TradingAction::PostBid {
            agent_id,
            market_id: MarketId::Financial(market_id),
            quantity: amount,
            price,
        })]
    }
}

impl BankingDomain {
    fn validate(&self, action: &BankingAction, state: &SimState) -> Result<(), String> {
        match action {
            BankingAction::Deposit { agent_id, bank, amount } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::agent_exists(*agent_id, state)?;
                DomainValidator::bank_exists(*bank, state)?;

                let available_cash = state.financial_system.get_cash_assets(agent_id);
                if available_cash < *amount {
                    return Err(format!(
                        "Insufficient cash for deposit: agent has ${:.2}, needs ${:.2}",
                        available_cash, amount
                    ));
                }
                Ok(())
            }
            BankingAction::Withdraw { agent_id, bank, amount } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::agent_exists(*agent_id, state)?;
                DomainValidator::bank_exists(*bank, state)?;
                
                let available_deposits = state.financial_system.get_deposits_at_bank(agent_id, bank);
                if available_deposits < *amount {
                    return Err(format!(
                        "Insufficient deposits for withdrawal: agent has ${:.2}, needs ${:.2}",
                        available_deposits, amount
                    ));
                }
                Ok(())
            }
            BankingAction::Transfer { from, to, amount }
            | BankingAction::PayWages { agent_id: from, employee: to, amount } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::agent_exists(*from, state)?;
                DomainValidator::agent_exists(*to, state)?;
                let available_funds = state.financial_system.get_liquid_assets(from);
                if available_funds < *amount {
                     return Err(format!(
                        "Insufficient liquid assets for transfer: agent has ${:.2}, needs ${:.2}",
                        available_funds, amount
                    ));
                }
                Ok(())
            }
            BankingAction::UpdateReserves { bank, amount_change: _ } => {
                DomainValidator::bank_exists(*bank, state)
            }
            BankingAction::InjectLiquidity => Ok(()),
        }
    }
}

impl BankingDomain {
    fn execute_deposit(&self, depositor: AgentId, bank: AgentId, amount: f64) -> DomainResult {
        let effect = StateEffect::Financial(FinancialEffect::DepositFunds { depositor, bank, amount });
        DomainResult::success(vec![effect])
    }

    fn execute_withdraw(&self, account_holder: AgentId, bank: AgentId, amount: f64) -> DomainResult {
        let effect = StateEffect::Financial(FinancialEffect::WithdrawFunds { account_holder, bank, amount });
        DomainResult::success(vec![effect])
    }

    fn execute_transfer(&self, from: AgentId, to: AgentId, amount: f64) -> DomainResult {
        let effect = StateEffect::Financial(FinancialEffect::TransferFunds { from, to, amount });
        DomainResult::success(vec![effect])
    }

    fn execute_pay_wages(&self, employer: AgentId, employee: AgentId, amount: f64) -> DomainResult {
        let effect = StateEffect::Financial(FinancialEffect::PayWages { employer, employee, amount });
        DomainResult::success(vec![effect])
    }

    fn execute_inject_liquidity(&self, state: &SimState) -> DomainResult {
        let recipients: Vec<AgentId> = state.agents.consumers.keys().cloned().collect();
        let amount_per_recipient = 1000.0;

        let effect = StateEffect::Financial(FinancialEffect::InjectLiquidity { recipients, amount_per_recipient });
        DomainResult::success(vec![effect])
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Banking",
        constructor: || Box::new(BankingDomain::new()),
    }
}
```

---

## `crates/domains/src/banking/mod.rs`

```rust
pub mod behavior;
pub mod domain;

pub use behavior::*;
pub use domain::*;

```

---

## `crates/domains/src/consumption/behavior.rs`

```rust
use sim_core::*;
use std::any::Any;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimpleConsumerDecisionModel {
    pub mpc: f64, // Marginal propensity to consume
}

impl Default for SimpleConsumerDecisionModel {
    fn default() -> Self {
        Self { mpc: 0.7 }
    }
}

#[typetag::serde]
impl DecisionModel for SimpleConsumerDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let consumer = match agent.downcast_ref::<Consumer>() {
            Some(c) => c,
            None => return vec![],
        };

        let mut intentions = Vec::new();

        // Handle employment first
        self.handle_employment(consumer, &mut intentions);

        // Calculate budget
        let fs = &state.financial_system;
        let weekly_income = consumer.income / 52.0;
        let liquid_assets = fs.get_liquid_assets(&consumer.id);
        let total_resources = weekly_income + liquid_assets;
        
        if total_resources < 1.0 {
            return intentions;
        }

        let budget = total_resources * self.mpc;
        let save_amount = total_resources - budget;

        // Make purchases
        self.make_purchases(consumer, budget, &mut intentions);

        // Handle savings
        if save_amount > 1.0 {
            intentions.push(SimIntention::DepositFunds {
                agent_id: consumer.id,
                bank: consumer.bank_id,
                amount: save_amount,
            });
        }

        intentions
    }
}

impl SimpleConsumerDecisionModel {
    fn handle_employment(&self, consumer: &Consumer, intentions: &mut Vec<SimIntention>) {
        if consumer.employed_by.is_none() {
            let expected_hourly_wage = match consumer.personality {
                PersonalityArchetype::Balanced => 25.0,
                PersonalityArchetype::Spender => 30.0,
                PersonalityArchetype::Saver => 20.0,
            };

            let application = JobApplication {
                application_id: Uuid::new_v4(),
                consumer_id: consumer.id,
                reservation_wage: expected_hourly_wage * 0.9,
                hours_desired: 40.0,
            };

            intentions.push(SimIntention::ApplyForJob {
                agent_id: consumer.id,
                market_id: LabourMarketId::GeneralLabour,
                application,
            });
        }
    }

    fn make_purchases(&self, consumer: &Consumer, budget: f64, intentions: &mut Vec<SimIntention>) {
        // Simple consumption basket
        let consumption_basket = [
            ("bread", 0.4, 3.0),
            ("petrol", 0.6, 4.0),
        ];

        for (good_slug, budget_share, _fallback_price) in consumption_basket {
            if let Some(good_id) = goods::CATALOGUE.get_good_id_by_slug(good_slug) {
                let allocation = budget * budget_share;
                
                if allocation > 0.01 {
                    intentions.push(SimIntention::SpendOnGood {
                        agent_id: consumer.id,
                        good_id,
                        max_notional: allocation,
                    });
                }
            }
        }
    }
}

// Advanced consumer model using CES utility
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CESConsumerDecisionModel {
    pub sigma: f64,                    // Elasticity of substitution
    pub weights: HashMap<GoodId, f64>, // Consumption weights
    pub mpc_base: f64,                 // Base marginal propensity to consume
}

impl Default for CESConsumerDecisionModel {
    fn default() -> Self {
        let mut weights = HashMap::new();
        if let Some(petrol_id) = goods::CATALOGUE.get_good_id_by_slug("petrol") {
            weights.insert(petrol_id, 0.2);
        }
        if let Some(bread_id) = goods::CATALOGUE.get_good_id_by_slug("bread") {
            weights.insert(bread_id, 0.5);
        }

        Self {
            sigma: 1.5,
            weights,
            mpc_base: 0.8,
        }
    }
}

#[typetag::serde]
impl DecisionModel for CESConsumerDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let consumer = match agent.downcast_ref::<Consumer>() {
            Some(c) => c,
            None => return vec![],
        };

        let mut intentions = Vec::new();

        self.handle_employment(consumer, &mut intentions);

        // Interest rate sensitive MPC
        let nominal_rate_bps = state.financial_system.central_bank.policy_rate_bps;
        let nominal_rate = bps_to_decimal(nominal_rate_bps);
        let expected_inflation = consumer.expectations.expected_inflation;
        let real_rate = nominal_rate - expected_inflation;

        let mpc_adjustment = (real_rate - 0.02).max(0.0) * 5.0;
        let mpc = (self.mpc_base - mpc_adjustment).max(0.1).min(0.95);

        // Budget calculation
        let fs = &state.financial_system;
        let weekly_income = consumer.income / 52.0;
        let liquid_assets = fs.get_liquid_assets(&consumer.id);
        let total_resources = weekly_income + liquid_assets;
        let budget = total_resources * mpc;
        let save_amount = total_resources - budget;

        if budget < 1.0 {
            self.handle_savings(consumer, save_amount, &mut intentions);
            return intentions;
        }

        // CES utility optimization
        let market_data = self.collect_market_data(state);
        if !market_data.is_empty() {
            self.optimize_ces_consumption(consumer, budget, &market_data, &mut intentions);
        }

        self.handle_savings(consumer, save_amount, &mut intentions);

        intentions
    }
}

impl CESConsumerDecisionModel {
    fn handle_employment(&self, consumer: &Consumer, intentions: &mut Vec<SimIntention>) {
        if consumer.employed_by.is_none() {
            let expected_hourly_wage = match consumer.personality {
                PersonalityArchetype::Balanced => 25.0,
                PersonalityArchetype::Spender => 30.0,
                PersonalityArchetype::Saver => 20.0,
            };

            let application = JobApplication {
                application_id: Uuid::new_v4(),
                consumer_id: consumer.id,
                reservation_wage: expected_hourly_wage * 0.9,
                hours_desired: 40.0,
            };

            intentions.push(SimIntention::ApplyForJob {
                agent_id: consumer.id,
                market_id: LabourMarketId::GeneralLabour,
                application,
            });
        }
    }

    fn collect_market_data(&self, state: &SimState) -> Vec<(GoodId, f64, f64)> {
        let mut market_data = Vec::new();

        for (good_id, weight) in &self.weights {
            if let Some(view) = state.market_view(&MarketId::Goods(*good_id)) {
                if let Some(price) = view.last_or_mid() {
                    if price > 1e-6 {
                        market_data.push((*good_id, price, *weight));
                    }
                }
            }
        }

        market_data
    }

    fn optimize_ces_consumption(
        &self,
        consumer: &Consumer,
        budget: f64,
        market_data: &[(GoodId, f64, f64)],
        intentions: &mut Vec<SimIntention>,
    ) {
        // CES demand system calculation
        let denominator: f64 = market_data.iter()
            .map(|(_, price, weight)| weight * price.powf(1.0 - self.sigma))
            .sum();

        if denominator <= 1e-9 {
            return;
        }

        for (good_id, price, weight) in market_data {
            let share = (weight * price.powf(1.0 - self.sigma)) / denominator;
            let notional = share * budget;

            if notional > 0.01 {
                intentions.push(SimIntention::SpendOnGood {
                    agent_id: consumer.id,
                    good_id: *good_id,
                    max_notional: notional,
                });
            }
        }
    }

    fn handle_savings(&self, consumer: &Consumer, save_amount: f64, intentions: &mut Vec<SimIntention>) {
        if save_amount > 1.0 {
            intentions.push(SimIntention::DepositFunds {
                agent_id: consumer.id,
                bank: consumer.bank_id,
                amount: save_amount,
            });
        }
    }
}
```

---

## `crates/domains/src/consumption/domain.rs`

```rust
use serde::{Deserialize, Serialize};
use sim_core::*;
use crate::{Any, inventory, Domain, DomainResult, DomainValidator, ResolutionContext, ResolutionResult, ResolutionPhase};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumptionDomain {}

impl ConsumptionDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for ConsumptionDomain {
    fn name(&self) -> &'static str { 
        "Consumption" 
    }

    fn resolve_intention(&self, intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            // Consumer spending gets resolved by trading domain's SpendOnGood logic
            // Employment applications handled by labour domain
            SimIntention::ApplyForJob { agent_id: _, market_id, application } => {
                vec![SimAction::Labour(LabourAction::ApplyForJob { 
                    market_id: market_id.clone(), 
                    application: application.clone() 
                })]
            },
            
            // Direct consumption of owned goods
            SimIntention::ConsumeGood { agent_id, good_id, quantity } => {
                vec![SimAction::Consumption(ConsumptionAction::Consume { 
                    agent_id: *agent_id, 
                    good_id: *good_id, 
                    amount: *quantity 
                })]
            },
            
            // Not a consumption intention (SpendOnGood is handled by trading domain)
            _ => return None,
        };
        
        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::ApplyForJob { .. } => Some(ResolutionPhase::Independent),
            SimIntention::ConsumeGood { .. } => Some(ResolutionPhase::Independent),
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let consumption_action = match action {
            SimAction::Consumption(action) => action,
            _ => return DomainResult::failure(vec!["Not a consumption action".to_string()]),
        };

        if let Err(error) = self.validate(consumption_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match consumption_action {
            ConsumptionAction::Purchase { agent_id, seller, good_id, amount } => {
                self.execute_purchase(*agent_id, *seller, *good_id, *amount, state)
            },
            ConsumptionAction::PurchaseAtBest { agent_id, good_id, max_notional } => {
                self.execute_purchase_at_best(*agent_id, *good_id, *max_notional, state)
            },
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                self.execute_consume(*agent_id, *good_id, *amount)
            },
            ConsumptionAction::NoAction { agent_id: _ } => {
                DomainResult::empty()
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ConsumptionDomain {
    fn validate(&self, action: &ConsumptionAction, state: &SimState) -> Result<(), String> {
        match action {
            ConsumptionAction::Purchase { agent_id: buyer, seller, good_id, amount } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::agent_exists(*buyer, state)?;
                DomainValidator::agent_exists(*seller, state)?;

                // Check seller has inventory
                let seller_bs = state.financial_system.balance_sheets.get(seller)
                    .ok_or("Seller not found")?;
                let available_inventory = seller_bs.get_inventory()
                    .and_then(|inv| inv.get(good_id))
                    .map_or(0.0, |item| item.quantity);
                    
                if available_inventory < *amount {
                    return Err(format!(
                        "Seller has insufficient inventory: needs {:.2}, has {:.2}",
                        amount, available_inventory
                    ));
                }

                // Check buyer has funds (using market price estimate)
                let price = state.financial_system.exchange.goods_market(good_id)
                    .and_then(|m| m.best_ask())
                    .map_or(1.0, |ask| ask.price);
                let total_cost = amount * price;
                let available_funds = state.financial_system.get_liquid_assets(buyer);
                
                if available_funds < total_cost {
                    return Err(format!(
                        "Buyer has insufficient funds: needs ${:.2}, has ${:.2}", 
                        total_cost, available_funds
                    ));
                }

                Ok(())
            },
            
            ConsumptionAction::PurchaseAtBest { agent_id, good_id: _, max_notional } => {
                DomainValidator::positive_amount(*max_notional)?;
                DomainValidator::agent_exists(*agent_id, state)?;
                
                let available_funds = state.financial_system.get_liquid_assets(agent_id);
                if available_funds < *max_notional {
                    return Err(format!(
                        "Buyer has insufficient funds for max notional: needs ${:.2}, has ${:.2}", 
                        max_notional, available_funds
                    ));
                }
                Ok(())
            },
            
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::agent_exists(*agent_id, state)?;

                let bs = state.financial_system.balance_sheets.get(agent_id)
                    .ok_or(format!("Agent {:?} not found", agent_id))?;
                let available = bs.get_inventory()
                    .and_then(|inv| inv.get(good_id))
                    .map_or(0.0, |item| item.quantity);

                if available < *amount {
                    return Err(format!(
                        "Agent has insufficient goods to consume: needs {:.2}, has {:.2}", 
                        amount, available
                    ));
                }
                Ok(())
            },
            
            ConsumptionAction::NoAction { .. } => Ok(()),
        }
    }

    fn execute_purchase(&self, buyer: AgentId, seller: AgentId, good_id: GoodId, amount: f64, state: &SimState) -> DomainResult {
        let price = state.financial_system.exchange.goods_market(&good_id)
            .and_then(|m| m.best_ask())
            .map_or(1.0, |ask| ask.price);
        let total_cost = amount * price;

        let effects = vec![
            StateEffect::Financial(FinancialEffect::RecordTransaction(Transaction {
                id: uuid::Uuid::new_v4(),
                date: state.ticknum,
                qty: total_cost,
                from: buyer,
                to: seller,
                tx_type: TransactionType::Transfer { from: buyer, to: seller, amount: total_cost },
                instrument_id: None,
            })),
            StateEffect::Inventory(InventoryEffect::RemoveInventory {
                owner: seller,
                good_id,
                quantity: amount,
            }),
            StateEffect::Inventory(InventoryEffect::AddInventory {
                owner: buyer,
                good_id,
                quantity: amount,
                unit_cost: price,
            }),
        ];

        DomainResult::success(effects)
    }

    fn execute_purchase_at_best(&self, buyer: AgentId, good_id: GoodId, max_notional: f64, state: &SimState) -> DomainResult {
        let order = Order::Bid(Bid {
            agent_id: buyer,
            quantity: max_notional, // Quantity here is max notional
            price: state.financial_system.exchange.goods_market(&good_id).unwrap().best_ask().unwrap().price, // Price is ignored for market orders
        });
        let effects = vec![
            StateEffect::Market(MarketEffect::PlaceOrderInBook {
                market_id: MarketId::Goods(good_id),
                order,
            }),
        ];

        DomainResult::success(effects)
    }

    fn execute_consume(&self, agent_id: AgentId, good_id: GoodId, amount: f64) -> DomainResult {
        let effects = vec![
            StateEffect::Inventory(InventoryEffect::RemoveInventory { 
                owner: agent_id, 
                good_id, 
                quantity: amount 
            })
        ];

        DomainResult::success(effects)
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Consumption",
        constructor: || Box::new(ConsumptionDomain::new()),
    }
}

```

---

## `crates/domains/src/consumption/mod.rs`

```rust
//! # Consumption Domain Crate
//!
//! This crate contains all logic related to consumer agents. It governs how consumers
//! make decisions about what to buy and handles the mechanics of purchasing and
//! consuming goods.
//!
//! ## Crate Structure and Purpose
//!
//! Following the standard domain pattern, this crate separates action execution from
//! agent behavior.
//!
//! - **`domain.rs`**: Implements the `ConsumptionDomain` struct. This service validates
//!   and executes `ConsumptionAction`s, such as `Purchase` or `Consume`. It ensures
//!   a consumer has sufficient funds for a purchase and enough inventory to consume.
//!   Executing these actions generates the appropriate `StateEffect`s, like removing
//!   inventory or creating payment transfers.
//!
//! - **`behavior.rs`**: Implements decision models for consumer agents. This includes
//!   `BasicConsumerDecisionModel`, which uses simple heuristics to decide on purchases,
//!   and `ParametricMPC`, which uses a marginal propensity to consume (MPC) to determine
//    spending levels based on income.
//!
//! ## Key Components
//!
//! - **`ConsumptionDomain`**: The handler for executing consumer actions.
//! - **`BasicConsumerDecisionModel`**: The default "AI" for consumer agents.
//! - **`ConsumptionAction`**: The set of actions available to consumers (defined in `sim_actions`).
pub mod behavior;
pub mod domain;

pub use behavior::*;
pub use domain::*;
```

---

## `crates/domains/src/fiscal/behaviour.rs`

```rust
use sim_core::*;
use std::any::Any;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use chrono::Datelike;

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct BasicGovernmentDecisionModel;

#[typetag::serde]
impl DecisionModel for BasicGovernmentDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let government = match agent.downcast_ref::<Government>() {
            Some(g) => g,
            None => return vec![],
        };

        let mut intentions = Vec::new();
        
        if state.current_date.ordinal() % 30 == 0 {
            self.collect_taxes(government, state, &mut intentions);
        }
        
        self.handle_funding(government, state, &mut intentions);
        
        intentions
    }
}

impl BasicGovernmentDecisionModel {
    fn collect_taxes(&self, government: &Government, state: &SimState, intentions: &mut Vec<SimIntention>) {
        let tax_rate = government.tax_rates.income_tax;

        for consumer in state.agents.consumers.values() {
            let monthly_tax_liability = (consumer.income / 12.0) * tax_rate;
            if monthly_tax_liability > 0.0 {
                intentions.push(SimIntention::CollectTaxes {
                    government_id: government.id,
                    target: consumer.id,
                    amount: monthly_tax_liability,
                });
            }
        }
    }

    fn handle_funding(&self, government: &Government, state: &SimState, intentions: &mut Vec<SimIntention>) {
        let fs = &state.financial_system;
        let current_balance = fs.get_liquid_assets(&government.id);
        let monthly_spending_target = 1_000_000.0 / 12.0;
        
        if current_balance < monthly_spending_target {
            let deficit = monthly_spending_target - current_balance;
            
            let issue_distribution = [
                (Tenor::T2Y, 0.15),
                (Tenor::T5Y, 0.25),
                (Tenor::T10Y, 0.40),
                (Tenor::T30Y, 0.20),
            ];

            let coupon_rate = fs.central_bank.policy_rate_bps;
            
            for (tenor, percentage) in issue_distribution {
                let amount_to_raise = deficit * percentage;
                
                if amount_to_raise > 0.0 {
                    intentions.push(SimIntention::IssueDebtToRaise {
                        government_id: government.id,
                        tenor,
                        amount_to_raise,
                        coupon_rate,
                    });
                }
            }
        }
    }
}
```

---

## `crates/domains/src/fiscal/domain.rs`

```rust
use serde::{Deserialize, Serialize};
use sim_core::*;
use crate::{Any, inventory, Domain, DomainResult, ResolutionContext, ResolutionResult, ResolutionPhase};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FiscalDomain {}

impl FiscalDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for FiscalDomain {
    fn name(&self) -> &'static str { 
        "Fiscal" 
    }

    fn resolve_intention(&self, intention: &SimIntention, context: &ResolutionContext) -> Option<ResolutionResult> {
        match intention {
            SimIntention::IssueDebtToRaise { government_id, tenor, amount_to_raise, coupon_rate } => {
                Some(self.resolve_debt_issuance(*government_id, *tenor, *amount_to_raise, *coupon_rate, context))
            },
            _ => None, // Not a fiscal intention
        }
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::IssueDebtToRaise { .. } => Some(ResolutionPhase::Dependent),
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let fiscal_action = match action {
            SimAction::Fiscal(action) => action,
            _ => return DomainResult::failure(vec!["Not a fiscal action".to_string()]),
        };

        if let Err(error) = self.validate(fiscal_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match fiscal_action {
            FiscalAction::IssueDebt { government_id, tenor, face_value, quantity, coupon_rate } => {
                self.execute_debt_issuance(*government_id, *tenor, *face_value, *quantity, *coupon_rate, state)
            },
            FiscalAction::ChangeTaxRate { .. } => {
                DomainResult::empty()
            },
            FiscalAction::SetSpendingTarget { .. } => {
                DomainResult::empty()
            },
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl FiscalDomain {
    fn resolve_debt_issuance(
        &self,
        government_id: AgentId,
        tenor: Tenor,
        amount_to_raise: f64,
        coupon_rate: BasisPoints,
        context: &ResolutionContext
    ) -> ResolutionResult {
        const FACE_VALUE: f64 = 1000.0;
        
        let fs = &context.state.financial_system;
        let market_id_enum = FinancialMarketId::Treasury { tenor };
        let market_id = MarketId::Financial(market_id_enum.clone());

        let discovered_price = self.discover_market_price(&market_id_enum, &market_id, fs, context);

        match discovered_price {
            Some(price) => {
                let quantity = (amount_to_raise / price).ceil() as u32;
                
                if quantity > 0 {
                    let actions = vec![SimAction::Fiscal(FiscalAction::IssueDebt {
                        government_id,
                        tenor,
                        face_value: FACE_VALUE,
                        quantity,
                        coupon_rate,
                    })];
                    
                    ResolutionResult::success(actions)
                } else {
                    ResolutionResult::failure(vec!["Cannot determine valid quantity for debt issuance".to_string()])
                }
            },
            None => {
                ResolutionResult::failure(vec![
                    format!("No market price available for {} treasury bonds", tenor)
                ])
            }
        }
    }

    fn discover_market_price(
        &self,
        market_id_enum: &FinancialMarketId,
        market_id: &MarketId,
        fs: &FinancialSystem,
        context: &ResolutionContext
    ) -> Option<f64> {
        if let Some(market) = fs.exchange.financial_market(market_id_enum) {
            if let Some(best_bid) = market.order_book.best_bid() {
                return Some(best_bid.price);
            }
        }

        if let Some(market_view) = context.state.market_view(market_id) {
            if let Some(last_price) = market_view.last_or_mid() {
                return Some(last_price);
            }
        }

        if let Some(theoretical_price) = self.calculate_theoretical_price(market_id_enum, fs) {
            return Some(theoretical_price);
        }

        const FACE_VALUE: f64 = 1000.0;
        Some(FACE_VALUE * 0.98) // 2% discount
    }

    fn calculate_theoretical_price(&self, market_id: &FinancialMarketId, fs: &FinancialSystem) -> Option<f64> {
        if let FinancialMarketId::Treasury { tenor } = market_id {
            const FACE_VALUE: f64 = 1000.0;
            const FREQUENCY: usize = 2;
            
            let policy_rate_bps = fs.central_bank.policy_rate_bps;
            let term_premium = match tenor {
                Tenor::T2Y => 20.0,
                Tenor::T5Y => 40.0,
                Tenor::T10Y => 60.0,
                Tenor::T30Y => 80.0,
            };
            
            let estimated_yield_bps = policy_rate_bps + term_premium;
            let coupon_rate_bps = policy_rate_bps; // Assume benchmark coupon
            
            let price = pricing::bond_price(
                FACE_VALUE,
                bps_to_decimal(coupon_rate_bps),
                bps_to_decimal(estimated_yield_bps),
                tenor.to_years(),
                FREQUENCY
            );
            
            Some(price)
        } else {
            None
        }
    }
}

impl FiscalDomain {
    fn validate(&self, action: &FiscalAction, state: &SimState) -> Result<(), String> {
        match action {
            FiscalAction::IssueDebt { government_id: _, quantity, .. } => {
                if *quantity == 0 {
                    return Err("Cannot issue zero bonds".to_string());
                }
                if let Some(g) = state.financial_system.get_government() {
                    if g.tax_rates.income_tax <= 0.0 {
                        return Err("Government must have a positive income tax rate to issue debt".to_string());
                    }
                } else {
                    return Err("Government agent not found".to_string());
                }
                
                Ok(())
            },
            FiscalAction::ChangeTaxRate { .. } |
            FiscalAction::SetSpendingTarget { .. } => Ok(()),
        }
    }

    fn execute_debt_issuance(
        &self,
        government_id: AgentId,
        tenor: Tenor,
        face_value: f64,
        quantity: u32,
        _coupon_rate: BasisPoints,
        state: &SimState
    ) -> DomainResult {
        let theoretical_price = self.calculate_theoretical_price(
            &FinancialMarketId::Treasury { tenor }, 
            &state.financial_system
        ).unwrap_or(face_value * 0.98);

        let ask = Ask {
            agent_id: government_id,
            quantity: quantity as f64,
            price: theoretical_price,
        };

        let market_effect = StateEffect::Market(MarketEffect::PlaceOrderInBook {
            market_id: MarketId::Financial(FinancialMarketId::Treasury { tenor }),
            order: Order::Ask(ask),
        });

        DomainResult::success(vec![market_effect])
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Fiscal",
        constructor: || Box::new(FiscalDomain::new()),
    }
}

```

---

## `crates/domains/src/fiscal/mod.rs`

```rust
//! # Fiscal Domain Crate
//!
//! This crate manages all logic related to government agents and fiscal policy. It
//! is responsible for executing fiscal actions and defining the decision-making
//! processes of the government entity.
//!
//! ## Crate Structure and Purpose
//!
//! The crate is organized into the standard domain/behavior modules:
//!
//! - **`domain.rs`**: Contains the `FiscalDomain` struct. This module handles the
//!   execution of `FiscalAction`s, such as collecting taxes or distributing
//!   transfer payments. It translates these high-level policy actions into concrete
//!   `StateEffect`s that modify agent balance sheets.
//!
//! - **`behaviour.rs`**: Contains the `BasicGovernmentDecisionModel`. This model implements
//!   the `DecisionModel` trait and defines how the government agent behaves. For example,
//!   it may decide to issue new bonds to fund a deficit based on its defined `FiscalPolicy`
//!   (e.g., `Expansionary`, `Contractionary`).
//!
//! ## Key Components
//!
//! - **`FiscalDomain`**: The service for executing government-level actions.
//! - **`BasicGovernmentDecisionModel`**: The logic controller for the government agent.
//! - **`FiscalAction`**: The set of actions available to the government (defined in `sim_actions`).
//! - **`FiscalPolicy`**: An enum from `sim_types` that guides the government's decision-making.
pub mod behaviour;
pub mod domain;

pub use behaviour::*;
pub use domain::*;
```

---

## `crates/domains/src/labour/mod.rs`

```rust
use serde::{Deserialize, Serialize};
use sim_core::*;
use crate::{Any, inventory, Domain, DomainResult, ResolutionContext, ResolutionResult, ResolutionPhase};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LabourDomain {}

impl LabourDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for LabourDomain {
    fn name(&self) -> &'static str { 
        "Labour" 
    }

    fn resolve_intention(&self, intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::ApplyForJob { agent_id: _, market_id, application } => {
                vec![SimAction::Labour(LabourAction::ApplyForJob { 
                    market_id: market_id.clone(), 
                    application: application.clone() 
                })]
            },
            

            
            _ => return None,
        };
        
        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::ApplyForJob { .. } => Some(ResolutionPhase::Independent),
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, _state: &SimState) -> DomainResult {
        let labour_action = match action {
            SimAction::Labour(action) => action,
            _ => return DomainResult::failure(vec!["Not a labour action".to_string()]),
        };

        match labour_action {
            LabourAction::ApplyForJob { market_id, application } => {
                self.execute_apply(market_id.clone(), application.clone())
            },
            LabourAction::PostJobOffer { market_id, offer } => {
                self.execute_post_offer(market_id.clone(), offer.clone())
            },
            LabourAction::Fire { firm_id, employee_id } => {
                self.execute_fire(*firm_id, *employee_id)
            },
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl LabourDomain {
    fn execute_apply(&self, market_id: LabourMarketId, application: JobApplication) -> DomainResult {
        let effect = StateEffect::Market(MarketEffect::UpdateLabourMarket {
            market_id,
            update: LabourMarketUpdate::AddApplication(application),
        });
        
        DomainResult::success(vec![effect])
    }

    fn execute_post_offer(&self, market_id: LabourMarketId, offer: JobOffer) -> DomainResult {
        let effect = StateEffect::Market(MarketEffect::UpdateLabourMarket {
            market_id,
            update: LabourMarketUpdate::AddOffer(offer),
        });
        
        DomainResult::success(vec![effect])
    }

    fn execute_fire(&self, firm_id: AgentId, employee_id: AgentId) -> DomainResult {
        let effect = StateEffect::Agent(AgentEffect::TerminateEmployment {
            firm_id,
            consumer_id: employee_id,
        });
        
        DomainResult::success(vec![effect])
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Labour",
        constructor: || Box::new(LabourDomain::new()),
    }
}
```

---

## `crates/domains/src/lib.rs`

```rust
pub use std::any::Any;
use sim_core::*;
extern crate inventory;

pub trait Domain: Send + Sync {
    fn name(&self) -> &'static str;
    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult;
    
    fn resolve_intention(&self, _intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        None
    }
    
    fn resolution_phase(&self, _intention: &SimIntention) -> Option<ResolutionPhase> {
        None
    }
    
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug)]
pub struct DomainResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl DomainResult {
    pub fn success(effects: Vec<StateEffect>) -> Self {
        Self { success: true, effects, errors: vec![] }
    }
    
    pub fn failure(errors: Vec<String>) -> Self {
        Self { success: false, effects: vec![], errors }
    }
    
    pub fn empty() -> Self {
        Self { success: true, effects: vec![], errors: vec![] }
    }
}

#[derive(Debug, Clone)]
pub struct ResolutionContext<'a> {
    pub state: &'a SimState,
    pub current_tick: u32,
}

#[derive(Debug)]
pub struct ResolutionResult {
    pub actions: Vec<SimAction>,
    pub success: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResolutionPhase {
    Independent = 0,
    Market = 1,
    Dependent = 2,
}

impl ResolutionResult {
    pub fn success(actions: Vec<SimAction>) -> Self {
        Self { actions, success: true, errors: vec![] }
    }
    
    pub fn failure(errors: Vec<String>) -> Self {
        Self { actions: vec![], success: false, errors }
    }
    
    pub fn not_handled() -> Self {
        Self { actions: vec![], success: true, errors: vec![] }
    }
}

pub struct DomainValidator;

impl DomainValidator {
    pub fn positive_amount(amount: f64) -> Result<(), String> {
        if amount <= 0.0 {
            Err("Amount must be positive".to_string())
        } else {
            Ok(())
        }
    }
    
    pub fn non_negative_amount(amount: f64) -> Result<(), String> {
        if amount < 0.0 {
            Err("Amount cannot be negative".to_string())
        } else {
            Ok(())
        }
    }
    
    pub fn agent_exists(agent_id: AgentId, state: &SimState) -> Result<(), String> {
        if state.financial_system.balance_sheets.contains_key(&agent_id) {
            Ok(())
        } else {
            Err(format!("Agent {} does not exist", agent_id.0))
        }
    }
    
    pub fn bank_exists(bank_id: AgentId, state: &SimState) -> Result<(), String> {
        if state.agents.banks.contains_key(&bank_id) {
            Ok(())
        } else {
            Err("Target is not a valid commercial bank".to_string())
        }
    }
    
    pub fn firm_exists(firm_id: AgentId, state: &SimState) -> Result<(), String> {
        if state.agents.firms.contains_key(&firm_id) {
            Ok(())
        } else {
            Err("Target is not a valid firm".to_string())
        }
    }
    
    pub fn positive_integer(value: u32, field_name: &str) -> Result<(), String> {
        if value == 0 {
            Err(format!("{} must be positive", field_name))
        } else {
            Ok(())
        }
    }
    
    pub fn percentage(value: f64) -> Result<(), String> {
        if value < 0.0 || value > 1.0 {
            Err("Value must be between 0.0 and 1.0".to_string())
        } else {
            Ok(())
        }
    }
}

pub struct DomainRegistration {
    pub name: &'static str,
    pub constructor: fn() -> Box<dyn Domain>,
}

inventory::collect!(DomainRegistration);

pub mod banking;
pub mod consumption;
pub mod fiscal;
pub mod labour;
pub mod production;
pub mod settlement;
pub mod trading;

pub mod prelude {
    pub use crate::{
        Domain, DomainResult, DomainValidator, DomainRegistration,
        ResolutionContext, ResolutionResult, ResolutionPhase,
    };
    
    pub use crate::banking::{BankingDomain, BasicBankDecisionModel};
    pub use crate::consumption::{ConsumptionDomain, SimpleConsumerDecisionModel, CESConsumerDecisionModel};
    pub use crate::fiscal::{FiscalDomain, BasicGovernmentDecisionModel};
    pub use crate::labour::LabourDomain;
    pub use crate::production::{ProductionDomain, SimpleFirmDecisionModel};
    pub use crate::settlement::SettlementDomain;
    pub use crate::trading::TradingDomain;
    
    pub use sim_core::*;
    pub use std::any::Any;
}
```

---

## `crates/domains/src/prelude/mod.rs`

```rust
pub use crate::{Domain, DomainResult, DomainValidator, DomainRegistration};

// Resolution system
pub use crate::{ResolutionContext, ResolutionResult, ResolutionPhase};

// Domain implementations
pub use crate::banking::{BankingDomain, BasicBankDecisionModel};
pub use crate::consumption::{ConsumptionDomain, SimpleConsumerDecisionModel, CESConsumerDecisionModel};
pub use crate::fiscal::{FiscalDomain, BasicGovernmentDecisionModel};
pub use crate::labour::{LabourDomain};
pub use crate::production::{ProductionDomain, SimpleFirmDecisionModel};
pub use crate::settlement::{SettlementDomain};
pub use crate::trading::{TradingDomain};

// Essential sim_core re-exports
pub use sim_core::*;
pub use std::any::Any;

// External dependencies commonly used in domains
extern crate inventory;
```

---

## `crates/domains/src/production/behavior.rs`

```rust
use sim_core::*;
use std::any::Any;
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimpleFirmDecisionModel {
    pub target_markup: f64,
    pub base_wage: f64,
    pub target_employees: usize,
}

impl Default for SimpleFirmDecisionModel {
    fn default() -> Self {
        Self {
            target_markup: 1.25, // 25% markup
            base_wage: 25.0,     // Base hourly wage
            target_employees: 3, // Target workforce size
        }
    }
}

#[typetag::serde]
impl DecisionModel for SimpleFirmDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let firm = match agent.downcast_ref::<Firm>() {
            Some(f) => f,
            None => return vec![],
        };
        
        let mut intentions = Vec::new();

        self.handle_hiring(firm, &mut intentions);
        self.handle_production(firm, state, &mut intentions);
        self.handle_wages(firm, &mut intentions);
        self.handle_sales(firm, state, &mut intentions);
        self.handle_input_purchases(firm, state, &mut intentions);

        intentions
    }
}

impl SimpleFirmDecisionModel {
    fn handle_hiring(&self, firm: &Firm, intentions: &mut Vec<SimIntention>) {
        let current_employees = firm.employees.len();
        if current_employees < self.target_employees {
            let positions_to_fill = self.target_employees - current_employees;
            intentions.push(SimIntention::HireWorkers { 
                agent_id: firm.id, 
                count: positions_to_fill as u32,
                wage_rate: self.base_wage,
            });
        }
    }

    fn handle_production(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        if firm.employees.is_empty() {
            return; // Can't produce without workers
        }

        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = state.financial_system.goods.get_recipe(&recipe_id) {
                if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id)
                    .and_then(|bs| bs.get_inventory()) {
                    
                    let can_produce = recipe.inputs.iter().all(|input| {
                        inventory.get(&input.good_id)
                            .map_or(false, |item| item.quantity >= input.quantity)
                    });

                    if can_produce {
                        intentions.push(SimIntention::Produce { 
                            agent_id: firm.id, 
                            recipe_id, 
                            batches: 1,
                        });
                    }
                }
            }
        }
    }

    fn handle_wages(&self, firm: &Firm, intentions: &mut Vec<SimIntention>) {
        for (employee_id, contract) in &firm.employees {
            let weekly_wage = contract.wage_rate * contract.hours;
            if weekly_wage > 0.0 {
                intentions.push(SimIntention::PayWages {
                    employer: firm.id,
                    employee: *employee_id,
                    amount: weekly_wage,
                });
            }
        }
    }

    fn handle_sales(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id)
            .and_then(|bs| bs.get_inventory()) {
            
            for (good_id, item) in inventory {
                if item.quantity > 0.1 {
                    intentions.push(SimIntention::SellInventory {
                        agent_id: firm.id,
                        good_id: *good_id,
                        quantity: item.quantity,
                        desired_markup: self.target_markup,
                    });
                }
            }
        }
    }

    fn handle_input_purchases(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = state.financial_system.goods.get_recipe(&recipe_id) {
                if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id)
                    .and_then(|bs| bs.get_inventory()) {
                    
                    for input in &recipe.inputs {
                        let current_qty = inventory.get(&input.good_id)
                            .map_or(0.0, |item| item.quantity);
                        
                        let target_qty = input.quantity * 2.0; // Keep 2 batches worth
                        if current_qty < target_qty {
                            let buy_qty = target_qty - current_qty;
                            let max_price = 100.0; // Willing to pay up to $100 per unit
                            
                            intentions.push(SimIntention::PurchaseInputs {
                                agent_id: firm.id,
                                good_id: input.good_id,
                                quantity: buy_qty,
                                max_price,
                            });
                        }
                    }
                }
            }
        }
    }
}
```

---

## `crates/domains/src/production/domain.rs`

```rust
use serde::{Deserialize, Serialize};
use sim_core::*;
use crate::{Any, inventory, Domain, DomainResult, DomainValidator, ResolutionContext, ResolutionResult, ResolutionPhase};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionDomain {}

impl ProductionDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for ProductionDomain {
    fn name(&self) -> &'static str { 
        "Production" 
    }

    fn resolve_intention(&self, intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::Produce { agent_id, recipe_id, batches } => {
                vec![SimAction::Production(ProductionAction::Produce { 
                    agent_id: *agent_id, recipe_id: *recipe_id, batches: *batches 
                })]
            },
            
            SimIntention::HireWorkers { agent_id, count, wage_rate } => {
                vec![SimAction::Labour(LabourAction::PostJobOffer { 
                    market_id: LabourMarketId::GeneralLabour, 
                    offer: JobOffer {
                        offer_id: Uuid::new_v4(), 
                        firm_id: *agent_id,
                        quantity: *count,
                        wage_rate: *wage_rate,
                        hours_required: 40.0,
                    } 
                })]
            },
            
            _ => return None,
        };
        
        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::Produce { .. } => Some(ResolutionPhase::Independent),
            SimIntention::HireWorkers { .. } => Some(ResolutionPhase::Independent),
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let production_action = match action {
            SimAction::Production(action) => action,
            _ => return DomainResult::failure(vec!["Not a production action".to_string()]),
        };

        if let Err(error) = self.validate(production_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match production_action {
            ProductionAction::Hire { agent_id, count } => {
                self.execute_hire(*agent_id, *count)
            },
            ProductionAction::Produce { agent_id, recipe_id, batches } => {
                self.execute_produce(*agent_id, *recipe_id, *batches, state)
            },
            ProductionAction::Fire { agent_id, employee_id } => {
                self.execute_fire(*agent_id, *employee_id)
            },
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ProductionDomain {
    fn validate(&self, action: &ProductionAction, state: &SimState) -> Result<(), String> {
        match action {
            ProductionAction::Hire { agent_id, count } => {
                if *count == 0 {
                    Err("Cannot hire zero workers".to_string())
                } else {
                    DomainValidator::firm_exists(*agent_id, state)
                }
            },
            ProductionAction::Produce { agent_id, recipe_id, batches } => {
                if *batches == 0 {
                    return Err("Cannot produce zero batches".to_string());
                }
                
                DomainValidator::firm_exists(*agent_id, state)?;
                
                if !state.financial_system.goods.recipes.contains_key(recipe_id) {
                    return Err("Recipe not found".to_string());
                }
                
                if let Some(recipe) = state.financial_system.goods.recipes.get(recipe_id) {
                    if let Some(inventory) = state.financial_system.get_bs_by_id(agent_id)
                        .and_then(|bs| bs.get_inventory()) {
                        
                        for input in &recipe.inputs {
                            let available = inventory.get(&input.good_id)
                                .map_or(0.0, |item| item.quantity);
                            let required = input.quantity * (*batches as f64);
                            
                            if available < required {
                                return Err(format!(
                                    "Insufficient {} for production: need {:.2}, have {:.2}",
                                    input.good_id, required, available
                                ));
                            }
                        }
                    } else {
                        return Err("Firm has no inventory".to_string());
                    }
                }
                
                Ok(())
            },
            ProductionAction::Fire { agent_id, employee_id } => {
                if let Some(firm) = state.agents.firms.get(agent_id) {
                    if firm.employees.iter().any(|(id, _)| id == employee_id) {
                        Ok(())
                    } else {
                        Err("Employee not found in firm's employee list".to_string())
                    }
                } else {
                    Err("Firm not found".to_string())
                }
            }
        }
    }

    fn execute_hire(&self, agent_id: AgentId, count: u32) -> DomainResult {
        let effects = vec![
            StateEffect::Market(MarketEffect::UpdateLabourMarket {
                market_id: LabourMarketId::GeneralLabour,
                update: LabourMarketUpdate::AddOffer(JobOffer {
                    offer_id: Uuid::new_v4(),
                    firm_id: agent_id,
                    quantity: count,
                    wage_rate: 25.0, // Base wage - should come from firm's strategy
                    hours_required: 40.0,
                }),
            })
        ];
        
        DomainResult::success(effects)
    }

    fn execute_produce(&self, agent_id: AgentId, recipe_id: RecipeId, batches: u32, state: &SimState) -> DomainResult {
        if let Some(recipe) = state.financial_system.goods.recipes.get(&recipe_id) {
            let mut effects = Vec::new();
            
            for input in &recipe.inputs {
                effects.push(StateEffect::Inventory(InventoryEffect::RemoveInventory {
                    owner: agent_id,
                    good_id: input.good_id,
                    quantity: input.quantity * batches as f64,
                }));
            }
            
            for output in &recipe.outputs {
                let input_cost: f64 = recipe.inputs.iter()
                    .map(|input| {
                        state.financial_system.get_bs_by_id(&agent_id)
                            .and_then(|bs| bs.get_inventory())
                            .and_then(|inv| inv.get(&input.good_id))
                            .map_or(1.0, |item| item.unit_cost) * input.quantity
                    })
                    .sum();
                
                let labor_cost = recipe.labour_hours * 25.0; // Base wage rate
                let total_output_quantity: f64 = recipe.outputs.iter().map(|o| o.quantity).sum();
                let unit_cost = (input_cost + labor_cost) / total_output_quantity;
                
                effects.push(StateEffect::Inventory(InventoryEffect::AddInventory {
                    owner: agent_id,
                    good_id: output.good_id,
                    quantity: output.quantity * batches as f64,
                    unit_cost,
                }));
            }
            
            DomainResult::success(effects)
        } else {
            DomainResult::failure(vec!["Recipe not found".to_string()])
        }
    }

    fn execute_fire(&self, firm_id: AgentId, employee_id: AgentId) -> DomainResult {
        let effect = StateEffect::Agent(AgentEffect::TerminateEmployment {
            firm_id,
            consumer_id: employee_id,
        });
        
        DomainResult::success(vec![effect])
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Production",
        constructor: || Box::new(ProductionDomain::new()),
    }
}
```

---

## `crates/domains/src/production/mod.rs`

```rust
//! # Production Domain Crate
//!
//! This crate encapsulates all logic related to producer agents (firms). It governs
//! how firms make decisions about production levels, hiring, and resource management,
//! and it handles the execution of these production-related actions.
//!
//! ## Crate Structure and Purpose
//!
//! The crate is divided into action execution and agent behavior modules:
//!
//! - **`domain.rs`**: Implements the `ProductionDomain` struct. This service validates
//!   and executes `ProductionAction`s, such as `Produce` and `Hire`. For a `Produce`
//!   action, it validates that the firm has the necessary input goods and labour, as
//!   defined by its `ProductionRecipe`. If valid, it generates `StateEffect`s to consume
//!   the inputs and add the finished product to the firm's inventory.
//!
//! - **`behavior.rs`**: Implements the `BasicFirmDecisionModel`. This is the "AI" for
//!   firm agents. It analyzes market conditions and its own inventory levels to decide
//!   whether to increase production, hire more employees, or purchase more raw materials.
//!
//! ## Key Components
//!
//! - **`ProductionDomain`**: The handler for executing firm-specific actions.
//! - **`BasicFirmDecisionModel`**: The default logic controller for firm agents.
//! - **`ProductionAction`**: The set of actions available to firms (defined in `sim_actions`).
//! - **`ProductionRecipe`**: A data structure from `sim_types` that defines the inputs,
//!   outputs, and labour required for a production process.
pub mod behavior;
pub mod domain;
pub use domain::*;
pub use behavior::*;

```

---

## `crates/domains/src/settlement/domain.rs`

```rust
use crate::{Any, Domain, DomainResult, inventory};
use serde::{Deserialize, Serialize};
use sim_core::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettlementDomain {}

impl SettlementDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for SettlementDomain {
    fn name(&self) -> &'static str {
        "Settlement"
    }


    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let settlement_action = match action {
            SimAction::Settlement(action) => action,
            _ => return DomainResult::failure(vec!["Not a settlement action".to_string()]),
        };

        if let Err(error) = self.validate(settlement_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match settlement_action {
            SettlementAction::AccrueInterest { instrument_id } => self.execute_accrue_interest(instrument_id, state),
            SettlementAction::PayInterest { instrument_id } => self.execute_pay_interest(instrument_id, state),
            SettlementAction::ProcessCouponPayment { instrument_id } => {
                self.execute_process_coupon_payment(instrument_id, state)
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl SettlementDomain {
    fn validate(&self, action: &SettlementAction, state: &SimState) -> Result<(), String> {
        match action {
            SettlementAction::AccrueInterest { instrument_id }
            | SettlementAction::PayInterest { instrument_id }
            | SettlementAction::ProcessCouponPayment { instrument_id } => {
                if !state.financial_system.instruments.contains_key(instrument_id) {
                    Err(format!("Instrument {} not found", instrument_id.0))
                } else {
                    Ok(())
                }
            }
        }
    }

    fn execute_accrue_interest(&self, instrument_id: &InstrumentId, state: &SimState) -> DomainResult {
        if let Some(instrument) = state.financial_system.instruments.get(instrument_id) {
            let daily_accrual = self.calculate_daily_interest_accrual(instrument, state.current_date);

            if daily_accrual > 1e-6 {
                let effect = StateEffect::Financial(FinancialEffect::AccrueInterest {
                    instrument_id: *instrument_id,
                    accrued_amount: daily_accrual,
                    accrual_date: state.current_date,
                });
                DomainResult::success(vec![effect])
            } else {
                DomainResult::empty()
            }
        } else {
            DomainResult::failure(vec!["Instrument not found".to_string()])
        }
    }

    fn execute_pay_interest(&self, instrument_id: &InstrumentId, state: &SimState) -> DomainResult {
        if let Some(instrument) = state.financial_system.instruments.get(instrument_id) {
            let interest_amount = instrument.accrued_interest;

            if interest_amount <= 1e-6 {
                return DomainResult::empty();
            }

            let payment_effects =
                self.create_payment_effects(instrument.debtor, instrument.creditor, interest_amount, state);

            let mut effects = payment_effects;
            effects
                .push(StateEffect::Financial(FinancialEffect::ResetAccruedInterest { instrument_id: *instrument_id }));

            DomainResult::success(effects)
        } else {
            DomainResult::failure(vec!["Instrument not found".to_string()])
        }
    }

    fn execute_process_coupon_payment(&self, instrument_id: &InstrumentId, state: &SimState) -> DomainResult {
        if let Some(instrument) = state.financial_system.instruments.get(instrument_id) {
            if let Some(payment_amount) = self.get_coupon_payment_amount(instrument) {
                if payment_amount <= 1e-6 {
                    return DomainResult::empty();
                }

                let effects =
                    self.create_payment_effects(instrument.debtor, instrument.creditor, payment_amount, state);

                DomainResult::success(effects)
            } else {
                DomainResult::failure(vec!["Instrument is not a bond".to_string()])
            }
        } else {
            DomainResult::failure(vec!["Instrument not found".to_string()])
        }
    }

    fn calculate_daily_interest_accrual(
        &self, instrument: &FinancialInstrument, current_date: chrono::NaiveDate,
    ) -> f64 {
        if current_date <= instrument.last_accrual_date {
            return 0.0;
        }

        let (annual_rate_bps, day_count) =
            if let Some(deposit) = instrument.details.as_any().downcast_ref::<DemandDepositDetails>() {
                (deposit.interest_rate_bps, deposit.day_count)
            } else if let Some(deposit) = instrument.details.as_any().downcast_ref::<SavingsDepositDetails>() {
                (deposit.interest_rate_bps, deposit.day_count)
            } else if let Some(bond) = instrument.details.as_any().downcast_ref::<BondDetails>() {
                (bond.coupon_rate_bps, bond.day_count)
            } else {
                return 0.0;
            };

        day_count.calculate_accrued_interest(
            instrument.principal,
            annual_rate_bps,
            instrument.last_accrual_date,
            current_date,
        )
    }

    fn get_coupon_payment_amount(&self, instrument: &FinancialInstrument) -> Option<f64> {
        if let Some(bond) = instrument.details.as_any().downcast_ref::<BondDetails>() {
            Some(instrument.principal * bps_to_decimal(bond.coupon_rate_bps) / 2.0) // Semi-annual
        } else {
            None
        }
    }

    fn create_payment_effects(&self, from: AgentId, to: AgentId, amount: f64, state: &SimState) -> Vec<StateEffect> {
        vec![
            StateEffect::Financial(FinancialEffect::TransferFunds { from, to, amount }),
            StateEffect::Financial(FinancialEffect::RecordTransaction(Transaction {
                id: uuid::Uuid::new_v4(),
                date: state.ticknum,
                qty: amount,
                from,
                to,
                tx_type: TransactionType::InterestPayment { payer: from, receiver: to, amount: amount },
                instrument_id: None,
            })),
        ]
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Settlement",
        constructor: || Box::new(SettlementDomain::new()),
    }
}

```

---

## `crates/domains/src/settlement/mod.rs`

```rust
//! # Settlement Domain Crate
//!
//! This crate is responsible for the financial settlement processes that occur
//! periodically within the simulation. It handles "background" financial mechanics
//! that are not typically initiated by direct agent decisions, such as interest
//! accrual and coupon payments.
//!
//! ## Crate Structure and Purpose
//!
//! Unlike other domains, the settlement domain primarily consists of a `domain.rs` module.
//! There is no `behavior.rs` because settlement actions are usually triggered by the
//! simulation engine's clock rather than by an agent's `DecisionModel`.
//!
//! - **`domain.rs`**: Contains the `SettlementDomain` struct. This service validates
//!   and executes `SettlementAction`s.
//!   - **`AccrueInterest`**: Calculates and records the interest that has accrued on an
//!     instrument since the last calculation.
//!   - **`PayInterest`**: Creates the financial transaction to move accrued interest from
//!     the debtor to the creditor.
//!   - **`ProcessCouponPayment`**: Handles the fixed payments for bond instruments.
//!
//! The `SettlementDomain` translates these financial events into concrete `StateEffect`s,
//! ensuring that the simulation's financial plumbing works correctly over time.
//!
//! ## Key Components
//!
//! - **`SettlementDomain`**: The primary handler for executing all settlement-related tasks.
//! - **`SettlementAction`**: The set of financial settlement actions available, such as
//!   `AccrueInterest` (defined in `sim_actions`).
//! - **`SettlementResult`**: A struct wrapping the outcome, containing effects or errors.
pub mod domain;
pub use domain::*;
```

---

## `crates/domains/src/trading/domain.rs`

```rust
use serde::{Deserialize, Serialize};
use sim_core::*;
use crate::{Any, inventory, Domain, DomainResult, DomainValidator, ResolutionContext, ResolutionResult, ResolutionPhase};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradingDomain {}

impl TradingDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for TradingDomain {
    fn name(&self) -> &'static str { 
        "Trading" 
    }

    fn resolve_intention(&self, intention: &SimIntention, context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            // Treasury market making - complex pricing logic
            SimIntention::MarketMakeTreasuries { agent_id, tenor, quantity, bid_yield_bps, ask_yield_bps } => {
                self.resolve_treasury_market_making(*agent_id, *tenor, *quantity, *bid_yield_bps, *ask_yield_bps, context)
            },

            // Inventory sales with markup pricing
            SimIntention::SellInventory { agent_id, good_id, quantity, desired_markup } => {
                self.resolve_inventory_sale(*agent_id, *good_id, *quantity, *desired_markup, context)
            },

            // Input purchases at maximum price
            SimIntention::PurchaseInputs { agent_id, good_id, quantity, max_price } => {
                vec![SimAction::Trading(TradingAction::PostBid {
                    agent_id: *agent_id,
                    market_id: MarketId::Goods(*good_id),
                    quantity: *quantity,
                    price: *max_price,
                })]
            },

            // Consumer spending - purchase goods with budget
            SimIntention::SpendOnGood { agent_id, good_id, max_notional } => {
                self.resolve_consumer_spending(*agent_id, *good_id, *max_notional, context)
            },

            // Not a trading intention
            _ => return None,
        };

        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            // All trading intentions create market orders
            SimIntention::MarketMakeTreasuries { .. } |
            SimIntention::SellInventory { .. } |
            SimIntention::PurchaseInputs { .. } |
            SimIntention::SpendOnGood { .. } => Some(ResolutionPhase::Market),
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let trading_action = match action {
            SimAction::Trading(action) => action,
            _ => return DomainResult::failure(vec!["Not a trading action".to_string()]),
        };

        if let Err(error) = self.validate(trading_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match trading_action {
            TradingAction::PostBid { agent_id, market_id, quantity, price } => {
                self.execute_post_bid(*agent_id, market_id.clone(), *quantity, *price)
            },
            TradingAction::PostAsk { agent_id, market_id, quantity, price } => {
                self.execute_post_ask(*agent_id, market_id.clone(), *quantity, *price)
            },
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Resolution logic - domain-specific pricing and strategy
impl TradingDomain {
    fn resolve_treasury_market_making(
        &self, 
        agent_id: AgentId, 
        tenor: Tenor, 
        quantity: f64, 
        bid_yield_bps: BasisPoints, 
        ask_yield_bps: BasisPoints,
        context: &ResolutionContext
    ) -> Vec<SimAction> {
        const FACE_VALUE: f64 = 1000.0;
        const FREQUENCY: usize = 2;
        
        let benchmark_coupon_bps = context.state.financial_system.central_bank.policy_rate_bps;
        
        // Convert yields to prices using bond pricing
        let bid_price = pricing::bond_price(
            FACE_VALUE,
            bps_to_decimal(benchmark_coupon_bps),
            bps_to_decimal(bid_yield_bps),
            tenor.to_years(),
            FREQUENCY
        );

        let ask_price = pricing::bond_price(
            FACE_VALUE,
            bps_to_decimal(benchmark_coupon_bps),
            bps_to_decimal(ask_yield_bps),
            tenor.to_years(),
            FREQUENCY
        );

        let market_id = MarketId::Financial(FinancialMarketId::Treasury { tenor });

        vec![
            SimAction::Trading(TradingAction::PostBid {
                agent_id,
                market_id: market_id.clone(),
                quantity,
                price: bid_price,
            }),
            SimAction::Trading(TradingAction::PostAsk {
                agent_id,
                market_id,
                quantity,
                price: ask_price,
            }),
        ]
    }

    fn resolve_inventory_sale(
        &self, 
        agent_id: AgentId, 
        good_id: GoodId, 
        quantity: f64, 
        desired_markup: f64,
        context: &ResolutionContext
    ) -> Vec<SimAction> {
        // Price discovery: look up unit cost from inventory
        let unit_cost = context.state.financial_system
            .get_bs_by_id(&agent_id)
            .and_then(|bs| bs.get_inventory())
            .and_then(|inv| inv.get(&good_id))
            .map(|item| item.unit_cost)
            .unwrap_or(1.0); // Fallback cost if unknown

        let price = unit_cost * desired_markup;

        vec![SimAction::Trading(TradingAction::PostAsk {
            agent_id,
            market_id: MarketId::Goods(good_id),
            quantity,
            price,
        })]
    }

    fn resolve_consumer_spending(
        &self,
        agent_id: AgentId,
        good_id: GoodId,
        max_notional: f64,
        context: &ResolutionContext
    ) -> Vec<SimAction> {
        let state = context.state;
        
        // Get market and create bids at various price levels
        let market = match state.financial_system.exchange.goods_market(&good_id) {
            Some(m) => m,
            None => return vec![], // Market doesn't exist
        };

        let mut remaining_notional = max_notional;
        let mut actions = Vec::new();

        // Clone and sort asks by price (ascending)
        let mut asks = market.order_book.asks.clone();
        asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));

        // Create bid orders to match available asks within budget
        for ask in asks {
            if remaining_notional <= 1e-6 {
                break;
            }

            let cost_at_ask_price = ask.quantity * ask.price;
            let bid_quantity = if cost_at_ask_price <= remaining_notional {
                remaining_notional -= cost_at_ask_price;
                ask.quantity
            } else {
                let qty = remaining_notional / ask.price;
                remaining_notional = 0.0;
                qty
            };

            if bid_quantity > 1e-6 {
                actions.push(SimAction::Trading(TradingAction::PostBid {
                    agent_id,
                    market_id: MarketId::Goods(good_id),
                    quantity: bid_quantity,
                    price: ask.price,
                }));
            }
        }

        actions
    }
}

// Validation and execution
impl TradingDomain {
    fn validate(&self, action: &TradingAction, state: &SimState) -> Result<(), String> {
        match action {
            TradingAction::PostBid { agent_id, quantity, price, .. } |
            TradingAction::PostAsk { agent_id, quantity, price, .. } => {
                DomainValidator::positive_amount(*quantity)?;
                DomainValidator::positive_amount(*price)?;
                DomainValidator::agent_exists(*agent_id, state)?;
                Ok(())
            }
        }
    }

    fn execute_post_bid(&self, agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64) -> DomainResult {
        let bid = Bid { agent_id, quantity, price };
        let order = Order::Bid(bid);
        let effect = StateEffect::Market(MarketEffect::PlaceOrderInBook { market_id, order });
        
        DomainResult::success(vec![effect])
    }

    fn execute_post_ask(&self, agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64) -> DomainResult {
        let ask = Ask { agent_id, quantity, price };
        let order = Order::Ask(ask);
        let effect = StateEffect::Market(MarketEffect::PlaceOrderInBook { market_id, order });
        
        DomainResult::success(vec![effect])
    }
}

// Domain registration
inventory::submit! {
    crate::DomainRegistration {
        name: "Trading",
        constructor: || Box::new(TradingDomain::new()),
    }
}
```

---

## `crates/domains/src/trading/mod.rs`

```rust
//! # Trading Domain Crate
//!
//! This crate provides the logic for agent interactions with the simulated markets.
//! It handles the mechanics of placing orders and the financial settlement of executed trades.
//!
//! ## Crate Structure and Purpose
//!
//! The core of this crate is the `TradingDomain` struct, which acts as the interface
//! between agents and the market exchange.
//!
//! - **`domain.rs`**: Implements the `TradingDomain` struct. This service has two main roles:
//!   1.  **Executing Trading Actions**: It handles `TradingAction`s like `PostBid` and `PostAsk`.
//!       When an agent decides to trade, this domain validates the action (e.g., does the seller
//!       have the asset to sell?) and then creates a `PlaceOrderInBook` market effect. The actual
//!       matching of bids and asks is handled by the `Exchange` in `sim_types`.
//!   2.  **Settling Trades**: After the `Exchange` matches orders and creates `Trade` records, the
//!       `TradingDomain`'s `settle_financial_trade` method is called. This method is responsible
//!       for creating the `StateEffect`s that represent the financial outcome of the trade:
//!       transferring the asset from the seller to the buyer and transferring payment from the
//!       buyer to the seller.
//!
//! This crate does *not* contain a `behavior.rs` module because the *decision* to trade
//! is made within each agent's own domain (e.g., a bank's `BasicBankDecisionModel` decides
//! to trade bonds). This crate only provides the *mechanism* for that trade to occur.
//!
//! ## Key Components
//!
//! - **`TradingDomain`**: The service for posting orders and settling completed trades.
//! - **`TradingAction`**: The actions for posting bids and asks (defined in `sim_actions`).
//! - **`Trade`**: A data structure from `sim_types` representing a matched trade to be settled.
pub mod domain;
pub use domain::*;
```

---

