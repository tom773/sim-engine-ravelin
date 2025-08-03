use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::Debug;
use dyn_clone::DynClone;
use rand::RngCore;

#[derive(Debug)]
pub struct ExecutionResult<E> {
    pub success: bool,
    pub effects: Vec<E>,
    pub errors: Vec<Box<dyn Error + Send + Sync>>,
}

pub trait Core: 'static + Sized + Send + Sync {
    type State: SimulationState<Core = Self>;
    type Action: SimulationAction<Core = Self>;
    type Effect: StateEffect<Core = Self>;
    type Scenario: SimulationScenario<Core = Self>;
    type DomainRegistry: ExecutionDomainRegistry<Core = Self>;

    fn inject_liquidity_action() -> Self::Action;
}

pub trait SimulationState:
    Clone + Debug + Default + Serialize + for<'de> Deserialize<'de> + Send + Sync
{
    type Core: Core<State = Self>;

    fn advance_time(&mut self);
    fn get_agents(&self) -> Vec<Box<dyn AbstractAgent<Core = Self::Core>>>;
    fn get_domain_registry(&self) -> &<Self::Core as Core>::DomainRegistry;
    fn apply_effects(
        &mut self,
        effects: &[<Self::Core as Core>::Effect],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn clear_markets_and_settle(&mut self) -> Vec<<Self::Core as Core>::Effect>;
    fn initialize_from_scenario(scenario: &<Self::Core as Core>::Scenario, rng: &mut dyn RngCore) -> Self;
    fn get_stats_json(&self) -> serde_json::Value;
}

pub trait AbstractAgent: DynClone + Send + Sync {
    type Core: Core;
    fn decide_and_act(
        &self,
        state: &<Self::Core as Core>::State,
        rng: &mut dyn RngCore,
    ) -> Vec<<Self::Core as Core>::Action>;
}
dyn_clone::clone_trait_object!(<C: Core> AbstractAgent<Core = C>);


pub trait SimulationAction: Clone + Debug + Serialize + for<'de> Deserialize<'de> + Send + Sync {
    type Core: Core<Action = Self>;
    fn name(&self) -> String;
}

pub trait StateEffect: Clone + Debug + Serialize + for<'de> Deserialize<'de> + Send + Sync {
    type Core: Core<Effect = Self>;
    fn name(&self) -> String;
}

pub trait SimulationScenario: Clone + Debug + Serialize + for<'de> Deserialize<'de> + Send + Sync {
    type Core: Core<Scenario = Self>;
    fn from_file(path: &str) -> Result<Self, Box<dyn Error>>;
    fn name(&self) -> &str;
}

pub trait ExecutionDomainRegistry:
    Clone + Debug + Default + Serialize + for<'de> Deserialize<'de> + Send + Sync
{
    type Core: Core<DomainRegistry = Self>;
    fn execute(
        &self,
        action: &<Self::Core as Core>::Action,
        state: &<Self::Core as Core>::State,
    ) -> ExecutionResult<<Self::Core as Core>::Effect>;
}

pub mod macros {
    #[macro_export]
    macro_rules! define_core {
        (
            $core_name:ident,
            State = $state_ty:ty,
            Action = $action_ty:ty,
            Effect = $effect_ty:ty,
            Scenario = $scenario_ty:ty,
            DomainRegistry = $registry_ty:ty
        ) => {
            #[derive(Clone, Copy, Debug)]
            pub struct $core_name;

            impl $crate::Core for $core_name {
                type State = $state_ty;
                type Action = $action_ty;
                type Effect = $effect_ty;
                type Scenario = $scenario_ty;
                type DomainRegistry = $registry_ty;

                fn inject_liquidity_action() -> Self::Action {
                    <$action_ty>::InjectLiquidity
                }
            }
        };
    }

    /**
    Implements a "named" trait for a core type by delegating to the type's own `name()` method.
    This works for `SimulationAction`, `StateEffect`, and `SimulationScenario`.
    */
    #[macro_export]
    macro_rules! impl_named_trait_for_core {
        ($trait_name:path, $concrete_type:ty, $core_type:ty, $return_type:ty) => {
            impl $trait_name for $concrete_type {
                type Core = $core_type;
                fn name(&self) -> $return_type {
                    self.name()
                }
            }
        };
    }
    
    /**
    Implements the `ExecutionDomainRegistry` trait for a core type.
    */
    #[macro_export]
    macro_rules! impl_domain_registry_for_core {
        ($registry_type:ty, $core_type:ty) => {
            impl $crate::ExecutionDomainRegistry for $registry_type {
                type Core = $core_type;
                fn execute(
                    &self,
                    action: &<$core_type as $crate::Core>::Action,
                    state: &<$core_type as $crate::Core>::State,
                ) -> $crate::ExecutionResult<<$core_type as $crate::Core>::Effect> {
                    self.execute(action, state)
                }
            }
        };
    }

    /**
    Defines the `AgentAdapter` struct and its `AbstractAgent` implementation,
    bridging a concrete `Agent` trait to the `dyn AbstractAgent`.
    */
    #[macro_export]
    macro_rules! define_agent_adapter {
        (
            $adapter_name:ident,
            $core_type:ty,
            $agent_trait:path,
            $state_type:ty,
            $action_type:ty
        ) => {
            #[derive(Clone)]
            pub struct $adapter_name<A>(A);

            impl<A> $crate::AbstractAgent for $adapter_name<A>
            where
                A: $agent_trait + Clone + Send + Sync + 'static,
                <A as $agent_trait>::DecisionType: Send + Sync,
            {
                type Core = $core_type;

                fn decide_and_act(
                    &self,
                    state: &$state_type,
                    rng: &mut dyn rand::RngCore,
                ) -> Vec<$action_type> {
                    let decisions = self.0.decide(&state.financial_system, rng);
                    self.0.act(&decisions)
                }
            }
        };
    }
}