use polars::prelude::*;
use ndarray::{Array1, Array2};
use std::error::Error;
use serde::{Deserialize, Serialize};
use linfa_logistic::FittedLogisticRegression;
use lightgbm3::Booster;
use std::{io::{BufReader, BufWriter}, fs::File};

pub fn extract_ce_data(data_dir: &str) -> Result<DataFrame, Box<dyn Error>> {
    println!("Extracting Consumer Expenditure data from {}", data_dir);
    
    let quarters = vec!["232", "233", "234", "241"];
    let mut all_households = Vec::new();
    
    for quarter in quarters {
        let file_path = format!("{}/fmli{}.csv", data_dir, quarter);
        if std::path::Path::new(&file_path).exists() {
            println!("  Loading {}", file_path);
            
            let path = PlPath::new(&file_path);
            let df = LazyCsvReader::new(path)
                .with_dtype_overwrite(Some(Arc::from(Schema::from_iter([
                    ("AGE_REF".into(), DataType::UInt32),
                    ("FAM_SIZE".into(), DataType::UInt32),
                    ("NO_EARNR".into(), DataType::UInt32),
                    ("PERSLT18".into(), DataType::UInt32),
                    ("FINCBTXM".into(), DataType::Float64),
                    ("TOTEXPCQ".into(), DataType::Float64),
                    ("FOODCQ".into(), DataType::Float64),
                    ("HOUSCQ".into(), DataType::Float64),
                    ("TRANSCQ".into(), DataType::Float64),
                    ("HEALTHCQ".into(), DataType::Float64),
                    ("EDUC_REF".into(), DataType::UInt32),
                    ("REGION".into(), DataType::UInt32),
                    ("BLS_URBN".into(), DataType::UInt32),
                    ("CUTENURE".into(), DataType::UInt32),
                ]))))
                .finish()?;
                
            let selected = df.lazy()
                .select([
                    col("AGE_REF"), col("FAM_SIZE"), col("NO_EARNR"), col("PERSLT18"),
                    col("FINCBTXM"), col("TOTEXPCQ"), col("FOODCQ"), col("HOUSCQ"),
                    col("TRANSCQ"), col("HEALTHCQ"), col("EDUC_REF"), col("REGION"),
                    col("BLS_URBN"), col("CUTENURE"),
                ])
                .collect()?;
                
            all_households.push(selected.lazy());
        }
    }
    
    if all_households.is_empty() {
        return Err("No data files found".into());
    }
    
    let combined = concat(&all_households, UnionArgs::default())?
        .filter(col("FINCBTXM").is_not_null())
        .filter(col("TOTEXPCQ").gt(0))
        .filter(col("AGE_REF").is_not_null())
        .filter(col("FAM_SIZE").is_not_null())
        .filter(col("EDUC_REF").is_not_null());
    
    let processed = combined.lazy()
        .filter(col("FINCBTXM").gt(0))
        .filter(col("TOTEXPCQ").gt(0))
        .filter(col("AGE_REF").gt_eq(18).and(col("AGE_REF").lt_eq(95)))
        .filter((col("TOTEXPCQ") * lit(4) / col("FINCBTXM")).lt_eq(10.0))
        .with_column((col("TOTEXPCQ") * lit(4)).alias("annual_spending"))
        .with_column((col("TOTEXPCQ") * lit(4) / col("FINCBTXM")).alias("spending_rate"))
        .with_column((col("FINCBTXM") - (col("TOTEXPCQ") * lit(4))).alias("implied_savings"))
        .filter(col("annual_spending").gt(1000.0).and(col("annual_spending").lt(500000.0)))
        .filter(col("FINCBTXM").lt(1000000.0))
        .with_column(
            (col("NO_EARNR").cast(DataType::Float64) / col("FAM_SIZE").cast(DataType::Float64))
                .alias("earner_ratio")
        )
        .with_column(
            when(col("PERSLT18").gt(0))
                .then(lit(1))
                .otherwise(lit(0))
                .alias("has_children")
        )
        .with_column(
            when(col("EDUC_REF").lt(12)).then(lit(0))
            .when(col("EDUC_REF").eq(12)).then(lit(1))
            .when(col("EDUC_REF").gt(12).and(col("EDUC_REF").lt_eq(14))).then(lit(2))
            .when(col("EDUC_REF").eq(15)).then(lit(3))
            .when(col("EDUC_REF").gt_eq(16)).then(lit(4))
            .otherwise(lit(1))
            .alias("education")
        )
        .with_column(
            when(col("CUTENURE").eq(1)).then(lit(0))
            .when(col("CUTENURE").eq(2)).then(lit(1))
            .otherwise(lit(2))
            .alias("housing_status")
        )
        .with_column(
            when(col("BLS_URBN").eq(1)).then(lit(1))
            .otherwise(lit(0))
            .alias("is_urban")
        )
        .select([
            col("annual_spending"),
            col("spending_rate"),
            col("FINCBTXM").alias("income"),
            col("AGE_REF").alias("age"),
            col("FAM_SIZE").alias("family_size"),
            col("has_children"),
            col("education"),
            col("housing_status"),
            col("is_urban"),
            col("implied_savings"),
            col("earner_ratio"),
            col("REGION").alias("region"),
            (col("FOODCQ") / col("TOTEXPCQ")).alias("food_share"),
            (col("HOUSCQ") / col("TOTEXPCQ")).alias("housing_share"),
            (col("TRANSCQ") / col("TOTEXPCQ")).alias("transport_share"),
            (col("HEALTHCQ") / col("TOTEXPCQ")).alias("health_share"),
        ])
        .collect()?;
    
    println!("Loaded {} household records", processed.height());
    Ok(processed)
}

