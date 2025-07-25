
use serde::{Serialize, Deserialize};
use crate::{Consumer, FinancialSystem, SpendingPredictor, FeatureSource};
use rand::{rngs::StdRng, RngCore};
use dyn_clone::{clone_trait_object, DynClone};
use std::fmt::Debug;
use ndarray::Array1;

pub trait Agent {
    fn act(&self, decision: &Decision) -> Vec<Action>;
    fn decide(&self, fs: &FinancialSystem, rng: &mut StdRng) -> Decision;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    DepositCash { amount: f64 },
    WithdrawCash { amount: f64 },
    Buy { good_id: String, quantity: u32, amount: f64 },
    Sell { good_id: String, quantity: u32, amount: f64 },
    ReceiveIncome { amount: f64 },
}
impl Action {
    pub fn name(&self) -> String {
        match self {
            Action::DepositCash { .. } => "Deposit Cash".to_string(),
            Action::WithdrawCash { .. } => "Withdraw Cash".to_string(),
            Action::Buy { .. } => "Buy Good".to_string(),
            Action::Sell { .. } => "Sell Good".to_string(),
            Action::ReceiveIncome { .. } => "Receive Income".to_string(),
        }
    }
    pub fn amount(&self) -> f64 {
        match self {
            Action::DepositCash { amount } => *amount,
            Action::WithdrawCash { amount } => *amount,
            Action::Buy { amount, .. } => *amount,
            Action::Sell { amount, .. } => *amount,
            Action::ReceiveIncome { amount } => *amount,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decision {
    pub spend_amount: f64,
    pub save_amount: f64,
    pub total_available: f64, // Income + existing cash
}

#[typetag::serde(tag = "type")]
pub trait DecisionModel: DynClone + Send + Sync {
    fn decide(&self, consumer: &Consumer, fs: &FinancialSystem, rng: &mut dyn RngCore) -> Decision;
}

clone_trait_object!(DecisionModel);

impl Debug for dyn DecisionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecisionModel")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasicDecisionModel {
    pub propensity_to_consume: f64, // 0.0 to 1.0
}

#[typetag::serde]
impl DecisionModel for BasicDecisionModel {
    fn decide(&self, consumer: &Consumer, fs: &FinancialSystem, _rng: &mut dyn RngCore) -> Decision {
        let cash_holdings = fs.balance_sheets
            .get(&consumer.id)
            .map(|bs| bs.assets.values()
                .filter(|inst| matches!(inst.instrument_type, crate::InstrumentType::Cash))
                .map(|inst| inst.principal)
                .sum::<f64>()
            )
            .unwrap_or(0.0);
        
        let total_available = consumer.income + cash_holdings;
        let spend_amount = total_available * self.propensity_to_consume;
        let save_amount = total_available * (1.0 - self.propensity_to_consume);
        
        Decision {
            spend_amount,
            save_amount,
            total_available,
        }
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
    fn decide(&self, consumer: &Consumer, fs: &FinancialSystem, _rng: &mut dyn RngCore) -> Decision {
        if let Some(predictor) = &self.predictor {
            let features = extract_consumer_features(consumer, fs);
            let predicted_annual_spending = predictor.predict_spending(&features);
            
            let cash_holdings = fs.balance_sheets
                .get(&consumer.id)
                .map(|bs| bs.assets.values()
                    .filter(|inst| matches!(inst.instrument_type, crate::InstrumentType::Cash))
                    .map(|inst| inst.principal)
                    .sum::<f64>()
                )
                .unwrap_or(0.0);
            
            let total_available = consumer.income + cash_holdings;
            let spending_per_period = predicted_annual_spending / 12.0; // Monthly
            let spend_amount = spending_per_period.min(total_available);
            let save_amount = total_available - spend_amount;
            
            println!("ML Model Decision: Available ${:.2}, Spend ${:.2}, Save ${:.2}", 
                total_available, spend_amount, save_amount);
            
            Decision {
                spend_amount,
                save_amount,
                total_available,
            }
        } else {
            let basic = BasicDecisionModel { propensity_to_consume: 0.7 };
            basic.decide(consumer, fs, _rng)
        }
    }
}

fn extract_consumer_features(consumer: &Consumer, _fs: &FinancialSystem) -> Array1<f64> {
    let income = consumer.get_income();
    let log_income = income.max(1000.0).ln();
    
    let income_bracket = if income < 30000.0 { 1.0 }
                        else if income < 50000.0 { 2.0 }
                        else if income < 75000.0 { 3.0 }
                        else if income < 100000.0 { 4.0 }
                        else if income < 150000.0 { 5.0 }
                        else { 6.0 };

    let food_share = 0.15;
    let housing_share = 0.30;
    let transport_share = 0.20;
    let health_share = 0.10;

    let age = consumer.get_age();
    let age_group = if age < 35 { 1.0 } 
                    else if age < 55 { 2.0 } 
                    else if age < 65 { 3.0 } 
                    else { 4.0 };

    let education = consumer.get_education_level_numeric() as f64;
    
    Array1::from(vec![
        income,
        log_income,
        age_group,
        consumer.get_family_size() as f64,
        if consumer.get_has_children() { 1.0 } else { 0.0 },
        education,
        consumer.get_housing_status_numeric() as f64,
        if consumer.get_is_urban() { 1.0 } else { 0.0 },
        1.0,
        consumer.get_region_numeric() as f64,
        food_share,
        housing_share,
        transport_share,
        health_share,
        income_bracket * age_group,
        income_bracket * education,
    ])
}