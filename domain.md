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
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct BasicBankDecisionModel;

#[typetag::serde]
impl DecisionModel for BasicBankDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, rng: &mut dyn RngCore) -> Vec<SimAction> {
        let bank = match agent.downcast_ref::<Bank>() {
            Some(b) => b,
            None => return vec![],
        };
        let mut actions = Vec::new();
        let fs = &state.financial_system;

        self.manage_reserves(bank, fs, &mut actions);
        self.market_make_treasuries(bank, fs, &mut actions, rng);

        actions
    }
}

impl BasicBankDecisionModel {
    fn manage_reserves(&self, bank: &Bank, fs: &FinancialSystem, actions: &mut Vec<SimAction>) {
        let total_deposits = fs.get_total_liabilities(&bank.id);
        let required_reserves = total_deposits * fs.central_bank.reserve_requirement;
        let desired_buffer = total_deposits * 0.02;
        let target_reserve_level = required_reserves + desired_buffer;

        let current_reserves = fs.get_bank_reserves(&bank.id).unwrap_or(0.0);
        let reserve_surplus_or_shortfall = current_reserves - target_reserve_level;

        let fed_funds_market_id = FinancialMarketId::FederalFundsOvernight;

        let floor_rate_bps = fs.central_bank.policy_rate_bps;
        let ceiling_rate_bps = floor_rate_bps + 25.0;
        let target_rate_bps = (floor_rate_bps + ceiling_rate_bps) / 2.0;

        let daily_rate = fed_funds_market_id.annual_bps_to_daily_rate(target_rate_bps);
        let price = 1.0 / (1.0 + daily_rate);

        if reserve_surplus_or_shortfall < -1.0 {
            let amount_needed = -reserve_surplus_or_shortfall;

            actions.push(SimAction::Trading(TradingAction::PostBid {
                agent_id: bank.id,
                market_id: MarketId::Financial(fed_funds_market_id.clone()),
                quantity: amount_needed,
                price,
            }));
        } else if reserve_surplus_or_shortfall > 1.0 {
            let amount_to_lend = reserve_surplus_or_shortfall * 0.75;
            if amount_to_lend > 100.0 {
                actions.push(SimAction::Trading(TradingAction::PostAsk {
                    agent_id: bank.id,
                    market_id: MarketId::Financial(fed_funds_market_id.clone()),
                    quantity: amount_to_lend,
                    price,
                }));
            }
        }
    }

    fn market_make_treasuries(&self, bank: &Bank, fs: &FinancialSystem, actions: &mut Vec<SimAction>, _rng: &mut dyn RngCore) {
        let bs = fs.get_bs_by_id(&bank.id).expect("Bank must have BS");

        let mut holdings_by_tenor: HashMap<Tenor, u64> = HashMap::new();
        for inst in bs.assets.values() {
            if let Some(bond_details) = inst.details.as_any().downcast_ref::<BondDetails>() {
                if bond_details.bond_type == BondType::Government {
                    *holdings_by_tenor.entry(bond_details.tenor).or_insert(0) += bond_details.quantity;
                }
            }
        }
        let quantity_to_quote = 5.0;
        const FACE_VALUE: f64 = 1000.0;
        let frequency = 2;

        for (market_id, _) in &fs.exchange.financial_markets {
            if let FinancialMarketId::Treasury { tenor } = market_id {
                let term_premium = match tenor {
                    Tenor::T2Y => 13.0,
                    Tenor::T5Y => 31.0,
                    Tenor::T10Y => 42.0,
                    Tenor::T30Y => 50.0,
                };
                let bid_ask_spread_bps = rand::rng().random_range(13.0..32.0); 
                let target_yield_bps = fs.central_bank.policy_rate_bps;

                let bid_yield_bps = term_premium + target_yield_bps + (bid_ask_spread_bps / 2.0);
                let ask_yield_bps = term_premium + target_yield_bps - (bid_ask_spread_bps / 2.0);

                let benchmark_coupon_bps = fs.central_bank.policy_rate_bps;
                
                let bid_price =
                    self.calculate_bond_price(FACE_VALUE, benchmark_coupon_bps, bid_yield_bps, *tenor, frequency);

                let ask_price =
                    self.calculate_bond_price(FACE_VALUE, benchmark_coupon_bps, ask_yield_bps, *tenor, frequency);

                actions.push(SimAction::Trading(TradingAction::PostBid {
                    agent_id: bank.id,
                    market_id: MarketId::Financial(market_id.clone()),
                    quantity: quantity_to_quote,
                    price: bid_price,
                }));

                let holdings = holdings_by_tenor.get(tenor).cloned().unwrap_or(0) as f64;
                if holdings >= quantity_to_quote {
                    actions.push(SimAction::Trading(TradingAction::PostAsk {
                        agent_id: bank.id,
                        market_id: MarketId::Financial(market_id.clone()),
                        quantity: quantity_to_quote,
                        price: ask_price,
                    }));
                }
            }
        }
    }

    fn calculate_bond_price(
        &self, face_value: f64, coupon_rate_bps: BasisPoints, ytm_bps: BasisPoints, tenor: Tenor, frequency: usize,
    ) -> f64 {
        let k = frequency as f64;
        let n = tenor.periods(frequency);

        if n == 0 {
            return face_value;
        }

        let coupon_rate = bps_to_decimal(coupon_rate_bps);
        let ytm = bps_to_decimal(ytm_bps);

        let c = coupon_rate * face_value / k;
        let y = ytm / k;

        let mut price = 0.0;
        let n_f64 = n as f64;

        if (y).abs() > 1e-9 {
            price += c * (1.0 - (1.0 + y).powf(-n_f64)) / y;
        } else {
            price += c * n_f64;
        }

        price += face_value / (1.0 + y).powf(n_f64);

        price
    }
}

#[cfg(test)]
mod banking_tests {
    use super::*;
    use chrono::naive;
    use uuid::Uuid;

    #[test]
    fn test_reserves_management() {
        let target = 442.5;
        let daily_rate = FinancialMarketId::FederalFundsOvernight.annual_bps_to_daily_rate(target);
        let price = 1.0 / (1.0 + daily_rate);
        println!("Price: {}", price);
        println!("Daily Rate: {}", daily_rate*10000.0);
        assert!(daily_rate > 0.0, "Daily rate should be positive"); 
    }

    #[test]
    fn test_bond_price_calculation() {
        let debtor = AgentId(Uuid::new_v4());
        let creditor = AgentId(Uuid::new_v4());
        let n_date = naive::NaiveDate::from_ymd_opt(2036, 1, 1).unwrap();
        let o_date = naive::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let _bond = bond!(
            creditor,
            debtor,
            1000.0,
            400.0,
            n_date,
            1000.0,
            BondType::Government,
            2,
            Tenor::T10Y,
            o_date
        );

    } 
}
```

---

## `crates/domains/src/banking/domain.rs`

```rust
use serde::{Deserialize, Serialize};
use sim_core::*;
use sim_macros::SimDomain;