pub fn dataframe_to_features(
    df: &DataFrame, 
    feature_cols: &[&str]
) -> Result<(Array2<f64>, Vec<String>), Box<dyn Error>> {
    let n_rows = df.height();
    let n_cols = feature_cols.len();
    let mut matrix = Array2::<f64>::zeros((n_rows, n_cols));
    
    for (col_idx, col_name) in feature_cols.iter().enumerate() {
        let column = df.column(col_name)?;
        
        match column.dtype() {
            DataType::Float64 => {
                let values = column.f64()?;
                for (row_idx, val) in values.into_iter().enumerate() {
                    matrix[[row_idx, col_idx]] = val.unwrap_or(0.0);
                }
            }
            DataType::UInt32 => {
                let values = column.u32()?;
                for (row_idx, val) in values.into_iter().enumerate() {
                    matrix[[row_idx, col_idx]] = val.unwrap_or(0) as f64;
                }
            }
            DataType::Int32 => {
                let values = column.i32()?;
                for (row_idx, val) in values.into_iter().enumerate() {
                    matrix[[row_idx, col_idx]] = val.unwrap_or(0) as f64;
                }
            }
            _ => return Err(format!("Unsupported dtype for column {}", col_name).into()),
        }
    }
    
    let feature_names = feature_cols.iter().map(|s| s.to_string()).collect();
    Ok((matrix, feature_names))
}

pub fn column_to_array(column: &Column) -> Result<Array1<f64>, Box<dyn Error>> {
    match column.dtype() {
        DataType::Float64 => {
            let ca = column.f64()?;
            let vec: Vec<f64> = ca.into_iter()
                .map(|opt| opt.unwrap_or(0.0))
                .collect();
            Ok(Array1::from(vec))
        }
        DataType::Int32 => {
            let ca = column.i32()?;
            let vec: Vec<f64> = ca.into_iter()
                .map(|opt| opt.unwrap_or(0) as f64)
                .collect();
            Ok(Array1::from(vec))
        }
        DataType::UInt32 => {
            let ca = column.u32()?;
            let vec: Vec<f64> = ca.into_iter()
                .map(|opt| opt.unwrap_or(0) as f64)
                .collect();
            Ok(Array1::from(vec))
        }
        _ => Err(format!("Unsupported dtype: {:?}", column.dtype()).into())
    }
}

