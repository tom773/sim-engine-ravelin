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