#[derive(Clone, Debug, Serialize, Deserialize, SimDomain)]
pub struct BankingDomain {}

#[derive(Debug, Clone)]
pub struct BankingResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl BankingDomain {
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&self, action: &BankingAction, state: &SimState) -> BankingResult {
        if let Err(error) = self.basic_validate(action, state) {
            return BankingResult { success: false, effects: vec![], errors: vec![error] };
        }

        match action {
            BankingAction::Deposit { agent_id, bank, amount } => self.execute_deposit(*agent_id, *bank, *amount),
            BankingAction::Withdraw { agent_id, bank, amount } => self.execute_withdraw(*agent_id, *bank, *amount),
            BankingAction::Transfer { from, to, amount } => self.execute_transfer(*from, *to, *amount),
            BankingAction::PayWages { agent_id, employee, amount } => {
                self.execute_pay_wages(*agent_id, *employee, *amount)
            }
            BankingAction::UpdateReserves { bank: _, amount_change: _ } => {
                BankingResult {
                    success: false,
                    effects: vec![],
                    errors: vec!["Reserve updates not yet implemented with semantic effects".to_string()],
                }
            }
            BankingAction::InjectLiquidity => self.execute_inject_liquidity(state),
        }
    }

    fn basic_validate(&self, action: &BankingAction, state: &SimState) -> Result<(), String> {
        match action {
            BankingAction::Deposit { agent_id, bank, amount }
            | BankingAction::Withdraw { agent_id , bank, amount } => {
                Validator::positive_amount(*amount)?;
                self.validate_agent_exists(*agent_id, state)?;
                self.validate_bank_exists(*bank, state)?;
                Ok(())
            }
            BankingAction::Transfer { from, to, amount }
            | BankingAction::PayWages { agent_id: from, employee: to, amount } => {
                Validator::positive_amount(*amount)?;
                self.validate_agent_exists(*from, state)?;
                self.validate_agent_exists(*to, state)?;
                Ok(())
            }
            BankingAction::UpdateReserves { bank, amount_change: _ } => self.validate_bank_exists(*bank, state),
            BankingAction::InjectLiquidity => Ok(()),
        }
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

    pub fn execute_deposit(&self, depositor: AgentId, bank: AgentId, amount: f64) -> BankingResult {
        let effect = StateEffect::Financial(FinancialEffect::DepositFunds { depositor, bank, amount });
        BankingResult { success: true, effects: vec![effect], errors: vec![] }
    }

    pub fn execute_withdraw(&self, account_holder: AgentId, bank: AgentId, amount: f64) -> BankingResult {
        let effect = StateEffect::Financial(FinancialEffect::WithdrawFunds { account_holder, bank, amount });
        BankingResult { success: true, effects: vec![effect], errors: vec![] }
    }

    pub fn execute_transfer(&self, from: AgentId, to: AgentId, amount: f64) -> BankingResult {
        let effect = StateEffect::Financial(FinancialEffect::TransferFunds { from, to, amount });
        BankingResult { success: true, effects: vec![effect], errors: vec![] }
    }

    pub fn execute_pay_wages(&self, employer: AgentId, employee: AgentId, amount: f64) -> BankingResult {
        let effect = StateEffect::Financial(FinancialEffect::PayWages { employer, employee, amount });
        BankingResult { success: true, effects: vec![effect], errors: vec![] }
    }

    pub fn execute_inject_liquidity(&self, state: &SimState) -> BankingResult {
        let recipients: Vec<AgentId> = state.agents.consumers.keys().cloned().collect();
        let amount_per_recipient = 1000.0;

        let effect = StateEffect::Financial(FinancialEffect::InjectLiquidity { recipients, amount_per_recipient });
        BankingResult { success: true, effects: vec![effect], errors: vec![] }
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
    pub mpc: f64,
}

impl Default for SimpleConsumerDecisionModel {
    fn default() -> Self {
        Self {
            mpc: 0.7,
        }
    }
}

#[typetag::serde]
impl DecisionModel for SimpleConsumerDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, rng: &mut dyn RngCore) -> Vec<SimAction> {
        let consumer = match agent.downcast_ref::<Consumer>() {
            Some(c) => c,
            None => return vec![],
        };

        let mut actions = Vec::new();

        self.handle_employment(consumer, state, &mut actions);

        let fs = &state.financial_system;
        let weekly_income = consumer.income / 52.0;
        let liquid_assets = fs.get_liquid_assets(&consumer.id);
        let total_resources = weekly_income + liquid_assets;
        
        if total_resources < 1.0 {
            return actions;
        }

        let budget = total_resources * self.mpc;
        let save_amount = total_resources - budget;

        self.make_simple_purchases(consumer, budget, state, &mut actions, rng);

        if save_amount > 1.0 {
            actions.push(SimAction::Banking(BankingAction::Deposit {
                agent_id: consumer.id,
                bank: consumer.bank_id,
                amount: save_amount,
            }));
        }

        actions
    }
}

impl SimpleConsumerDecisionModel {
    fn handle_employment(&self, consumer: &Consumer, _state: &SimState, actions: &mut Vec<SimAction>) {
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

            actions.push(SimAction::Labour(LabourAction::ApplyForJob {
                market_id: LabourMarketId::GeneralLabour,
                application,
            }));
        }
    }

    fn make_simple_purchases(&self, consumer: &Consumer, budget: f64, state: &SimState, actions: &mut Vec<SimAction>, rng: &mut dyn RngCore) {

        let consumption_basket = vec![
            ("bread", 0.4, 3.0),
            ("petrol", 0.6, 4.0),
        ];

        for (good_slug, budget_share, fallback_price) in consumption_basket {
            let good_id = match goods::CATALOGUE.get_good_id_by_slug(good_slug) {
                Some(id) => id,
                None => {
                    println!("Warning: Good '{}' not found in catalogue", good_slug);
                    continue;
                }
            };

            let allocation = budget * budget_share;
            

            let price = state.market_view(&MarketId::Goods(good_id))
                .and_then(|view| view.last_or_mid())
                .unwrap_or(fallback_price);


            let bid_price = price * rng.random_range(0.95..1.05);
            let max_quantity = allocation / bid_price;

            if allocation > 0.01 && max_quantity > 0.01 {
                actions.push(SimAction::Trading(TradingAction::PostBid {
                    agent_id: consumer.id,
                    market_id: MarketId::Goods(good_id),
                    quantity: max_quantity,
                    price: bid_price,
                }));
            }
        }
    }
}




