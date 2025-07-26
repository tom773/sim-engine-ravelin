use crate::*;
use serde::{Serialize, Deserialize};
use rand::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Firm {
    pub id: AgentId,
    pub bank_id: AgentId,
    pub name: String,
    pub employees: u32,
    pub wage_rate: f64,
    pub productivity: f64,
}

impl Firm {
    pub fn new(id: AgentId, bank_id: AgentId, name: String) -> Self {
        Self {
            id,
            name,
            bank_id,
            employees: 0,
            wage_rate: 15.0,  // $15/hour default
            productivity: 1.0, // Units per employee per period
        }
    }
}

impl Agent for Firm {
    type DecisionType = FirmDecision;
    
    fn decide(&self, fs: &FinancialSystem, _rng: &mut StdRng) -> Vec<FirmDecision> {
        let mut decisions = Vec::new();
        
        // Check current market conditions
        let market = fs.exchange.goods_market(&GoodId::generic());
        let _current_price = market
            .and_then(|m| m.quote(&GoodId::generic()))
            .unwrap_or(25.0);
        
        // Simple decision logic: always try to produce and maintain some employees
        if self.employees < 10 {
            decisions.push(FirmDecision::Hire { quantity: 5 });
        }
        
        if self.employees > 0 {
            let production = (self.employees as f64 * self.productivity) as u32;
            decisions.push(FirmDecision::Produce { 
                good_id: GoodId::generic(), 
                quantity: production.min(100), // Cap production
            });
        }
        
        // Set price based on costs plus markup
        let unit_cost = if self.employees > 0 {
            (self.wage_rate * 40.0) / self.productivity // Weekly wage / productivity
        } else {
            20.0 // Default cost estimate
        };
        let target_price = unit_cost * 1.2; // 20% markup
        
        decisions.push(FirmDecision::SetPrice {
            good_id: GoodId::generic(),
            price: target_price,
        });
        
        decisions
    }
    
    fn act(&self, decisions: &[FirmDecision]) -> Vec<SimAction> {
        let mut actions = Vec::new();
        
        for decision in decisions {
            match decision {
                FirmDecision::Produce { good_id, quantity } => {
                    if *quantity > 0 {
                        actions.push(SimAction::Produce { 
                            agent_id: self.id.clone(),
                            good_id: good_id.clone(), 
                            amount: *quantity as f64,
                        });
                    }
                }
                FirmDecision::Hire { quantity } => {
                    if *quantity > 0 {
                        actions.push(SimAction::Hire { 
                            agent_id: self.id.clone(), 
                            count: *quantity,
                        });
                    }
                }
                FirmDecision::SetPrice { good_id, price } => {
                    // TODO
                    let _ = good_id; // Fuck off unused
                    let _p = price; // Get rid of unused variable warning
                }
            }
        }
        
        actions
    }
}