pub fn engineer_additional_features(df: DataFrame) -> Result<DataFrame, Box<dyn Error>> {
    let mut df = df.lazy()
        .with_column((col("income") / lit(10000.0)).alias("income_10k"))
        .with_column(
            when(col("age").lt(35)).then(lit(1))
            .when(col("age").lt(55)).then(lit(2))
            .when(col("age").lt(65)).then(lit(3))
            .otherwise(lit(4))
            .alias("age_group")
        )
        .with_column(
            when(col("income").lt(30000)).then(lit(1))
            .when(col("income").lt(50000)).then(lit(2))
            .when(col("income").lt(75000)).then(lit(3))
            .when(col("income").lt(100000)).then(lit(4))
            .when(col("income").lt(150000)).then(lit(5))
            .otherwise(lit(6))
            .alias("income_bracket")
        )
        .with_column((col("income_bracket") * col("age_group")).alias("income_age_interaction"))
        .with_column((col("income_bracket") * col("education")).alias("income_education"))
        .with_column((col("family_size") * col("has_children")).alias("family_children"))
        .with_column((col("food_share") + col("housing_share")).alias("necessities_share"))
        .collect()?;
        
    let log_income = df.column("income")?
        .f64()?
        .apply(|val| val.map(|v| v.max(1000.0).ln()))
        .with_name("log_income".into())
        .into_series();
    
    df.with_column(log_income)?;
    
    Ok(df)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumerDecisionModel {
    pub classifier: FittedLogisticRegression<f64, bool>,
    pub regressor_model_string: String,
    pub feature_names: Vec<String>,
    pub feature_means: Vec<f64>,
    pub feature_stds: Vec<f64>,
    pub spending_threshold: f64,
}

impl ConsumerDecisionModel {
    pub fn predict(&self, features: &Array1<f64>) -> f64 {
        let normalized_features = self.normalize_features(features);
        
        let booster = match Booster::from_string(&self.regressor_model_string) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Error loading booster: {}", e);
                return features[0] * 0.7;
            }
        };
        
        let prediction_result = match booster.predict(
            normalized_features.as_slice().unwrap(),
            normalized_features.len() as i32,
            true
        ) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error in prediction: {}", e);
                return features[0] * 0.7;
            }
        };
        
        let predicted_log = prediction_result[0] as f64;
        let prediction = predicted_log.exp();
        
        let income = features[0];
        let min_spending = 1000.0;
        let max_spending = income * 2.0;
        
        prediction.max(min_spending).min(max_spending)
    }

    pub fn normalize_features(&self, features: &Array1<f64>) -> Array1<f64> {
        let mut normalized = features.clone();
        for i in 0..features.len() {
            if self.feature_stds[i] > 0.0 {
                normalized[i] = (normalized[i] - self.feature_means[i]) / self.feature_stds[i];
            }
        }
        normalized
    }
    
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        let config = bincode::config::standard();
        bincode::serde::encode_into_std_write(self, &mut writer, config)?;
        Ok(())
    }
    
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn Error>> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let config = bincode::config::standard();
        let model = bincode::serde::decode_from_std_read(&mut reader, config)?;
        Ok(model)
    }
}

impl Clone for ConsumerDecisionModel {
    fn clone(&self) -> Self {
        // Note: This is a shallow clone - the internal models are serialized strings
        Self {
            classifier: self.classifier.clone(),
            regressor_model_string: self.regressor_model_string.clone(),
            feature_names: self.feature_names.clone(),
            feature_means: self.feature_means.clone(),
            feature_stds: self.feature_stds.clone(),
            spending_threshold: self.spending_threshold,
        }
    }
}