#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CESConsumerDecisionModel {
    pub sigma: f64,
    pub weights: HashMap<GoodId, f64>,
    pub mpc_base: f64,
}

impl Default for CESConsumerDecisionModel {
    fn default() -> Self {
        let mut weights = HashMap::new();
        if let Some(petrol_id) = goods::CATALOGUE.get_good_id_by_slug("petrol") {
            weights.insert(petrol_id, 0.2);
            weights.insert(goods::CATALOGUE.get_good_id_by_slug("bread").unwrap(), 0.5);
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
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimAction> {
        let consumer = match agent.downcast_ref::<Consumer>() {
            Some(c) => c,
            None => return vec![],
        };
        println!("Consumer {} making decisions", consumer.id);
        let mut actions = Vec::new();

        self.handle_employment(consumer, state, &mut actions);

        let nominal_rate_bps = state.financial_system.central_bank.policy_rate_bps;
        let nominal_rate = bps_to_decimal(nominal_rate_bps);
        let expected_inflation = consumer.expectations.expected_inflation;
        let real_rate = nominal_rate - expected_inflation;

        let mpc_adjustment = (real_rate - 0.02).max(0.0) * 5.0;
        let mpc = (self.mpc_base - mpc_adjustment).max(0.1).min(0.95);

        let fs = &state.financial_system;
        let weekly_income = consumer.income / 52.0;
        let liquid_assets = fs.get_liquid_assets(&consumer.id);
        let total_resources = weekly_income + liquid_assets;

        let budget = total_resources * mpc;
        let save_amount = total_resources - budget;

        if budget < 1.0 {
            self.handle_savings(consumer, save_amount, &mut actions);
            return actions;
        }

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
        println!("Consumer {} market data: {:#?}", consumer.id, market_data.clone());
        if market_data.is_empty() {
            self.handle_savings(consumer, save_amount, &mut actions);
            return actions;
        }

        let denominator: f64 = market_data.iter().map(|(_, price, weight)| {
            weight * price.powf(1.0 - self.sigma)
        }).sum();

        if denominator <= 1e-9 {
            self.handle_savings(consumer, save_amount, &mut actions);
            return actions;
        }

        for (good_id, price, weight) in market_data {
            let share = (weight * price.powf(1.0 - self.sigma)) / denominator;

            let notional = share * budget;

            if notional > 0.01 {
                actions.push(SimAction::Consumption(ConsumptionAction::PurchaseAtBest {
                    agent_id: consumer.id,
                    good_id,
                    max_notional: notional,
                }));
            }
        }
        let bread_id = goods::CATALOGUE.get_good_id_by_slug("bread").unwrap();
        self.handle_savings(consumer, save_amount, &mut actions);
        self.handle_purchase(consumer, bread_id, 1.0, &mut actions);
        
        actions
    }
}

impl CESConsumerDecisionModel {
    fn handle_savings(&self, consumer: &Consumer, save_amount: f64, actions: &mut Vec<SimAction>) {
        if save_amount > 1.0 {
            actions.push(SimAction::Banking(BankingAction::Deposit {
                agent_id: consumer.id,
                bank: consumer.bank_id,
                amount: save_amount
            }));
        }
    }

    fn handle_employment(&self, consumer: &Consumer, _state: &SimState, actions: &mut Vec<SimAction>) {
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

            actions.push(SimAction::Labour(LabourAction::ApplyForJob {
                market_id: LabourMarketId::GeneralLabour,
                application,
            }));
        }
    }

    fn handle_purchase(
        &self,
        consumer: &Consumer,
        good_id: GoodId,
        amount: f64,
        actions: &mut Vec<SimAction>,
    ) {
        println!("Consumer {} attempting to purchase {} units of good {}", consumer.id, amount, good_id);
        if amount > 0.0 {
            actions.push(SimAction::Consumption(ConsumptionAction::Purchase {
                agent_id: consumer.id,
                seller: consumer.id,
                good_id,
                amount,
            }));
        }
    }
}
```

---

## `crates/domains/src/consumption/domain.rs`

```rust
use serde::{Deserialize, Serialize};
use sim_core::*;
use sim_macros::SimDomain;

#[derive(Clone, Debug, Serialize, Deserialize, SimDomain)]
pub struct ConsumptionDomain {}

#[derive(Debug, Clone)]
pub struct ConsumptionResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl ConsumptionDomain {
    pub fn new() -> Self {
        Self {}
    }

    pub fn can_handle(&self, action: &ConsumptionAction) -> bool {
        matches!(action, ConsumptionAction::Purchase { .. } | ConsumptionAction::Consume { .. } | ConsumptionAction::PurchaseAtBest { .. })
    }

    pub fn validate(&self, action: &ConsumptionAction, state: &SimState) -> Result<(), String> {
        match action {
            ConsumptionAction::Purchase { agent_id, seller, good_id, amount } => {
                self.validate_purchase(*agent_id, *seller, *good_id, *amount, state)
            }
            ConsumptionAction::PurchaseAtBest { agent_id, good_id, max_notional } => {
                self.validate_purchase_at_best(*agent_id, *good_id, *max_notional, state)
           }
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                self.validate_consume(*agent_id, *good_id, *amount, state)
            }
            ConsumptionAction::NoAction { agent_id: _agent_id } => Ok(()),
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

        if !state.financial_system.balance_sheets.contains_key(&buyer) {
            return Err(format!("Buyer {:?} not found", buyer));
        }
        if !state.financial_system.balance_sheets.contains_key(&seller) {
            return Err(format!("Seller {:?} not found", seller));
        }

        let seller_bs = state.financial_system.balance_sheets.get(&seller).unwrap();
        let available_inventory =
            seller_bs.get_inventory().and_then(|inv| inv.get(&good_id)).map_or(0.0, |item| item.quantity);
        if available_inventory < amount {
            return Err(format!(
                "Seller has insufficient inventory: needs {:.2}, has {:.2}",
                amount, available_inventory
            ));
        }

        let price =
            state.financial_system.exchange.goods_market(&good_id).and_then(|m| m.best_ask()).map_or(1.0, |ask| ask.price);
        let total_cost = amount * price;
        let available_funds = state.financial_system.get_liquid_assets(&buyer);
        if available_funds < total_cost {
            return Err(format!("Buyer has insufficient funds: needs ${:.2}, has ${:.2}", total_cost, available_funds));
        }

        Ok(())
    }

