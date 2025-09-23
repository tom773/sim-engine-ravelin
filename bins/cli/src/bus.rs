use chrono::NaiveDate;
use parking_lot::{Mutex, MutexGuard};
use serde::{Deserialize, Serialize};
use sim_core::{CompactEvent, SimEvent, TickEventSummary};
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::broadcast;

const DEFAULT_INLINE_EVENT_LIMIT: usize = 5000;

const LOCK_CONTENTION_METRIC: &str = "engine.events_bus.history_lock_contention";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    Tick { tick: u32, date: NaiveDate, summary: TickEventSummary, truncated: bool, compact_events: Vec<CompactEvent> },
    Heartbeat { tick: u32, date: NaiveDate },
}

#[derive(Clone)]
pub struct EventsBus {
    pub tx: broadcast::Sender<ServerEvent>,
    history: Arc<Mutex<VecDeque<(u32, NaiveDate, Arc<Vec<SimEvent>>, Arc<TickEventSummary>)>>>,
    cap: usize,
    inline_event_limit: usize,
}

impl EventsBus {
    pub fn new(cap: usize) -> Self {
        Self::with_inline_limit(cap, DEFAULT_INLINE_EVENT_LIMIT)
    }

    pub fn with_inline_limit(cap: usize, inline_event_limit: usize) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx, history: Arc::new(Mutex::new(VecDeque::new())), cap, inline_event_limit }
    }
    pub fn push_tick(&self, tick: u32, date: NaiveDate, events: Vec<SimEvent>) {
        let summary_struct = TickEventSummary::from_events(&events);
        let tick_event = self.make_tick_event(tick, date, &events, &summary_struct);

        let summary = Arc::new(summary_struct);
        let events = Arc::new(events);
        {
            let mut h = self.lock_history("push_tick");
            h.push_back((tick, date, events.clone(), summary.clone()));
            while h.len() > self.cap {
                h.pop_front();
            }
        }
        let _ = self.tx.send(tick_event);
    }
    pub fn latest_n(&self, n: usize) -> Vec<(u32, NaiveDate, Arc<Vec<SimEvent>>, Arc<TickEventSummary>)> {
        let h = self.lock_history("latest_n");
        h.iter().rev().take(n).cloned().collect::<Vec<_>>().into_iter().rev().collect()
    }
    pub fn get(&self, tick: u32) -> Option<Arc<Vec<SimEvent>>> {
        self.lock_history("get").iter().find(|(t, _, _, _)| *t == tick).map(|(_, _, v, _)| v.clone())
    }
    pub fn clear(&self) {
        self.lock_history("clear").clear();
    }

    pub fn to_server_event(
        &self, tick: u32, date: NaiveDate, events: &Arc<Vec<SimEvent>>, summary: &Arc<TickEventSummary>,
    ) -> ServerEvent {
        self.make_tick_event(tick, date, events.as_ref().as_slice(), summary.as_ref())
    }

    fn make_tick_event(
        &self, tick: u32, date: NaiveDate, events: &[SimEvent], summary: &TickEventSummary,
    ) -> ServerEvent {
        let truncated = events.len() > self.inline_event_limit;
        let compact_events: Vec<CompactEvent> =
            events.iter().take(self.inline_event_limit).map(|event| event.compact()).collect();

        ServerEvent::Tick { tick, date, summary: summary.clone(), truncated, compact_events }
    }

    fn lock_history(
        &self, callsite: &'static str,
    ) -> MutexGuard<'_, VecDeque<(u32, NaiveDate, Arc<Vec<SimEvent>>, Arc<TickEventSummary>)>> {
        if let Some(guard) = self.history.try_lock() {
            return guard;
        }
        metrics::counter!(LOCK_CONTENTION_METRIC, 1u64, "callsite" => callsite);
        self.history.lock()
    }
}
