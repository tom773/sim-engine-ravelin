pub struct Validator;

impl Validator {
    // Amount validations
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
    
    pub fn amount_in_range(amount: f64, min: f64, max: f64) -> Result<(), String> {
        if amount < min || amount > max {
            Err(format!("Amount {:.2} must be between {:.2} and {:.2}", amount, min, max))
        } else {
            Ok(())
        }
    }
    
    // String validations
    pub fn non_empty_string(value: &str, field_name: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            Err(format!("{} cannot be empty", field_name))
        } else {
            Ok(())
        }
    }
    
    // Numeric validations
    pub fn percentage(value: f64) -> Result<(), String> {
        if value < 0.0 || value > 1.0 {
            Err(format!("Percentage must be between 0 and 1, got: {:.4}", value))
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
}