    fn validate_purchase_at_best(
        &self,
        buyer: AgentId,
        _good_id: GoodId,
        max_notional: f64,
        state: &SimState,
    ) -> Result<(), String> {
        Validator::positive_amount(max_notional)?;
        if !state.financial_system.balance_sheets.contains_key(&buyer) {
            return Err(format!("Buyer {:?} not found", buyer));
        }
        let available_funds = state.financial_system.get_liquid_assets(&buyer);
        if available_funds < max_notional {
             return Err(format!("Buyer has insufficient funds for max notional: needs ${:.2}, has ${:.2}", max_notional, available_funds));
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

    pub fn execute(&self, action: &ConsumptionAction, state: &SimState) -> ConsumptionResult {
        if let Err(error) = self.validate(action, state) {
            return ConsumptionResult { success: false, effects: vec![], errors: vec![error] };
        }

        match action {
            ConsumptionAction::Purchase { agent_id, seller, good_id, amount } => {
                self.execute_purchase(*agent_id, *seller, *good_id, *amount, state)
            }
            ConsumptionAction::PurchaseAtBest { agent_id, good_id, max_notional } => {
                self.execute_purchase_at_best(*agent_id, *good_id, *max_notional, state)
            }
            ConsumptionAction::Consume { agent_id, good_id, amount } => self.execute_consume(*agent_id, *good_id, *amount),
            ConsumptionAction::NoAction { agent_id: _ } => {
                ConsumptionResult { success: true, effects: vec![], errors: vec![] }
            }
        }
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
        let price =
            state.financial_system.exchange.goods_market(&good_id).and_then(|m| m.best_ask()).map_or(1.0, |ask| ask.price);

        let total_cost = amount * price;

        effects.push(StateEffect::Financial(FinancialEffect::RecordTransaction(Transaction {
            id: uuid::Uuid::new_v4(),
            date: state.ticknum,
            qty: total_cost,
            from: buyer,
            to: seller,
            tx_type: TransactionType::Transfer { from: buyer, to: seller, amount: total_cost },
            instrument_id: None,
        })));

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

    pub fn execute_purchase_at_best(
        &self,
        buyer: AgentId,
        good_id: GoodId,
        max_notional: f64,
        state: &SimState,
    ) -> ConsumptionResult {
         let market = match state.financial_system.exchange.goods_market(&good_id) {
            Some(m) => m,
            None => return ConsumptionResult { success: true, effects: vec![], errors: vec![] }, // Market doesn't exist
        };

        let mut remaining_notional = max_notional;
        let mut effects = vec![];

        let mut asks = market.order_book.asks.clone();
        asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));

        for ask in asks {
            if remaining_notional <= 1e-6 {
                break;
            }

            let cost_at_ask_price = ask.quantity * ask.price;
            let bid_quantity;

            if cost_at_ask_price <= remaining_notional {
                bid_quantity = ask.quantity;
                remaining_notional -= cost_at_ask_price;
            } else {
                bid_quantity = remaining_notional / ask.price;
                remaining_notional = 0.0;
            }

            if bid_quantity > 1e-6 {
                effects.push(StateEffect::Market(MarketEffect::PlaceOrderInBook {
                    market_id: MarketId::Goods(good_id),
                    order: Order::Bid(Bid {
                        agent_id: buyer,
                        quantity: bid_quantity,
                        price: ask.price,
                    }),
                }));
            }
        }

        ConsumptionResult { success: true, effects, errors: vec![] }
    }

    pub fn execute_consume(&self, agent_id: AgentId, good_id: GoodId, amount: f64) -> ConsumptionResult {
        let effects =
            vec![StateEffect::Inventory(InventoryEffect::RemoveInventory { owner: agent_id, good_id, quantity: amount })];

        ConsumptionResult { success: true, effects, errors: vec![] }
    }
}

impl Default for ConsumptionDomain {
    fn default() -> Self {
        Self::new()
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
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimAction> {
        let government = match agent.downcast_ref::<Government>() {
            Some(g) => g,
            None => return vec![],
        };

        let mut actions = Vec::new();
        if state.current_date.ordinal() % 30 == 0{ // Every 30 days, collect taxes 
            let tax_rate = government.tax_rates.income_tax;

            for consumer in state.agents.consumers.values() {
                let tax_liability = (consumer.income/12.0) * tax_rate; // Monthly income tax
                if tax_liability > 0.0 {
                    actions.push(SimAction::Banking(BankingAction::Transfer {
                        from: consumer.id,
                        to: government.id,
                        amount: tax_liability,
                    }));
                }
            }
        }
        self.handle_funding(government, state, &mut actions);
        actions
    }
}

impl BasicGovernmentDecisionModel {
    fn handle_funding(&self, government: &Government, state: &SimState, actions: &mut Vec<SimAction>) {
        let fs = &state.financial_system;
        let gbs = fs.balance_sheets.get(&government.id);
        if gbs.is_none() {
            return;
        }
        let gbs = gbs.unwrap();
        let current_balance = gbs.liquid_assets(); 
        let spending_target = 1_000_000.0 / 12.0; // 1m / 12
        if current_balance < spending_target {
            let deficit = spending_target - current_balance;
            // Issuance distribution across different tenors
            let issue_distribution = [
                (Tenor::T2Y, 0.15), 
                (Tenor::T5Y, 0.25), 
                (Tenor::T10Y, 0.40), 
                (Tenor::T30Y, 0.20)
            ];
            let face_value = 1000.0;
            // Use the central bank policy rate as a proxy for the new coupon rate
            let coupon_rate = fs.central_bank.policy_rate_bps;
            for (tenor, percentage) in issue_distribution {
                let amount_to_issue = deficit * percentage;
                let quantity = (amount_to_issue / face_value).ceil() as u32;
                if quantity > 0 {
                    actions.push(SimAction::Fiscal(FiscalAction::IssueDebt {
                        government_id: government.id,
                        tenor,
                        quantity,
                        face_value,
                        coupon_rate,
                    }));
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
use sim_macros::SimDomain;

#[derive(Clone, Debug, Serialize, Deserialize, Default, SimDomain)]
pub struct FiscalDomain {}

#[derive(Debug, Clone)]
pub struct FiscalResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl FiscalDomain {
    pub fn new() -> Self {
        Self {}
    }
    pub fn can_handle(&self, action: &FiscalAction) -> bool {
        match action {
            FiscalAction::ChangeTaxRate { .. } => true,
            FiscalAction::IssueDebt { .. } => true,
            FiscalAction::SetSpendingTarget { .. } => true,
        }
    }
    pub fn validate(&self, _action: &FiscalAction, _state: &SimState) -> FiscalResult {
        let errors = vec![];

        FiscalResult { success: errors.is_empty(), effects: vec![], errors }
    }
    pub fn execute(&self, action: &FiscalAction, state: &SimState) -> FiscalResult {
        let mut effects = vec![];
        let fs = &state.financial_system;
        match action {
            FiscalAction::ChangeTaxRate { government_id, tax_type, new_rate } => {
                println!(
                    "[FISCAL DOMAIN] Executing ChangeTaxRate for {} | Setting {:?} to {}",
                    government_id, tax_type, new_rate
                );
            }
            FiscalAction::IssueDebt { government_id, tenor: _, quantity, face_value: _, coupon_rate: _ } => {
                for (market_id, _) in &fs.exchange.financial_markets {
                    if let FinancialMarketId::Treasury { tenor: _ } = market_id {
                        let ask_price = 1050.0; // Market order
                        let ask_order =
                            Order::Ask(Ask { agent_id: *government_id, quantity: *quantity as f64, price: ask_price });

                        effects
                            .push(StateEffect::Market(MarketEffect::PlaceOrderInBook { market_id: MarketId::Financial(market_id.clone()), order: ask_order }));
                    }
                }
            }
            FiscalAction::SetSpendingTarget { government_id, .. } => {
                println!("[FISCAL DOMAIN] Executing SetSpendingTarget for {}", government_id);
            }
        }
        FiscalResult { success: true, effects, errors: vec![] }
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
use sim_macros::SimDomain;

#[derive(Clone, Debug, Serialize, Deserialize, Default, SimDomain)]
pub struct LabourDomain {}

#[derive(Debug, Clone)]
pub struct LabourResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl LabourDomain {
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&self, action: &LabourAction, _state: &SimState) -> LabourResult {

        match action {
            LabourAction::ApplyForJob { market_id, application } => self.execute_apply(market_id.clone(), application.clone()),
            LabourAction::PostJobOffer { market_id, offer } => self.execute_post_offer(market_id.clone(), offer.clone()),
            LabourAction::Fire { firm_id, employee_id } => self.execute_fire(*firm_id, *employee_id),
        }
    }

    fn execute_apply(&self, market_id: LabourMarketId, application: JobApplication) -> LabourResult {
        let effect = StateEffect::Market(MarketEffect::UpdateLabourMarket {
            market_id,
            update: LabourMarketUpdate::AddApplication(application),
        });
        LabourResult { success: true, effects: vec![effect], errors: vec![] }
    }

    fn execute_post_offer(&self, market_id: LabourMarketId, offer: JobOffer) -> LabourResult {
        let effect = StateEffect::Market(MarketEffect::UpdateLabourMarket {
            market_id,
            update: LabourMarketUpdate::AddOffer(offer),
        });
        LabourResult { success: true, effects: vec![effect], errors: vec![] }
    }

    fn execute_fire(&self, firm_id: AgentId, employee_id: AgentId) -> LabourResult {
        let effect = StateEffect::Agent(AgentEffect::TerminateEmployment {
            firm_id,
            consumer_id: employee_id,
        });
        LabourResult { success: true, effects: vec![effect], errors: vec![] }
    }
}
```

---

## `crates/domains/src/lib.rs`

```rust
use serde::{Deserialize, Serialize};
use sim_core::{SimAction, SimState, StateEffect};
use std::any::Any;

pub trait Domain: Send + Sync {
    fn name(&self) -> &'static str;

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult;

    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

pub struct DomainRegistration {
    pub name: &'static str,
    pub constructor: fn() -> Box<dyn Domain>,
}

inventory::collect!(DomainRegistration);

pub mod banking;
pub mod consumption;
pub mod fiscal;
pub mod prelude;
pub mod production;
pub mod settlement;
pub mod trading;
pub mod labour;
```

---

## `crates/domains/src/prelude/mod.rs`

```rust
pub use super::{Domain, DomainRegistration, DomainResult};
pub use crate::banking::{BankingDomain, BankingResult, BasicBankDecisionModel};
pub use crate::consumption::{CESConsumerDecisionModel, ConsumptionDomain, ConsumptionResult};
pub use crate::fiscal::{BasicGovernmentDecisionModel, FiscalDomain, FiscalResult};
pub use crate::production::{SimpleFirmDecisionModel, ProductionDomain, ProductionResult};
pub use crate::settlement::{SettlementDomain, SettlementResult};
pub use crate::trading::{TradingDomain, TradingResult};
pub use crate::labour::{LabourDomain, LabourResult};
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
}

impl Default for SimpleFirmDecisionModel {
    fn default() -> Self {
        Self {
            target_markup: 1.25, // 25% markup
            base_wage: 25.0,     // Base hourly wage
        }
    }
}

#[typetag::serde]
impl DecisionModel for SimpleFirmDecisionModel {
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimAction> {
        let firm = match agent.downcast_ref::<Firm>() {
            Some(f) => f,
            None => return vec![],
        };
        
        let mut actions = Vec::new();
        let _fs = &state.financial_system;

        self.handle_hiring(firm, &mut actions);

        self.handle_production(firm, state, &mut actions);

        self.handle_wages(firm, &mut actions);

        self.handle_sales(firm, state, &mut actions);

        self.handle_input_purchases(firm, state, &mut actions);

        actions
    }
}

impl SimpleFirmDecisionModel {
    fn handle_hiring(&self, firm: &Firm, actions: &mut Vec<SimAction>) {
        let current_employees = firm.employees.len();
        let target_employees = 3;
        if current_employees < target_employees {
            let positions_to_fill = target_employees - current_employees;
            actions.push(SimAction::Production(ProductionAction::Hire { 
                agent_id: firm.id, 
                count: positions_to_fill as u32,
            }));
        }
    }

    fn handle_production(&self, firm: &Firm, state: &SimState, actions: &mut Vec<SimAction>) {
        if firm.employees.is_empty() {
            return;
        }

        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = state.financial_system.goods.get_recipe(&recipe_id) {
                if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id)
                    .and_then(|bs| bs.get_inventory()) {
                    
                    let can_produce = recipe.inputs.iter().all(|(good_id, required_qty)| {
                        inventory.get(good_id).map_or(false, |item| item.quantity >= *required_qty)
                    });

                    if can_produce {
                        actions.push(SimAction::Production(ProductionAction::Produce { 
                            agent_id: firm.id, 
                            recipe_id, 
                            batches: 1,
                        }));
                    } else {
                        println!("Firm {} cannot produce - missing inputs", firm.name);
                    }
                }
            }
        }
    }

    fn handle_wages(&self, firm: &Firm, actions: &mut Vec<SimAction>) {
        for (employee_id, contract) in &firm.employees {
            let weekly_wage = contract.wage_rate * contract.hours;
            if weekly_wage > 0.0 {
                actions.push(SimAction::Banking(BankingAction::PayWages {
                    agent_id: firm.id,
                    employee: *employee_id,
                    amount: weekly_wage,
                }));
            }
        }
    }

    fn handle_sales(&self, firm: &Firm, state: &SimState, actions: &mut Vec<SimAction>) {
        if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id)
            .and_then(|bs| bs.get_inventory()) {
            
            for (good_id, item) in inventory {
                if item.quantity > 0.1 {
                    let base_price = self.calculate_selling_price(firm, *good_id, state);
                    
                    actions.push(SimAction::Trading(TradingAction::PostAsk {
                        agent_id: firm.id,
                        market_id: MarketId::Goods(*good_id),
                        quantity: item.quantity,
                        price: base_price,
                    }));
                }
            }
        }
    }

    fn handle_input_purchases(&self, firm: &Firm, state: &SimState, actions: &mut Vec<SimAction>) {
        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = state.financial_system.goods.get_recipe(&recipe_id) {
                if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id)
                    .and_then(|bs| bs.get_inventory()) {
                    
                    for (input_good_id, required_qty) in &recipe.inputs {
                        let current_qty = inventory.get(input_good_id)
                            .map_or(0.0, |item| item.quantity);
                        
                        let target_qty = required_qty * 2.0;
                        if current_qty < target_qty {
                            let buy_qty = target_qty - current_qty;
                            let max_price = 100.0; // Willing to pay up to $100 per unit
                            
                            actions.push(SimAction::Trading(TradingAction::PostBid {
                                agent_id: firm.id,
                                market_id: MarketId::Goods(*input_good_id),
                                quantity: buy_qty,
                                price: max_price,
                            }));
                        }
                    }
                }
            }
        }
    }

    fn calculate_selling_price(&self, firm: &Firm, good_id: GoodId, state: &SimState) -> f64 {
        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = state.financial_system.goods.get_recipe(&recipe_id) {
                if recipe.output.0 == good_id {
                    let labor_cost_per_hour = self.base_wage;
                    let labor_hours_per_unit = recipe.labour_hours;
                    let labor_cost_per_unit = labor_cost_per_hour * labor_hours_per_unit;
                    
                    let input_cost_per_unit: f64 = recipe.inputs.iter()
                        .map(|(_, qty)| qty * 50.0) // Assume $50 per unit of input
                        .sum();
                    
                    let total_cost = (labor_cost_per_unit + input_cost_per_unit) / recipe.efficiency;
                    return total_cost * self.target_markup;
                }
            }
        }
        
        match goods::CATALOGUE.get_good_by_id(&good_id) {
            Some(good) => match good.name.as_str() {
                "Bread" => 3.0,
                "Petrol" => 4.0,
                "Crude Oil" => 60.0,
                "Wheat" => 8.0,
                _ => 10.0,
            },
            None => 10.0,
        }
    }
}
```

---

## `crates/domains/src/production/domain.rs`

```rust
use serde::{Deserialize, Serialize};
use sim_core::*;
use sim_macros::SimDomain;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, SimDomain)]
pub struct ProductionDomain {}

#[derive(Debug, Clone)]
pub struct ProductionResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl ProductionDomain {
    pub fn new() -> Self {
        Self {}
    }

