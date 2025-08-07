use domains::prelude::*;
use sim_core::{SimAction, SimState, StateEffect, Trade};
use std::collections::HashMap;

pub struct DomainRegistry {
    domains: HashMap<&'static str, Box<dyn Domain>>,
}

impl DomainRegistry {
    pub fn new() -> Self {
        let mut domains = HashMap::new();
        for registration in inventory::iter::<DomainRegistration> {
            let domain_instance = (registration.constructor)();
            domains.insert(registration.name, domain_instance);
        }
        println!("[Registry] Loaded {} domains.", domains.len());
        Self { domains }
    }

    pub fn execute(&self, action: &SimAction, state: &SimState) -> Vec<StateEffect> {
        let domain_name = match action {
            SimAction::Banking(_) => "Banking",
            SimAction::Consumption(_) => "Consumption",
            SimAction::Fiscal(_) => "Fiscal",
            SimAction::Production(_) => "Production",
            SimAction::Settlement(_) => "Settlement",
            SimAction::Trading(_) => "Trading",
            SimAction::Labour(_) => "Labour",
        };

        if let Some(domain) = self.domains.get(domain_name) {
            let result = domain.execute(action, state);
            if !result.success {
                println!("[ERROR] Action failed in domain '{}'. Errors: {:?}", domain.name(), result.errors);
            }
            result.effects
        } else {
            println!("[ERROR] No domain registered to handle action: {:?}", action.name());
            vec![]
        }
    }

    pub fn settle_financial_trade(&self, trade: &Trade, state: &SimState) -> TradingResult {
        if let Some(domain) = self.domains.get("Trading") {
            if let Some(trading_domain) = domain.as_any().downcast_ref::<TradingDomain>() {
                return trading_domain.settle_financial_trade(trade, state);
            }
        }
        TradingResult {
            success: false,
            effects: vec![],
            errors: vec!["TradingDomain not found for settlement.".to_string()],
        }
    }
}

impl Default for DomainRegistry {
    fn default() -> Self {
        Self::new()
    }
}