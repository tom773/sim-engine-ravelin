use crate::SimAction;

pub trait ActionValidator {
    fn validate(&self, action: &SimAction) -> Result<(), String>;
}

pub struct Validator;

impl Validator {
    pub fn positive_amount(amount: f64) -> Result<(), String> {
        if amount <= 0.0 {
            Err(format!("Amount must be positive, got: {:.2}", amount))
        } else {
            Ok(())
        }
    }
    
    pub fn non_negative_amount(amount: f64) -> Result<(), String> {
        if amount < 0.0 {
            Err(format!("Amount cannot be negative, got: {:.2}", amount))
        } else {
            Ok(())
        }
    }
    
    pub fn positive_integer(value: u32, field_name: &str) -> Result<(), String> {
        if value == 0 {
            Err(format!("{} must be greater than 0", field_name))
        } else {
            Ok(())
        }
    }
    
    pub fn percentage(value: f64) -> Result<(), String> {
        if value < 0.0 || value > 1.0 {
            Err(format!("Percentage must be between 0 and 1, got: {:.4}", value))
        } else {
            Ok(())
        }
    }
}