    pub fn can_handle(&self, action: &ProductionAction) -> bool {
        matches!(action, ProductionAction::Hire { .. } | ProductionAction::Produce { .. })
    }

    pub fn validate(&self, action: &ProductionAction, state: &SimState) -> Result<(), String> {
        match action {
            ProductionAction::Hire { agent_id, count } => self.validate_hire(*agent_id, *count, state),
            ProductionAction::Produce { agent_id, recipe_id, batches } => {
                self.validate_produce(*agent_id, *recipe_id, *batches, state)
            }
        }
    }

    fn validate_hire(&self, firm_id: AgentId, count: u32, state: &SimState) -> Result<(), String> {
        Validator::positive_integer(count, "hire count")?;
        if !state.agents.firms.contains_key(&firm_id) {
            return Err(format!("Firm {:?} not found", firm_id));
        }
        Ok(())
    }

    fn validate_produce(
        &self, firm_id: AgentId, recipe_id: RecipeId, batches: u32, state: &SimState,
    ) -> Result<(), String> {
        Validator::positive_integer(batches, "production batches")?;
        let firm = state.agents.firms.get(&firm_id).ok_or(format!("Firm {:?} not found", firm_id))?;
        let recipe =
            state.financial_system.goods.recipes.get(&recipe_id).ok_or(format!("Recipe {:?} not found", recipe_id))?;

        let bs = state.financial_system.balance_sheets.get(&firm_id).ok_or("Firm has no balance sheet")?;
        let inventory = bs.get_inventory().ok_or("Firm has no inventory")?;

        for (input_good, required_qty) in &recipe.inputs {
            let available = inventory.get(input_good).map_or(0.0, |item| item.quantity);
            let total_needed = *required_qty * batches as f64;
            if available < total_needed {
                return Err(format!(
                    "Insufficient input {:?}: have {:.2}, need {:.2}",
                    input_good, available, total_needed
                ));
            }
        }

        if firm.employees.is_empty() {
            return Err("Firm has no employees to produce".to_string());
        }
        Ok(())
    }

