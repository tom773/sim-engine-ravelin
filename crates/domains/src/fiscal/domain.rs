use serde::{Deserialize, Serialize};
use sim_core::*;
use sim_macros::SimDomain;

#[derive(Clone, Debug, Serialize, Deserialize, Default, SimDomain)]
pub struct FiscalDomain {}

#[derive(Debug, Clone)]
pub struct FiscalResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}


impl FiscalDomain {
    pub fn new() -> Self {
        Self {}
    }
    pub fn execute(&self, action: &FiscalAction, state: &SimState) -> FiscalResult {
        let mut effects = vec![];
        match action {
            FiscalAction::ChangeTaxRate { government_id, tax_type, new_rate } => {
                println!(
                    "[FISCAL DOMAIN] Executing ChangeTaxRate for {} | Setting {:?} to {}",
                    government_id, tax_type, new_rate
                );
            }
            FiscalAction::IssueDebt { government_id, tenor, quantity, face_value, coupon_rate: _ } => {
                let target_market_id = FinancialMarketId::Treasury { tenor: *tenor };
                
                if let Some(_market) = state.financial_system.exchange.financial_markets.get(&target_market_id) {
                    let ask_price = *face_value; // Par value - could be refined further
                    
                    let ask_order = Order::Ask(Ask { 
                        agent_id: *government_id, 
                        quantity: *quantity as f64, 
                        price: ask_price 
                    });

                    effects.push(StateEffect::Market(MarketEffect::PlaceOrderInBook { 
                        market_id: MarketId::Financial(target_market_id.clone()), 
                        order: ask_order 
                    }));
                    
                } else {
                    println!(
                        "[FISCAL DOMAIN] WARNING: Market for tenor {:?} not found!",
                        tenor
                    );
                }
            }
            FiscalAction::SetSpendingTarget { government_id, .. } => {
                println!("[FISCAL DOMAIN] Executing SetSpendingTarget for {}", government_id);
            }
        }
        FiscalResult { success: true, effects, errors: vec![] }
    }
}