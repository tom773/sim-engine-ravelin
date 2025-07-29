use std::any::type_name;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use shared::*;
use crate::{ExecutionResult, SimState, EffectError};

pub mod banking;
pub mod production;
pub mod trading;

pub use banking::BankingDomain;
pub use production::ProductionDomain;
pub use trading::TradingDomain;

pub trait ExecutionDomain: Send + Sync {
    fn name(&self) -> &'static str;
    fn can_handle(&self, action: &SimAction) -> bool;
    fn validate(&self, action: &SimAction, state: &SimState) -> bool;
    fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult;
    fn clone_box(&self) -> Box<dyn SerializableExecutionDomain>;
}

impl Clone for Box<dyn SerializableExecutionDomain> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[typetag::serde(tag = "domain_type")]
pub trait SerializableExecutionDomain: ExecutionDomain {}
impl std::fmt::Debug for dyn SerializableExecutionDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SerializableExecutionDomain({})", self.name())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DomainRegistry {
    domains: HashMap<String, Box<dyn SerializableExecutionDomain>>,
}

impl DomainRegistry {
    pub fn new() -> Self {
        Self {
            domains: HashMap::new(),
        }
    }
    
    pub fn builder() -> DomainRegistryBuilder {
        DomainRegistryBuilder::new()
    }
    
    pub fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        for (domain_type, domain) in &self.domains {
            if domain.can_handle(action) {
                if domain.validate(action, state) {
                    return domain.execute(action, state);
                } else {
                    return ExecutionResult {
                        success: false,
                        effects: vec![],
                        errors: vec![EffectError::InvalidState(format!("Validation failed in domain {}", domain_type))],
                    };
                }
            }
        }
        
        ExecutionResult {
            success: false,
            effects: vec![],
            errors: vec![EffectError::InvalidState(format!("No domain registered to handle action: {}", action.name()))],
        }
    }
}

impl Default for DomainRegistry {
    fn default() -> Self {
        DomainRegistryBuilder::new()
            .with_defaults()
            .build()
    }
}

#[derive(Default)]
pub struct DomainRegistryBuilder {
    domains: HashMap<String, Box<dyn SerializableExecutionDomain>>,
}

impl DomainRegistryBuilder {
    pub fn new() -> Self {
        Self {
            domains: HashMap::new(),
        }
    }
    
    pub fn with_domain<D: SerializableExecutionDomain + 'static>(mut self, domain: D) -> Self {
        let key = type_name::<D>().to_string();
        self.domains.insert(key, Box::new(domain));
        self
    }
    
    pub fn with_defaults(self) -> Self {
        self.with_domain(BankingDomain::new())
            .with_domain(TradingDomain::new())
            .with_domain(ProductionDomain::new())
    }
    
    pub fn build(self) -> DomainRegistry {
        DomainRegistry {
            domains: self.domains,
        }
    }
}