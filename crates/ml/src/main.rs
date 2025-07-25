use std::env;
mod process;
use process::*;
mod train;
use train::{train_model, TrainingData};

const CE_MODEL_PATH: &str = "ce_trained_model.bin";

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("_");
    let dir = args.get(2).map(|s| s.as_str()).unwrap_or("crates/ml/data");
    match mode {
        "train" => train_ce_model(dir),
        "predict" => test_predictions(),
        "validate" => validate_model(),
        _ => {
            println!("Please specify a mode: train, predict, or validate");
        }
    }
}


fn train_ce_model(data_dir: &str) {
    println!("=== Training Consumer Model on Real CE Data ===\n");
    
    let raw_df = match extract_ce_data(data_dir) {
        Ok(df) => df,
        Err(e) => {
            eprintln!("Error loading data: {}", e);
            return;
        }
    };
    
    let processed_df = match engineer_additional_features(raw_df) {
        Ok(df) => df,
        Err(e) => {
            eprintln!("Error engineering features: {}", e);
            return;
        }
    };
    
    let training_data = match TrainingData::from_dataframe(&processed_df) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error creating training data: {}", e);
            return;
        }
    };
    
    let model = match train_model(&training_data) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error training model: {}", e);
            return;
        }
    };
    
    println!("\n--- Saving trained model ---");
    match model.save_to_file(CE_MODEL_PATH) {
        Ok(_) => println!("Model saved to '{}'", CE_MODEL_PATH),
        Err(e) => eprintln!("Error saving model: {}", e),
    }
    
    println!("\n--- Feature Summary ---");
    for (i, name) in model.feature_names.iter().enumerate() {
        println!("{:>20}: μ={:>6.2}, σ={:>6.2}", 
            name, 
            model.feature_means[i],
            model.feature_stds[i]
        );
    }
}

