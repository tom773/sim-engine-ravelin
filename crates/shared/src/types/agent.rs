use serde::{Serialize, Deserialize};
use crate::{Consumer, FinancialSystem, SpendingPredictor, FeatureSource};
use rand::{rngs::StdRng, Rng, RngCore};
use dyn_clone::{clone_trait_object, DynClone};
use std::fmt::Debug;
use ndarray::Array1;

pub trait Agent {
    fn act(&self, decision: &Decision) -> Action;
    fn decide(&self, fs: &FinancialSystem, rng: &mut StdRng) -> Decision;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    Buy { good_id: String, quantity: u32 },
    Sell { good_id: String, quantity: u32 },
    Save,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Decision {
    Spend { amount: f64 },
    Save,
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
    pub wdf: f64, 
}

#[typetag::serde]
impl DecisionModel for BasicDecisionModel {
    fn decide(&self, consumer: &Consumer, _fs: &FinancialSystem, rng: &mut dyn RngCore) -> Decision {
        let u_spend = self.wdf * rng.random_range(0.0..1.0);
        let u_save = self.wdf * rng.random_range(0.0..1.0); 
        
        if u_spend > u_save {
            Decision::Spend { amount: consumer.income * consumer.propensity_to_consume }
        } else {
            Decision::Save
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
            
            println!("Consumer ID {:?} Assets: ${:.2} | Income: ${:.2} | Liabilities: ${:.2}",
                consumer.id, 
                fs.balance_sheets.get(&consumer.id).map(|bs| bs.total_assets()).unwrap_or(0.0),
                consumer.income, 
                fs.balance_sheets.get(&consumer.id).map(|bs| bs.total_liabilities()).unwrap_or(0.0)
            );
            
            let spending_rate = if consumer.income > 0.0 {
                (predicted_annual_spending / consumer.income).min(1.0)
            } else {
                0.0
            };
            
            println!("Predicted annual spending: ${:.2} ({:.1}% of income)", 
                predicted_annual_spending, 
                spending_rate * 100.0
            );
            
            let spending_per_tick = predicted_annual_spending / 12.0;
            
            Decision::Spend { amount: spending_per_tick }
        } else {

            Decision::Save
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