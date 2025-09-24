use crate::{DashboardDto, MirrorHandle, StateDelta, StateDigest, StateSnapshot, StatusDigest};
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

    pub fn latest_delta(&self) -> Option<StateDelta> {
        self.latest_snapshot().delta.clone()
    }

    pub fn subscribe(&self) -> Receiver<Arc<StateSnapshot>> {
        self.mirror.subscribe()
    }
}
