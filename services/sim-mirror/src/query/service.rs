use crate::{
    AgentsCatalogueDigest, BehaviourDigest, DashboardDto, InstrumentRegistryDigest, MarketInfrastructureDigest,
    MetadataDigest, MirrorHandle, StateDelta, StateDigest, StateSnapshot, StatusDigest, TickDetailDigest,
};
use crossbeam_channel::Receiver;
use std::sync::Arc;

#[derive(Clone)]
pub struct QueryService {
    mirror: MirrorHandle,
}

impl QueryService {
    pub fn new(mirror: MirrorHandle) -> Self {
        Self { mirror }
    }

    pub fn latest_snapshot(&self) -> Arc<StateSnapshot> {
        self.mirror.latest()
    }

    pub fn latest_digest(&self) -> Arc<StateDigest> {
        self.latest_snapshot().digest.clone()
    }

    pub fn status(&self) -> StatusDigest {
        self.latest_snapshot().digest.status.clone()
    }

    pub fn dashboard(&self) -> DashboardDto {
        let snapshot = self.latest_snapshot();
        DashboardDto::from(snapshot.digest.as_ref())
    }

    pub fn agent_catalogue(&self) -> Option<AgentsCatalogueDigest> {
        self.latest_snapshot().digest.agents.catalogue.clone()
    }

    pub fn instrument_registry(&self) -> Option<InstrumentRegistryDigest> {
        self.latest_snapshot().digest.instruments.clone()
    }

    pub fn market_infrastructure(&self) -> Option<MarketInfrastructureDigest> {
        self.latest_snapshot().digest.markets.infrastructure.clone()
    }

    pub fn behaviour(&self) -> Option<BehaviourDigest> {
        self.latest_snapshot().digest.behaviour.clone()
    }

    pub fn metadata(&self) -> Option<MetadataDigest> {
        self.latest_snapshot().digest.metadata.clone()
    }

    pub fn behaviour_tick(&self, tick: u32) -> Option<TickDetailDigest> {
        self.latest_snapshot()
            .digest
            .behaviour
            .as_ref()
            .and_then(|behaviour| behaviour.ticks.iter().find(|entry| entry.tick == tick))
            .and_then(|tick_digest| tick_digest.detail.clone())
    }

    pub fn latest_delta(&self) -> Option<StateDelta> {
        self.latest_snapshot().delta.clone()
    }

    pub fn subscribe(&self) -> Receiver<Arc<StateSnapshot>> {
        self.mirror.subscribe()
    }
}
