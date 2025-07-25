use ndarray::Array1;
use dyn_clone::{clone_trait_object, DynClone};
use std::fmt::Debug;

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