    pub fn execute(&self, action: &ProductionAction, state: &SimState) -> ProductionResult {
        if let Err(error) = self.validate(action, state) {
            return ProductionResult { success: false, effects: vec![], errors: vec![error] };
        }

        match action {
            ProductionAction::Hire { agent_id, count } => self.execute_hire(*agent_id, *count, state),
            ProductionAction::Produce { agent_id, recipe_id, batches } => {
                self.execute_produce(*agent_id, *recipe_id, *batches, state)
            }
        }
    }
    pub fn execute_hire(&self, firm_id: AgentId, count: u32, state: &SimState) -> ProductionResult {
        let firm = match state.agents.firms.get(&firm_id) {
            Some(f) => f,
            None => {
                return ProductionResult {
                    success: false,
                    effects: vec![],
                    errors: vec![format!("Firm {:?} not found", firm_id)],
                };
            }
        };

        let offer = JobOffer {
            offer_id: Uuid::new_v4(),
            firm_id,
            wage_rate: firm.wage_rate,
            hours_required: 40.0,
            quantity: count,
        };

        let effect = StateEffect::Market(MarketEffect::UpdateLabourMarket {
            market_id: LabourMarketId::GeneralLabour,
            update: LabourMarketUpdate::AddOffer(offer),
        });

        ProductionResult { success: true, effects: vec![effect], errors: vec![] }
    }

