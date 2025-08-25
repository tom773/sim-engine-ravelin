use std::collections::HashMap;
use sim_core::*;
use domains::*;
extern crate inventory;
pub struct DomainRegistry {
    domains: HashMap<String, Box<dyn Domain>>,
}

impl DomainRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            domains: HashMap::new(),
        };
        
        for registration in inventory::iter::<DomainRegistration> {
            let domain = (registration.constructor)();
            registry.domains.insert(registration.name.to_string(), domain);
        }
        
        registry
    }
    
    pub fn execute_action(&self, action: &SimAction, state: &SimState) -> Result<Vec<StateEffect>, String> {
        let domain_name = self.get_domain_name_for_action(action);
        
        let domain = self.domains.get(&domain_name)
            .ok_or_else(|| format!("No domain found for action: {}", domain_name))?;
        
        let result = domain.execute(action, state);
        
        if result.success {
            Ok(result.effects)
        } else {
            Err(result.errors.join("; "))
        }
    }

    pub fn categorize_intentions_by_phase(
        &self,
        intentions: Vec<SimIntention>,
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
    
    pub fn resolve_intention(
        &self,
        intention: &SimIntention,
        context: &ResolutionContext,
    ) -> ResolutionResult {
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
            SimAction::Labour(_) => "Labour".to_string(),
            SimAction::Production(_) => "Production".to_string(),
            SimAction::Settlement(_) => "Settlement".to_string(),
            SimAction::Trading(_) => "Trading".to_string(),
        }
    }
}

impl Default for DomainRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct DomainStats {
    pub total_domains: usize,
    pub domain_names: Vec<String>,
    pub resolution_capabilities: HashMap<String, Vec<ResolutionPhase>>,
}

impl DomainStats {
    pub fn print_summary(&self) {
        println!("Domain Registry Summary:");
        println!("  Total domains: {}", self.total_domains);
        println!("  Registered domains:");
        
        for name in &self.domain_names {
            let phases = self.resolution_capabilities.get(name)
                .map(|p| format!("{:?}", p))
                .unwrap_or_else(|| "None".to_string());
            println!("    {}: {}", name, phases);
        }
    }
}