use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
};

use crate::{
    action::*, 
    effect::*,
};

pub trait WorldState: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait Domain<W: WorldState>: Send + Sync {
    fn execute(&self, act: &dyn AnyAction, world: &mut W)
        -> Result<Vec<Box<dyn AnyEffect>>, ActionError>;
}

pub struct GenericDomain<A, W>
where
    A: Action + 'static,
    W: WorldState,
{
    _phantom: PhantomData<(A, W)>,
}

impl<A, W> GenericDomain<A, W>
where
    A: Action + 'static,
    W: WorldState,
{
    pub const fn new() -> Self {
        Self { _phantom: PhantomData }
    }
}

impl<A, W> Domain<W> for GenericDomain<A, W>
where
    A: Action + 'static,
    W: WorldState,
{
    fn execute(
        &self,
        act: &dyn AnyAction,
        world: &mut W,
    ) -> Result<Vec<Box<dyn AnyEffect>>, ActionError> {
        let a = act
            .as_any()
            .downcast_ref::<A>()
            .ok_or_else(|| ActionError::ExecutionError("type mismatch".into()))?;

        a.validate(world)?;
        Ok(a.execute(world)
            .into_iter()
            .map(|e| Box::new(e) as Box<dyn AnyEffect>)
            .collect())
    }
}

pub struct Executor<W: WorldState> {
    domains: HashMap<TypeId, Box<dyn Domain<W>>>,
    _phantom: PhantomData<W>,
}

impl<W: WorldState + 'static> Executor<W> {
    pub fn new() -> Self {
        Self { domains: HashMap::new(), _phantom: PhantomData }
    }

    pub fn register_action<A: Action + 'static>(&mut self)
    where
        A::Effect: 'static,
    {
        let id = TypeId::of::<A>();
        self.domains
            .insert(id, Box::new(GenericDomain::<A, W>::new()));
    }

    pub fn run(&self, act: &dyn AnyAction, world: &mut W) -> Result<(), ActionError> {
        if let Some(domain) = self.domains.get(&act.as_any().type_id()) {
            let effects = domain.execute(act, world)?;
            for eff in effects {
                eff.apply(world).map_err(|e| {
                    ActionError::ExecutionError(format!("effect failed: {e:?}"))
                })?;
            }
            Ok(())
        } else {
            Err(ActionError::ExecutionError("no domain registered".into()))
        }
    }
}