    pub fn execute_produce(
        &self, firm_id: AgentId, recipe_id: RecipeId, batches: u32, state: &SimState,
    ) -> ProductionResult {
        let recipe = match state.financial_system.goods.recipes.get(&recipe_id) {
            Some(r) => r,
            None => {
                return ProductionResult {
                    success: false,
                    effects: vec![],
                    errors: vec![format!("Recipe {:?} not found", recipe_id)],
                };
            }
        };

        let mut effects = vec![];
        let total_batches = batches as f64;

        for (input_good, required_qty) in &recipe.inputs {
            effects.push(StateEffect::Inventory(InventoryEffect::RemoveInventory {
                owner: firm_id,
                good_id: *input_good,
                quantity: *required_qty * total_batches,
            }));
        }

        // Compute total input cost using current moving-average costs from inventory, then convert to per-unit output.
        let bs = state.financial_system.get_bs_by_id(&firm_id).ok_or("Firm has no balance sheet").unwrap();
        let inv = bs.get_inventory().ok_or("Firm has no inventory").unwrap();

        let (output_good, output_qty) = &recipe.output;
        let total_output = output_qty * total_batches * recipe.efficiency;

        let total_input_cost: f64 = recipe
            .inputs
            .iter()
            .map(|(gid, req)| {
                let unit = inv.get(gid).map(|it| it.unit_cost).unwrap_or(0.0);
                unit * (*req) * total_batches
            })
            .sum();

        let unit_cost = if total_output > 0.0 { total_input_cost / total_output } else { 0.0 };

        effects.push(StateEffect::Inventory(InventoryEffect::AddInventory {
            owner: firm_id,
            good_id: *output_good,
            quantity: total_output,
            unit_cost,
        }));

        ProductionResult { success: true, effects, errors: vec![] }
    }
}

impl Default for ProductionDomain {
    fn default() -> Self {
        Self::new()
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
use serde::{Deserialize, Serialize};
use sim_core::*;
use sim_macros::SimDomain;

#[derive(Clone, Debug, Serialize, Deserialize, Default, SimDomain)]
pub struct SettlementDomain {}

#[derive(Debug, Clone)]
pub struct SettlementResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl SettlementDomain {
    pub fn new() -> Self {
        Self {}
    }

    pub fn can_handle(&self, action: &SettlementAction) -> bool {
        matches!(
            action,
            SettlementAction::AccrueInterest { .. }
                | SettlementAction::PayInterest { .. }
                | SettlementAction::ProcessCouponPayment { .. }
        )
    }

    pub fn validate(&self, action: &SettlementAction, state: &SimState) -> Result<(), String> {
        match action {
            SettlementAction::AccrueInterest { instrument_id } => self.validate_accrue_interest(instrument_id, state),
            SettlementAction::PayInterest { instrument_id } => self.validate_pay_interest(instrument_id, state),
            SettlementAction::ProcessCouponPayment { instrument_id } => {
                self.validate_process_coupon_payment(instrument_id, state)
            }
        }
    }

    fn validate_accrue_interest(&self, instrument_id: &InstrumentId, state: &SimState) -> Result<(), String> {
        if !state.financial_system.instruments.contains_key(instrument_id) {
            return Err(format!("Instrument {:?} not found for accrual.", instrument_id));
        }
        Ok(())
    }

    fn validate_pay_interest(&self, instrument_id: &InstrumentId, state: &SimState) -> Result<(), String> {
        let instrument = state
            .financial_system
            .instruments
            .get(instrument_id)
            .ok_or(format!("Instrument {:?} not found for interest payment.", instrument_id))?;

        let interest_to_pay = instrument.accrued_interest;
        if interest_to_pay <= 1e-6 {
            return Ok(());
        }

        let available_funds = state.financial_system.get_liquid_assets(&instrument.debtor);
        if available_funds < interest_to_pay {
            return Err(format!(
                "Insufficient funds for interest payment: agent {:?} needs ${:.2}, has ${:.2}",
                instrument.debtor, interest_to_pay, available_funds
            ));
        }
        Ok(())
    }

    fn get_coupon_payment_amount(&self, instrument: &FinancialInstrument) -> Option<f64> {
        if let Some(bond) = instrument.details.as_any().downcast_ref::<BondDetails>() {
            let annual_coupon_rate = bps_to_decimal(bond.coupon_rate_bps);
            let payment = (instrument.principal * annual_coupon_rate) / bond.frequency as f64;
            Some(payment)
        } else {
            None
        }
    }

    fn validate_process_coupon_payment(&self, instrument_id: &InstrumentId, state: &SimState) -> Result<(), String> {
        let instrument = state
            .financial_system
            .instruments
            .get(instrument_id)
            .ok_or(format!("Instrument {:?} not found for coupon payment.", instrument_id))?;

        let payment_amount = self
            .get_coupon_payment_amount(instrument)
            .ok_or(format!("Instrument {:?} is not a bond, no coupon payment.", instrument_id))?;

        let available_funds = state.financial_system.get_liquid_assets(&instrument.debtor);
        if available_funds < payment_amount {
            return Err(format!(
                "Insufficient funds for coupon payment: agent {:?} needs ${:.2}, has ${:.2}",
                instrument.debtor, payment_amount, available_funds
            ));
        }
        Ok(())
    }

    fn create_payment_effects(&self, from: AgentId, to: AgentId, amount: f64, state: &SimState) -> Vec<StateEffect> {
        let mut effects = vec![];
        let cb_id = state.financial_system.central_bank.id;
        if let Some(from_bs) = state.financial_system.get_bs_by_id(&from) {
            if let Some((cash_inst_id, cash_inst)) =
                from_bs.assets.iter().find(|(_, inst)| inst.details.as_any().is::<CashDetails>())
            {
                let new_principal = cash_inst.principal - amount;
                if new_principal < 1e-6 {
                    effects.push(StateEffect::Financial(FinancialEffect::RemoveInstrument(*cash_inst_id)));
                } else {
                    effects.push(StateEffect::Financial(FinancialEffect::UpdateInstrument {
                        id: *cash_inst_id,
                        new_principal,
                    }));
                }
                let new_cash_for_to = cash!(to, amount, cb_id, state.current_date);
                effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(new_cash_for_to)));
            }
        }
        effects
    }

    pub fn execute(&self, action: &SettlementAction, state: &SimState) -> SettlementResult {
        if let Err(e) = self.validate(action, state) {
            return SettlementResult { success: false, effects: vec![], errors: vec![e] };
        }

        match action {
            SettlementAction::AccrueInterest { instrument_id } => self.execute_accrue_interest(instrument_id, state),
            SettlementAction::PayInterest { instrument_id } => self.execute_pay_interest(instrument_id, state),
            SettlementAction::ProcessCouponPayment { instrument_id } => {
                self.execute_process_coupon_payment(instrument_id, state)
            }
        }
    }

    fn calculate_daily_interest_accrual(
        &self, instrument: &FinancialInstrument, current_date: chrono::NaiveDate,
    ) -> f64 {
        if current_date <= instrument.last_accrual_date {
            return 0.0;
        }

        let (annual_rate_bps, day_count) = if let Some(deposit) = instrument.details.as_any().downcast_ref::<DemandDepositDetails>() {
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
            current_date
        )
    }

