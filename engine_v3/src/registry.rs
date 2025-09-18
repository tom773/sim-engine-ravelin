use domains::prelude::*;
use indexmap::IndexMap;
use std::collections::HashMap;

pub struct DomainRegistry {
    domains: IndexMap<String, Box<dyn Domain>>,
}

impl DomainRegistry {
    pub fn new() -> Self {
        let mut registry = Self { domains: IndexMap::new() };
        registry.register("Transactions", Box::new(TransactionsDomain::new()));
        registry.register("Monetary", Box::new(MonetaryDomain::new()));
        registry.register("Banking", Box::new(BankingDomain::new()));
        registry.register("Fiscal", Box::new(FiscalDomain::new()));
        registry.register("Production", Box::new(ProductionDomain::new()));
        registry.register("Consumption", Box::new(ConsumptionDomain::new()));

        registry
    }

    pub fn register(&mut self, name: &str, domain: Box<dyn Domain>) {
        self.domains.insert(name.to_string(), domain);
    }

    pub fn get_domain(&self, name: &str) -> Option<&Box<dyn Domain>> {
        self.domains.get(name)
    }

    pub fn get_domain_for_action(&self, action: &SimAction) -> Option<&Box<dyn Domain>> {
        let domain_name = self.get_domain_name_for_action(action);
        self.domains.get(&domain_name)
    }

    fn get_domain_name_for_action(&self, action: &SimAction) -> String {
        match action {
            SimAction::Banking(_) => "Banking".to_string(),
            SimAction::Consumption(_) => "Consumption".to_string(),
            SimAction::Fiscal(_) => "Fiscal".to_string(),
            SimAction::Production(_) => "Production".to_string(),
            SimAction::Transaction(_) => "Transactions".to_string(),
            SimAction::Monetary(_) => "Monetary".to_string(),
            SimAction::Credit(_) => "Transactions".to_string(),
        }
    }

    pub fn categorize_intentions_by_phase(&self, intentions: Vec<SimIntention>) -> HashMap<String, Vec<SimIntention>> {
        let mut categorized: HashMap<String, Vec<SimIntention>> = HashMap::new();
        for intention in intentions {
            let mut found: Option<ResolutionPhase> = None;
            for domain in self.domains.values() {
                if let Some(phase) = domain.resolution_phase(&intention) {
                    if found.is_some() {
                        debug_assert!(
                            false,
                            "Multiple domains claimed resolution phase for intention {}",
                            intention.name()
                        );
                    } else {
                        found = Some(phase);
                    }
                }
            }
            if let Some(phase) = found {
                let k = match phase {
                    ResolutionPhase::Independent => "Independent",
                    ResolutionPhase::Market => "Market",
                    ResolutionPhase::Dependent => "Dependent",
                }
                .to_string();
                categorized.entry(k).or_default().push(intention);
            } else {
                tracing::warn!("No resolution phase found for intention: {}", intention.name());
            }
        }
        categorized
    }

    pub fn resolve_intention(&self, intention: &SimIntention, context: &ResolutionContext) -> ResolutionResult {
        let mut first: Option<ResolutionResult> = None;
        for domain in self.domains.values() {
            if let Some(result) = domain.resolve_intention(intention, context) {
                if first.is_some() {
                    debug_assert!(
                        false,
                        "Multiple domains attempted to resolve intention {}. \
                         Priority order kept the first.",
                        intention.name()
                    );
                } else {
                    first = Some(result);
                }
            }
        }
        first.unwrap_or_else(ResolutionResult::not_handled)
    }

    pub fn execute_action(&self, action: &SimAction, state: &SimState) -> DomainResult {
        if let Some(domain) = self.get_domain_for_action(action) {
            domain.execute(action, state)
        } else {
            DomainResult::failure(vec![format!("No domain found for action: {:?}", action)])
        }
    }

    pub fn settle_trade(&self, trade: &Trade, state: &SimState) -> DomainResult {
        if let Some(domain) = self.domains.get("Transactions") {
            domain.settle_trade(trade, state)
        } else {
            DomainResult::failure(vec!["Transactions domain not found".to_string()])
        }
    }
}
