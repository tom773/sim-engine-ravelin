use rand::RngCore;
use sim_prelude::*;
use serde::{Deserialize, Serialize};

/// Extended Bank with decision-making capabilities
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankingAgent {
    pub bank: Bank,
    pub decision_model: Box<dyn BankDecisionModel>,
}

impl BankingAgent {
    pub fn new(bank: Bank) -> Self {
        Self {
            bank,
            decision_model: Box::new(BasicBankDecisionModel),
        }
    }

    pub fn with_decision_model(mut self, model: Box<dyn BankDecisionModel>) -> Self {
        self.decision_model = model;
        self
    }

    pub fn decide(&self, fs: &FinancialSystem, rng: &mut dyn RngCore) -> Vec<BankDecision> {
        self.decision_model.decide(&self.bank, fs, rng)
    }

    pub fn act(&self, decisions: &[BankDecision]) -> Vec<SimAction> {
        let mut actions = Vec::new();

        for decision in decisions {
            match decision {
                BankDecision::BorrowOvernight { amount_dollars, max_annual_rate_bps } => {
                    // TODO: Convert to market action
                    println!("Bank {} wants to borrow ${} at max rate {}", 
                        self.bank.id.0, amount_dollars, max_annual_rate_bps);
                }
                BankDecision::LendOvernight { amount_dollars, min_annual_rate_bps } => {
                    // TODO: Convert to market action
                    println!("Bank {} wants to lend ${} at min rate {}", 
                        self.bank.id.0, amount_dollars, min_annual_rate_bps);
                }
                BankDecision::SetDepositRate { rate } => {
                    println!("Bank {} setting deposit rate to {}", self.bank.id.0, rate);
                }
                BankDecision::SetLendingRate { rate } => {
                    println!("Bank {} setting lending rate to {}", self.bank.id.0, rate);
                }
                BankDecision::ManageReserves { target_level } => {
                    // Calculate needed reserve adjustment
                    let current_reserves = self.get_current_reserves(); // TODO: implement
                    let adjustment = target_level - current_reserves;
                    
                    if adjustment.abs() > 100.0 { // Only act if significant difference
                        actions.push(SimAction::Banking(BankingAction::UpdateReserves {
                            bank: self.bank.id,
                            amount_change: adjustment,
                        }));
                    }
                }
            }
        }

        actions
    }

    fn get_current_reserves(&self) -> f64 {
        // TODO: Calculate current reserves from financial system
        10000.0 // Placeholder
    }
}