    fn execute_accrue_interest(&self, instrument_id: &InstrumentId, state: &SimState) -> SettlementResult {
        if let Some(instrument) = state.financial_system.instruments.get(instrument_id) {
            let accrued_amount = self.calculate_daily_interest_accrual(instrument, state.current_date);
            if accrued_amount > 1e-6 {
                let effect = StateEffect::Financial(FinancialEffect::AccrueInterest {
                    instrument_id: *instrument_id,
                    accrued_amount,
                    accrual_date: state.current_date,
                });
                SettlementResult { success: true, effects: vec![effect], errors: vec![] }
            } else {
                println!("[DEBUG] Not creating effects ");
                SettlementResult { success: true, effects: vec![], errors: vec![] }
            }
        } else {
            SettlementResult { success: false, effects: vec![], errors: vec!["Instrument not found".to_string()] }
        }
    }

    fn execute_pay_interest(&self, instrument_id: &InstrumentId, state: &SimState) -> SettlementResult {
        if let Some(instrument) = state.financial_system.instruments.get(instrument_id) {
            let interest_amount = instrument.accrued_interest;
            if interest_amount <= 1e-6 {
                return SettlementResult { success: true, effects: vec![], errors: vec![] };
            }
            let mut effects =
                self.create_payment_effects(instrument.debtor, instrument.creditor, interest_amount, state);
            effects
                .push(StateEffect::Financial(FinancialEffect::ResetAccruedInterest { instrument_id: *instrument_id }));
            SettlementResult { success: true, effects, errors: vec![] }
        } else {
            SettlementResult { success: false, effects: vec![], errors: vec!["Instrument not found".to_string()] }
        }
    }

    fn execute_process_coupon_payment(&self, instrument_id: &InstrumentId, state: &SimState) -> SettlementResult {
        if let Some(instrument) = state.financial_system.instruments.get(instrument_id) {
            if let Some(payment_amount) = self.get_coupon_payment_amount(instrument) {
                if payment_amount <= 1e-6 {
                    return SettlementResult { success: true, effects: vec![], errors: vec![] };
                }
                let effects =
                    self.create_payment_effects(instrument.debtor, instrument.creditor, payment_amount, state);
                SettlementResult { success: true, effects, errors: vec![] }
            } else {
                SettlementResult {
                    success: false,
                    effects: vec![],
                    errors: vec!["Instrument is not a bond".to_string()],
                }
            }
        } else {
            SettlementResult { success: false, effects: vec![], errors: vec!["Instrument not found".to_string()] }
        }
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
use sim_macros::SimDomain;

#[derive(Clone, Debug, Serialize, Deserialize, SimDomain)]
pub struct TradingDomain {}

#[derive(Debug, Clone)]
pub struct TradingResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl TradingDomain {
    pub fn new() -> Self {
        Self {}
    }

    pub fn can_handle(&self, action: &TradingAction) -> bool {
        matches!(action, TradingAction::PostBid { .. } | TradingAction::PostAsk { .. })
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
        &self, agent_id: AgentId, market_id: &MarketId, quantity: f64, state: &SimState,
    ) -> Result<(), String> {
        Validator::positive_amount(quantity)?;

        if !state.financial_system.balance_sheets.contains_key(&agent_id) {
            return Err(format!("Asking agent {:?} not found", agent_id));
        }
        let bs = state.financial_system.balance_sheets.get(&agent_id).unwrap();

        match market_id {
            MarketId::Goods(good_id) => {
                let available_inventory =
                    bs.get_inventory().and_then(|inv| inv.get(good_id)).map_or(0.0, |item| item.quantity);

                if available_inventory < quantity {
                    return Err(format!(
                        "Insufficient inventory for ask: agent {:?} needs {:.2}, has {:.2}",
                        agent_id, quantity, available_inventory
                    ));
                }
            }
            MarketId::Financial(fin_market_id) => match fin_market_id {
                FinancialMarketId::FederalFundsOvernight => {
                    let reserves = state.financial_system.get_bank_reserves(&agent_id).unwrap_or(0.0);
                    if reserves < quantity {
                        return Err(format!(
                            "Insufficient reserves for federal funds ask (lending): agent {:?} needs ${:.2}, has ${:.2}",
                            agent_id, quantity, reserves
                        ));
                    }
                }
                FinancialMarketId::TreasuryRepoOvernight => {
                    let reserves = state.financial_system.get_bank_reserves(&agent_id).unwrap_or(0.0);
                    if reserves < quantity {
                        return Err(format!(
                            "Insufficient reserves for Treasury repo ask (lending): agent {:?} needs ${:.2}, has ${:.2}",
                            agent_id, quantity, reserves
                        ));
                    }
                }
                FinancialMarketId::Treasury { tenor } => {
                    let held_quantity = bs
                        .assets
                        .values()
                        .map(|inst| {
                            if let Some(bond_details) = inst.details.as_any().downcast_ref::<BondDetails>() {
                                if bond_details.bond_type == BondType::Government && bond_details.tenor == *tenor {
                                    bond_details.quantity as f64
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            }
                        })
                        .sum::<f64>();
                    if held_quantity < quantity {
                        return Err(format!(
                            "Insufficient Treasury holdings ({:?}) for ask: agent {:?} needs {:.0}, has {:.0}",
                            tenor, agent_id, quantity, held_quantity
                        ));
                    }
                }
                FinancialMarketId::CorporateBond { .. } 
                | FinancialMarketId::DiscountWindow 
                | FinancialMarketId::StandingRepoFacility 
                | FinancialMarketId::OvernightReverseRepo => {
                }
            },
            MarketId::Labour(_) => {}
        }

        Ok(())
    }

    pub fn execute(&self, action: &TradingAction, state: &SimState) -> TradingResult {
        if let Err(error) = self.validate(action, state) {
            return TradingResult { success: false, effects: vec![], errors: vec![error] };
        }

        match action {
            TradingAction::PostBid { agent_id, market_id, quantity, price } => {
                self.execute_post_bid(*agent_id, market_id.clone(), *quantity, *price)
            }
            TradingAction::PostAsk { agent_id, market_id, quantity, price } => {
                self.execute_post_ask(*agent_id, market_id.clone(), *quantity, *price)
            }
        }
    }

    pub fn execute_post_bid(&self, agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64) -> TradingResult {
        let effects = vec![StateEffect::Market(MarketEffect::PlaceOrderInBook {
            market_id,
            order: Order::Bid(Bid { agent_id, quantity, price }),
        })];

        TradingResult { success: true, effects, errors: vec![] }
    }

    pub fn execute_post_ask(&self, agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64) -> TradingResult {
        let effects = vec![StateEffect::Market(MarketEffect::PlaceOrderInBook {
            market_id,
            order: Order::Ask(Ask { agent_id, quantity, price }),
        })];

        TradingResult { success: true, effects, errors: vec![] }
    }
}

impl Default for TradingDomain {
    fn default() -> Self {
        Self::new()
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

