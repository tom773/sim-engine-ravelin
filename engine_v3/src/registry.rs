use domains::*;
use sim_core::*;
use std::collections::HashMap;
extern crate inventory;

pub struct DomainRegistry {
    domains: HashMap<String, Box<dyn Domain>>,
}

impl DomainRegistry {
    pub fn new() -> Self {
        let mut registry = Self { domains: HashMap::new() };

        for registration in inventory::iter::<DomainRegistration> {
            let domain = (registration.constructor)();
            registry.domains.insert(registration.name.to_string(), domain);
        }

        registry
    }

    pub fn execute_action(&self, action: &SimAction, state: &SimState) -> Result<Vec<StateEffect>, String> {
        let domain_name = self.get_domain_name_for_action(action);

        let domain =
            self.domains.get(&domain_name).ok_or_else(|| format!("No domain found for action: {}", domain_name))?;

        let result = domain.execute(action, state);

        if result.success { Ok(result.effects) } else { Err(result.errors.join("; ")) }
    }

    pub fn settle_trade(&self, trade: &Trade, state: &SimState) -> Result<Vec<StateEffect>, String> {
        if let Some(trading_domain) = self.domains.get("Trading") {
            let result = trading_domain.settle_trade(trade, state);
            if result.success {
                return Ok(result.effects);
            }
        }

        for (domain_name, domain) in &self.domains {
            let result = domain.settle_trade(trade, state);
            if result.success {
                println!("[REGISTRY] Trade settled by domain: {}", domain_name);
                return Ok(result.effects);
            }
        }

        Err(format!("No domain could settle trade for market: {:?}", trade.market_id))
    }

    pub fn categorize_intentions_by_phase(
        &self, intentions: Vec<SimIntention>,
    ) -> HashMap<ResolutionPhase, Vec<SimIntention>> {
        let mut categorized: HashMap<ResolutionPhase, Vec<SimIntention>> = HashMap::new();
        for intention in intentions {
            let mut handled = false;
            for domain in self.domains.values() {
                if let Some(phase) = domain.resolution_phase(&intention) {
                    categorized.entry(phase).or_default().push(intention.clone());
                    handled = true;
                    break;
                }
            }
            if !handled {
                println!("[WARNING] No resolution phase found for intention: {:?}", intention);
            }
        }
        categorized
    }

    pub fn resolve_intention(&self, intention: &SimIntention, context: &ResolutionContext) -> ResolutionResult {
        for domain in self.domains.values() {
            if let Some(result) = domain.resolve_intention(intention, context) {
                return result;
            }
        }
        ResolutionResult::not_handled()
    }

    fn get_domain_name_for_action(&self, action: &SimAction) -> String {
        match action {
            SimAction::Banking(_) => "Banking".to_string(),
            SimAction::Consumption(_) => "Consumption".to_string(),
            SimAction::Fiscal(_) => "Fiscal".to_string(),
            SimAction::Production(_) => "Production".to_string(),
            SimAction::Transaction(_) => "Transactions".to_string(),
            SimAction::Monetary(_) => "Monetary".to_string(),
            SimAction::Credit(_) => "Credit".to_string(),
        }
    }
}

impl Default for DomainRegistry {
    fn default() -> Self {
        Self::new()
    }
}