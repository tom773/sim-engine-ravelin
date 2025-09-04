pub struct Validator;
use crate::*;
impl Validator {
    pub fn positive_amount(amount: f64) -> Result<(), String> {
        if amount <= 0.0 { Err(format!("Amount must be positive, got: {:.2}", amount)) } else { Ok(()) }
    }

    pub fn non_negative_amount(amount: f64) -> Result<(), String> {
        if amount < 0.0 { Err(format!("Amount cannot be negative, got: {:.2}", amount)) } else { Ok(()) }
    }

    pub fn positive_integer(value: u32, field_name: &str) -> Result<(), String> {
        if value == 0 { Err(format!("{} must be greater than 0", field_name)) } else { Ok(()) }
    }

    pub fn percentage(value: f64) -> Result<(), String> {
        if value < 0.0 || value > 1.0 {
            Err(format!("Percentage must be between 0 and 1, got: {:.4}", value))
        } else {
            Ok(())
        }
    }
    pub fn agent_exists(agent_id: AgentId, state: &SimState) -> Result<(), String> {
        if state.financial_system.balance_sheets.contains_key(&agent_id) {
            Ok(())
        } else {
            Err(format!("Agent {} does not exist", agent_id.0))
        }
    }

    pub fn bank_exists(bank_id: AgentId, state: &SimState) -> Result<(), String> {
        if state.agents.banks.contains_key(&bank_id) {
            Ok(())
        } else {
            Err("Target is not a valid commercial bank".to_string())
        }
    }

    pub fn firm_exists(firm_id: AgentId, state: &SimState) -> Result<(), String> {
        if state.agents.firms.contains_key(&firm_id) { Ok(()) } else { Err("Target is not a valid firm".to_string()) }
    }
}
