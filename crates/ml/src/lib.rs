use sim_prelude::*;
use ndarray::Array1;
use std::error::Error;

mod process;
mod train;

pub use process::{ConsumerDecisionModel, extract_ce_data, engineer_additional_features};
pub use train::{train_model, TrainingData};

impl SpendingPredictor for ConsumerDecisionModel {
    fn predict_spending(&self, features: &Array1<f64>) -> f64 {
        self.predict(features)
    }
    
    fn get_feature_names(&self) -> &[String] {
        &self.feature_names
    }
}

pub fn create_ml_decision_model(model_path: &str) -> Result<Box<dyn DecisionModel>, Box<dyn Error>> {
    let model = ConsumerDecisionModel::load_from_file(model_path)?;
    let predictor = Box::new(model) as Box<dyn SpendingPredictor>;
    
    Ok(Box::new(MLDecisionModel {
        predictor: Some(predictor),
        model_path: model_path.to_string(),
    }))
}