use ndarray::{Array1, Array2, Axis};
use linfa::prelude::*;
use linfa_logistic::LogisticRegression;
use lightgbm3::{Booster, Dataset as LightGbmDataset};
use polars::prelude::*;
use std::error::Error;
use crate::process::{column_to_array, dataframe_to_features, ConsumerDecisionModel};

pub struct TrainingData {
    pub features: Array2<f64>,
    pub feature_names: Vec<String>,
    pub spending_amounts: Array1<f64>,
}

impl TrainingData {
    pub fn from_dataframe(df: &DataFrame) -> Result<Self, Box<dyn Error>> {
        let feature_cols = vec![
            "income",                    // Keep raw for precision
            "log_income",               // Add log for better distribution
            "age_group", 
            "family_size", 
            "has_children", 
            "education", 
            "housing_status", 
            "is_urban", 
            "earner_ratio", 
            "region", 
            "food_share", 
            "housing_share", 
            "transport_share", 
            "health_share", 
            "income_age_interaction",   // Using brackets
            "income_education",         // Using brackets
        ];
        
        let (features, feature_names) = dataframe_to_features(df, &feature_cols)?;
        let spending_amounts = column_to_array(df.column("annual_spending")?)?;
        
        println!("Training data summary:");
        println!("  - {} samples", features.nrows());
        println!("  - {} features", features.ncols());
        println!("  - Mean spending: ${:.2}", spending_amounts.mean().unwrap());
        println!("  - Spending range: ${:.0} to ${:.0}", 
            spending_amounts.iter().cloned().fold(f64::INFINITY, f64::min),
            spending_amounts.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        );
        
        Ok(TrainingData {
            features,
            feature_names,
            spending_amounts,
        })
    }
}

pub fn train_model(training_data: &TrainingData) -> Result<ConsumerDecisionModel, Box<dyn Error>> {
    println!("\n=== Training Single-Stage LightGBM Model on ALL Data ===");
    println!("Using all {} samples for regression training", training_data.features.nrows());
    
    let (features_norm, feature_means, feature_stds) = normalize_features_with_stats(&training_data.features);
    
    let targets_log: Vec<f32> = training_data.spending_amounts
        .iter()
        .map(|&amount| amount.max(1.0).ln() as f32)
        .collect();
    
    let split_idx = (features_norm.nrows() as f64 * 0.8) as usize;
    let (train_features, valid_features) = features_norm.view().split_at(Axis(0), split_idx);
    let (train_targets, _valid_targets) = targets_log.split_at(split_idx);
    let (_, valid_amounts) = training_data.spending_amounts.view().split_at(Axis(0), split_idx);
    
    let dtrain = LightGbmDataset::from_slice(
        train_features.as_slice().unwrap(),
        train_targets,
        train_features.ncols() as i32,
        true,
    )?;
    
    let params = serde_json::json!({
        "objective": "regression",
        "metric": "rmse",
        "boosting_type": "gbdt",
        "learning_rate": 0.01,
        "num_leaves": 255,           // More complexity for nuanced patterns
        "min_data_in_leaf": 50,     
        "num_iterations": 500,       // More iterations for convergence
        "feature_fraction": 0.8,
        "bagging_fraction": 0.8,
        "bagging_freq": 5,
        "lambda_l1": 0.1,
        "lambda_l2": 0.1,
        "verbosity": -1
    });
    
    println!("Training LightGBM regressor...");
    let booster = Booster::train(dtrain, &params)?;
    
    println!("\n--- Feature Importance (Gain) ---");
    let importance = booster.feature_importance(lightgbm3::ImportanceType::Gain)?;
    let mut importance_pairs: Vec<(String, f64)> = training_data.feature_names.iter()
        .zip(importance.iter())
        .map(|(name, &imp)| (name.clone(), imp))
        .collect();
    importance_pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    for (name, imp) in importance_pairs.iter().take(10) {
        if *imp > 0.0 {
            println!("  {}: {:.1}", name, imp);
        }
    }
    
    let predictions_log_vec = booster.predict(
        valid_features.as_slice().unwrap(),
        valid_features.ncols() as i32,
        true
    )?;
    
    let predictions_log = Array1::from_vec(predictions_log_vec.into_iter().map(|v| v as f64).collect());
    evaluate_regressor(&predictions_log, &valid_amounts.to_owned())?;
    
    let regressor_model_string = booster.save_string()?;
    
    let mut dummy_targets = vec![false; features_norm.nrows()];
    // Set first half to true, second half to false to ensure we have both classes
    for i in 0..features_norm.nrows()/2 {
        dummy_targets[i] = true;
    }
    let dummy_targets_array = Array1::from(dummy_targets);
    let dummy_dataset = Dataset::new(features_norm, dummy_targets_array);
    let classifier = LogisticRegression::default().fit(&dummy_dataset)?;
    
    let mut spending_vec = training_data.spending_amounts.to_vec();
    spending_vec.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let spending_threshold = spending_vec[spending_vec.len() / 2];
    println!("\nMedian spending (for reference): ${:.2}", spending_threshold);
    
    Ok(ConsumerDecisionModel {
        classifier,  // Dummy classifier, not used
        regressor_model_string,
        feature_names: training_data.feature_names.clone(),
        feature_means,
        feature_stds,
        spending_threshold,  // Not used in single-stage model
    })
}

fn normalize_features_with_stats(features: &Array2<f64>) -> (Array2<f64>, Vec<f64>, Vec<f64>) {
    let mut normalized = features.clone();
    let mut means = Vec::new();
    let mut stds = Vec::new();
    
    for col_idx in 0..features.ncols() {
        let column = features.column(col_idx);
        let mean = column.mean().unwrap_or(0.0);
        let std = column.std(0.0);
        
        means.push(mean);
        stds.push(std);
        
        if std > 0.0 {
            normalized.column_mut(col_idx).mapv_inplace(|x| (x - mean) / std);
        }
    }
    
    (normalized, means, stds)
}

fn evaluate_regressor(predictions_log: &Array1<f64>, targets_actual: &Array1<f64>) -> Result<(), Box<dyn Error>> {
    let predictions_dollars = predictions_log.mapv(|x| x.exp());
    let errors = &predictions_dollars - targets_actual;
    let mae_dollars = errors.mapv(|x| x.abs()).mean().unwrap();
    let rmse_dollars = (errors.mapv(|x| x * x).mean().unwrap()).sqrt();
    
    let mape = errors.iter()
        .zip(targets_actual.iter())
        .filter(|&(_, &actual)| actual > 0.0)
        .map(|(&error, &actual)| (error / actual).abs() * 100.0)
        .sum::<f64>() / targets_actual.len() as f64;
    
    println!("\nRegressor Performance (on validation set):");
    println!("  - RMSE: ${:.2}", rmse_dollars);
    println!("  - MAE: ${:.2}", mae_dollars);
    println!("  - MAPE: {:.1}%", mape);
    
    let pred_min = predictions_dollars.iter().cloned().fold(f64::INFINITY, f64::min);
    let pred_max = predictions_dollars.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let pred_mean = predictions_dollars.mean().unwrap();
    
    println!("\nPrediction Distribution:");
    println!("  - Min: ${:.0}", pred_min);
    println!("  - Max: ${:.0}", pred_max);
    println!("  - Mean: ${:.0}", pred_mean);
    
    Ok(())
}


