use crate::*;
use dyn_clone::{clone_trait_object, DynClone};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;
use ndarray::Array1;

#[typetag::serde(tag = "type")]
pub trait DecisionModel: DynClone + Send + Sync {
    fn decide(&self, agent: &dyn Any, state: &SimState, rng: &mut dyn RngCore) -> Vec<SimAction>;
}

clone_trait_object!(DecisionModel);

impl Debug for dyn DecisionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecisionModel")
    }
}

// TODO reintgrate ML decision model once basic economy loop working
pub trait FeatureSource {
    fn get_age(&self) -> u32;
    fn get_income(&self) -> f64;
    fn get_savings(&self) -> f64;
    fn get_debt(&self) -> f64;
    fn get_family_size(&self) -> u32 { 1 }
    fn get_has_children(&self) -> bool { false }
    fn get_education_level_numeric(&self) -> u32 { 2 }
    fn get_housing_status_numeric(&self) -> u32 { 0 }
    fn get_is_urban(&self) -> bool { true }
    fn get_region_numeric(&self) -> u32 { 1 }
}

pub trait SpendingPredictor: DynClone + Send + Sync {
    fn predict_spending(&self, features: &Array1<f64>) -> f64;
    fn get_feature_names(&self) -> &[String];
}

clone_trait_object!(SpendingPredictor);

impl Debug for dyn SpendingPredictor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SpendingPredictor")
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
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimAction> {
        let consumer = match agent.downcast_ref::<Consumer>() {
            Some(c) => c,
            None => return vec![],
        };

        if let Some(predictor) = &self.predictor {
            let fs = &state.financial_system;
            let features = extract_consumer_features(consumer, fs);
            let predicted_annual_spending = predictor.predict_spending(&features);

            let cash_holdings = fs.get_cash_assets(&consumer.id);
            let weekly_income = consumer.income / 52.0;
            let total_available = weekly_income + cash_holdings;
            let spending_per_period = predicted_annual_spending / 52.0;
            let spend_amount = spending_per_period.min(total_available);
            let save_amount = total_available - spend_amount;

            let mut actions = Vec::new();
            let good_to_buy = good_id!("petrol");

            if let Some(seller) = fs.exchange.goods_market(&good_to_buy).and_then(|m| m.best_ask()) {
                if spend_amount > 0.0 {
                    actions.push(SimAction::Consumption(ConsumptionAction::Purchase {
                        agent_id: consumer.id,
                        seller: seller.agent_id,
                        amount: spend_amount,
                        good_id: good_to_buy,
                    }));
                }
            }
            if save_amount > 0.0 {
                actions.push(SimAction::Banking(BankingAction::Deposit {
                    agent_id: consumer.id,
                    bank: consumer.bank_id,
                    amount: save_amount
                }));
            }
            actions
        } else {
            vec![SimAction::Consumption(ConsumptionAction::NoAction {
                agent_id: consumer.id,
            })]
        }
    }
}


fn extract_consumer_features(consumer: &Consumer, _fs: &FinancialSystem) -> Array1<f64> {
    let income = consumer.income;
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

    let education = 2.0;
    let family_size = 1.0;
    let has_children = false;
    let housing_status = 0.0;
    let is_urban = true;
    let region = 1.0;

    Array1::from(vec![
        income,
        log_income,
        age_group,
        family_size,
        if has_children { 1.0 } else { 0.0 },
        education,
        housing_status,
        if is_urban { 1.0 } else { 0.0 },
        1.0,
        region,
        food_share,
        housing_share,
        transport_share,
        health_share,
        income_bracket * age_group,
        income_bracket * education,
    ])
}