#[allow(unused_variables)]
use crate::{EffectError, SimState};
use ravelin_traits::ExecutionResult;
use serde::{Deserialize, Serialize};
use crate::*;
use std::any::type_name;
use std::collections::HashMap;
use std::fmt;

#[macro_export]
macro_rules! impl_execution_domain {
    (
        $domain_struct:ty,
        $domain_name:expr,
        validate = |$action_validate:ident, $state_validate:ident| $validate_body:block,
        execute = |$self_param:ident, $action_param:ident, $state_param:ident| {
            $( $action_pat:pat => $handler_call:expr ),* $(,)?
        }
    ) => {
        impl ExecutionDomain for $domain_struct {
            fn name(&self) -> &'static str { $domain_name }

            fn can_handle(&self, action: &SimAction) -> bool {
                match action {
                    $(
                        $action_pat => true,
                    )*
                    _ => false,
                }
            }

            fn validate(&self, $action_validate: &SimAction, $state_validate: &SimState) -> bool {
                $validate_body
            }

            fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult<StateEffect> {
                let $self_param = self;
                let $action_param = action;
                let $state_param = state;
                
                match action {
                    $(
                        $action_pat => $handler_call,
                    )*
                    _ => crate::execution::domain::unhandled(self.name()),
                }
            }

            fn clone_box(&self) -> Box<dyn ExecutionDomain> {
                Box::new(self.clone())
            }
        }
    };
}

pub mod banking;
pub mod production;
pub mod trading;

pub use banking::BankingDomain;
pub use production::ProductionDomain;
pub use trading::TradingDomain;

pub fn unhandled(domain: &str) -> ExecutionResult<StateEffect> {
    ExecutionResult {
        success: false,
        effects: vec![],
        errors: vec![Box::new(EffectError::Unhandled(format!("Action not handled in domain {}", domain)))],
    }
}

pub trait ExecutionDomain: Send + Sync {
    fn name(&self) -> &'static str;
    fn can_handle(&self, action: &SimAction) -> bool;
    fn validate(&self, action: &SimAction, state: &SimState) -> bool;
    fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult<StateEffect>;
    fn clone_box(&self) -> Box<dyn ExecutionDomain>;
}

impl Clone for Box<dyn ExecutionDomain> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[typetag::serde(tag = "domain_type")]
pub trait SerializableExecutionDomain: ExecutionDomain {
    fn clone_box_serializable(&self) -> Box<dyn SerializableExecutionDomain>;
}

impl Clone for Box<dyn SerializableExecutionDomain> {
    fn clone(&self) -> Self {
        self.clone_box_serializable()
    }
}

impl fmt::Debug for dyn SerializableExecutionDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SerializableExecutionDomain").field("name", &self.name()).finish()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DomainRegistry {
    domains: HashMap<String, Box<dyn SerializableExecutionDomain>>,
}

impl DomainRegistry {
    pub fn new() -> Self {
        Self { domains: HashMap::new() }
    }

    pub fn builder() -> DomainRegistryBuilder {
        DomainRegistryBuilder::new()
    }

    pub fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult<StateEffect> {
        for (domain_type, domain) in &self.domains {
            if domain.can_handle(action) {
                if domain.validate(action, state) {
                    return domain.execute(action, state);
                } else {
                    return ExecutionResult {
                        success: false,
                        effects: vec![],
                        errors: vec![Box::new(EffectError::InvalidState(format!(
                            "Validation failed in domain {}",
                            domain_type
                        )))],
                    };
                }
            }
        }

        ExecutionResult {
            success: false,
            effects: vec![],
            errors: vec![Box::new(EffectError::InvalidState(format!(
                "No domain registered to handle action: {}",
                action.name()
            )))],
        }
    }
}

impl Default for DomainRegistry {
    fn default() -> Self {
        DomainRegistryBuilder::new().with_defaults().build()
    }
}

#[derive(Default)]
pub struct DomainRegistryBuilder {
    domains: HashMap<String, Box<dyn SerializableExecutionDomain>>,
}

impl DomainRegistryBuilder {
    pub fn new() -> Self {
        Self { domains: HashMap::new() }
    }

    pub fn with_domain<D: SerializableExecutionDomain + 'static>(mut self, domain: D) -> Self {
        let key = type_name::<D>().to_string();
        self.domains.insert(key, Box::new(domain));
        self
    }

    pub fn with_defaults(self) -> Self {
        self.with_domain(BankingDomain::new()).with_domain(TradingDomain::new()).with_domain(ProductionDomain::new())
    }

    pub fn build(self) -> DomainRegistry {
        DomainRegistry { domains: self.domains }
    }
}