fn test_predictions() {
    use ndarray::Array1;
    
    println!("=== Loading Model and Making Predictions ===\n");
    
    let model = match ConsumerDecisionModel::load_from_file(CE_MODEL_PATH) {
        Ok(m) => {
            println!("Model loaded successfully from '{}'", CE_MODEL_PATH);
            println!("Single-stage regression model trained on {} features", m.feature_names.len());
            m
        }
        Err(e) => {
            eprintln!("Failed to load model: {}", e);
            eprintln!("Please run 'train' mode first");
            return;
        }
    };
    
    // Helper to calculate income bracket
    let income_bracket = |income: f64| -> f64 {
        if income < 30000.0 { 1.0 }
        else if income < 50000.0 { 2.0 }
        else if income < 75000.0 { 3.0 }
        else if income < 100000.0 { 4.0 }
        else if income < 150000.0 { 5.0 }
        else { 6.0 }
    };
    
    // Test cases covering wide income range
    let test_cases = vec![
        (
            "Poverty line (single)", 
            20000.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            0.25, 0.40, 0.15, 0.08
        ),
        (
            "Young low-income", 
            35000.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            0.20, 0.35, 0.15, 0.10
        ),
        (
            "Middle-age moderate", 
            60000.0, 2.0, 3.0, 1.0, 2.0, 0.0, 1.0, 0.67, 2.0,
            0.15, 0.30, 0.18, 0.12
        ),
        (
            "Upper middle class", 
            90000.0, 2.0, 2.0, 0.0, 3.0, 0.0, 1.0, 1.0, 1.0,
            0.12, 0.28, 0.20, 0.13
        ),
        (
            "High income professional", 
            150000.0, 2.0, 2.0, 0.0, 4.0, 0.0, 1.0, 1.0, 3.0,
            0.10, 0.25, 0.22, 0.15
        ),
        (
            "Very high income", 
            250000.0, 3.0, 3.0, 1.0, 4.0, 0.0, 1.0, 1.0, 4.0,
            0.08, 0.22, 0.25, 0.18
        ),
    ];
    
    println!("{:<28} | {:<12} | {:<20} | {:<15}", 
        "Profile", "Income", "Predicted Spending", "Spending Rate");
    println!("{}", "-".repeat(85));
    
    for (name, income, age_group, family_size, has_children, education, 
         housing_status, is_urban, earner_ratio, region,
         food_share, housing_share, transport_share, health_share) in test_cases {
        
        let log_income = (income as f64).max(1000.0).ln();
        let income_bracket_val = income_bracket(income);
        
        let features = vec![
            income,                              // 0
            log_income,                          // 1
            age_group,                           // 2
            family_size,                         // 3
            has_children,                        // 4
            education,                           // 5
            housing_status,                      // 6
            is_urban,                            // 7
            earner_ratio,                        // 8
            region,                              // 9
            food_share,                          // 10
            housing_share,                       // 11
            transport_share,                     // 12
            health_share,                        // 13
            income_bracket_val * age_group,      // 14: income_age_interaction
            income_bracket_val * education,      // 15: income_education
        ];
        
        let features_array = Array1::from(features);
        let predicted = model.predict(&features_array);
        let spending_rate = predicted / income;
        
        println!("{:<28} | ${:>10.0} | ${:>18.2} | {:>13.1}%", 
            name, 
            income, 
            predicted,
            spending_rate * 100.0
        );
    }
    
    println!("\n--- Income Sensitivity Test ---");
    println!("Testing same profile with different incomes:");
    println!("{:<12} | {:<20} | {:<15}", "Income", "Predicted Spending", "Spending Rate");
    println!("{}", "-".repeat(50));
    
    // Same profile, different incomes
    for income in &[20000.0, 40000.0, 60000.0, 80000.0, 100000.0, 150000.0] {
        let log_income = (income as &f64).max(1000.0).ln();
        let income_bracket_val = income_bracket(*income);
        let age_group = 2.0;
        let education = 2.0;
        
        let features = vec![
            *income, log_income, age_group, 2.0, 0.0, education, 0.0, 1.0, 1.0, 1.0,
            0.15, 0.30, 0.18, 0.12,
            income_bracket_val * age_group,
            income_bracket_val * education,
        ];
        
        let features_array = Array1::from(features);
        let predicted = model.predict(&features_array);
        let spending_rate = predicted / income;
        
        println!("${:>10.0} | ${:>18.2} | {:>13.1}%", 
            income, predicted, spending_rate * 100.0);
    }
    
    println!("\nNote: Single-stage model trained on all {} CE survey households.", 12401);
}
fn validate_model() {
    println!("=== Validating Model Behavior with Controlled Inputs ===");

    use ndarray::Array1;

    let model = match ConsumerDecisionModel::load_from_file(CE_MODEL_PATH) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load model: {}", e);
            return;
        }
    };
    
    // Print model info
    println!("\n--- Model Information ---");
    println!("Number of features: {}", model.feature_names.len());
    println!("Feature names: {:?}", model.feature_names);
    println!("Spending threshold: ${:.2}", model.spending_threshold);
    
    // Helper function to calculate income bracket
    let income_bracket = |income: f64| -> f64 {
        if income < 30000.0 { 1.0 }
        else if income < 50000.0 { 2.0 }
        else if income < 75000.0 { 3.0 }
        else if income < 100000.0 { 4.0 }
        else if income < 150000.0 { 5.0 }
        else { 6.0 }
    };
    
    let predict = |name: &str, features: Vec<f64>| {
        let features_array = Array1::from(features.clone());
        let predicted = model.predict(&features_array);
        let income = features[0];
        let spending_rate = if income > 0.0 { predicted / income } else { 0.0 };
        println!("{:<35} | ${:>10.0} | ${:>18.2} | {:>15.1}%", 
            name, 
            income, 
            predicted,
            spending_rate * 100.0
        );
    };

    println!("\n{:<35} | {:<12} | {:<20} | {:<15}", 
        "Profile", "Income", "Predicted Spending", "Spending Rate");
    println!("{}", "-".repeat(95));

    // --- Test 1: Effect of Age Group (Income = $75k) ---
    println!("\n--- Test 1: Effect of Age Group (Income = $75k) ---");
    let income_t1 = 75000.0;
    let log_income_t1 = (income_t1 as f64).max(1000.0).ln();
    let income_bracket_t1 = income_bracket(income_t1);
    let education_t1 = 2.0; // Assume "Some College"
    
    let mut base_profile_t1 = vec![
        income_t1,               // 0: income
        log_income_t1,           // 1: log_income
        0.0,                     // 2: age_group (will be set)
        2.0,                     // 3: family_size
        0.0,                     // 4: has_children
        education_t1,            // 5: education
        0.0,                     // 6: housing_status
        1.0,                     // 7: is_urban
        1.0,                     // 8: earner_ratio
        1.0,                     // 9: region
        0.15,                    // 10: food_share
        0.3,                     // 11: housing_share
        0.2,                     // 12: transport_share
        0.1,                     // 13: health_share
        0.0,                     // 14: income_age_interaction (will be set)
        income_bracket_t1 * education_t1  // 15: income_education
    ];
    
    let age_group_1 = 1.0;
    base_profile_t1[2] = age_group_1;
    base_profile_t1[14] = income_bracket_t1 * age_group_1; // income_age_interaction
    predict("Young Adult (Group 1)", base_profile_t1.clone());
    
    let age_group_2 = 2.0;
    base_profile_t1[2] = age_group_2;
    base_profile_t1[14] = income_bracket_t1 * age_group_2;
    predict("Prime Age (Group 2)", base_profile_t1.clone());

    let age_group_4 = 4.0;
    base_profile_t1[2] = age_group_4;
    base_profile_t1[14] = income_bracket_t1 * age_group_4;
    predict("Senior (Group 4)", base_profile_t1.clone());

    // --- Test 2: Effect of Family Size (Income = $90k) ---
    println!("\n--- Test 2: Effect of Family Size (Income = $90k, Prime Age) ---");
    let income_t2 = 90000.0;
    let log_income_t2 = (income_t2 as f64).max(1000.0).ln();
    let income_bracket_t2 = income_bracket(income_t2);
    let age_group_t2 = 2.0;
    let education_t2 = 3.0; // Assume "Bachelor's"
    
    let mut base_profile_t2 = vec![
        income_t2,               // 0: income
        log_income_t2,           // 1: log_income
        age_group_t2,            // 2: age_group
        0.0,                     // 3: family_size (will be set)
        0.0,                     // 4: has_children (will be set)
        education_t2,            // 5: education
        0.0,                     // 6: housing_status
        1.0,                     // 7: is_urban
        1.0,                     // 8: earner_ratio
        1.0,                     // 9: region
        0.15,                    // 10: food_share
        0.3,                     // 11: housing_share
        0.2,                     // 12: transport_share
        0.1,                     // 13: health_share
        income_bracket_t2 * age_group_t2,     // 14: income_age_interaction
        income_bracket_t2 * education_t2      // 15: income_education
    ];

    base_profile_t2[3] = 1.0;
    base_profile_t2[4] = 0.0;
    predict("Single person", base_profile_t2.clone());
    
    base_profile_t2[3] = 2.0;
    base_profile_t2[4] = 0.0;
    predict("Couple, no kids", base_profile_t2.clone());
    
    base_profile_t2[3] = 4.0;
    base_profile_t2[4] = 1.0;
    predict("Family of 4 with kids", base_profile_t2.clone());

    // --- Test 3: Edge Case (Very Low Income) ---
    println!("\n--- Test 3: Edge Case (Very Low Income) ---");
    let income_t3 = 15000.0;
    let log_income_t3 = (income_t3 as f64).max(1000.0).ln();
    let income_bracket_t3 = income_bracket(income_t3);
    let age_group_t3 = 1.0;
    let education_t3 = 1.0; // Assume "HS Grad"
    
    let profile_t3 = vec![
        income_t3,               // 0: income
        log_income_t3,           // 1: log_income
        age_group_t3,            // 2: age_group
        1.0,                     // 3: family_size
        0.0,                     // 4: has_children
        education_t3,            // 5: education
        1.0,                     // 6: housing_status (renting)
        1.0,                     // 7: is_urban
        1.0,                     // 8: earner_ratio
        1.0,                     // 9: region
        0.25,                    // 10: food_share
        0.45,                    // 11: housing_share
        0.2,                     // 12: transport_share
        0.05,                    // 13: health_share
        income_bracket_t3 * age_group_t3,     // 14: income_age_interaction
        income_bracket_t3 * education_t3      // 15: income_education
    ];
    predict("Struggling (Income $15k)", profile_t3);

    // --- Test 4: Effect of Education (Income = $60k) ---
    println!("\n--- Test 4: Effect of Education (Income = $60k) ---");
    let income_t4 = 60000.0;
    let log_income_t4 = (income_t4 as f64).max(1000.0).ln();
    let income_bracket_t4 = income_bracket(income_t4);
    let age_group_t4 = 2.0;
    
    let base_profile_t4 = vec![
        income_t4,               // 0: income
        log_income_t4,           // 1: log_income
        age_group_t4,            // 2: age_group
        2.0,                     // 3: family_size
        0.0,                     // 4: has_children
        0.0,                     // 5: education (will be set)
        0.0,                     // 6: housing_status
        1.0,                     // 7: is_urban
        1.0,                     // 8: earner_ratio
        1.0,                     // 9: region
        0.15,                    // 10: food_share
        0.3,                     // 11: housing_share
        0.2,                     // 12: transport_share
        0.1,                     // 13: health_share
        income_bracket_t4 * age_group_t4,     // 14: income_age_interaction
        0.0                      // 15: income_education (will be set)
    ];
    
    let update_and_predict = |name: &str, edu_level: f64| {
        let mut profile = base_profile_t4.clone();
        profile[5] = edu_level; // Set education level
        profile[15] = income_bracket_t4 * edu_level; // Set income_education interaction
        predict(name, profile);
    };

    update_and_predict("Less than High School (Income $60k)", 0.0);
    update_and_predict("High School Grad (Income $60k)", 1.0);
    update_and_predict("Some College (Income $60k)", 2.0);
    update_and_predict("Bachelor's Degree (Income $60k)", 3.0);
    update_and_predict("Graduate Degree (Income $60k)", 4.0);
    
    // --- Test 5: Effect of spending shares ---
    println!("\n--- Test 5: Effect of Spending Patterns (Income = $80k) ---");
    let income_t5 = 80000.0;
    let log_income_t5 = (income_t5 as f64).max(1000.0).ln();
    let income_bracket_t5 = income_bracket(income_t5);
    let age_group_t5 = 2.0;
    let education_t5 = 2.0;
    
    let mut base_profile_t5 = vec![
        income_t5,               // 0: income
        log_income_t5,           // 1: log_income
        age_group_t5,            // 2: age_group
        2.0,                     // 3: family_size
        0.0,                     // 4: has_children
        education_t5,            // 5: education
        0.0,                     // 6: housing_status
        1.0,                     // 7: is_urban
        1.0,                     // 8: earner_ratio
        1.0,                     // 9: region
        0.0,                     // 10-13: spending shares (will be set)
        0.0, 0.0, 0.0,
        income_bracket_t5 * age_group_t5,     // 14: income_age_interaction
        income_bracket_t5 * education_t5      // 15: income_education
    ];
    
    // High housing cost
    base_profile_t5[10] = 0.10; // food
    base_profile_t5[11] = 0.45; // housing (high)
    base_profile_t5[12] = 0.15; // transport
    base_profile_t5[13] = 0.10; // health
    predict("High housing cost", base_profile_t5.clone());
    
    // High transport cost
    base_profile_t5[10] = 0.15; // food
    base_profile_t5[11] = 0.25; // housing
    base_profile_t5[12] = 0.30; // transport (high)
    base_profile_t5[13] = 0.10; // health
    predict("High transport cost", base_profile_t5.clone());
    
    println!("\n--- IMPORTANT ---");
    println!("Model must be retrained after removing savings features!");
    println!("Run: cargo run -- train");
}