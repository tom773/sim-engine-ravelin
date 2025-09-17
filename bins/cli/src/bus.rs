use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sim_core::SimEvent; // your enum
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    Tick { tick: u32, date: NaiveDate, events: Vec<SimEvent> },
    Heartbeat { tick: u32, date: NaiveDate },
}

#[derive(Clone)]
pub struct EventsBus {
    pub tx: broadcast::Sender<ServerEvent>,
    history: Arc<Mutex<VecDeque<(u32, NaiveDate, Arc<Vec<SimEvent>>)>>>,
    cap: usize,
}

impl EventsBus {
    pub fn new(cap: usize) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx, history: Arc::new(Mutex::new(VecDeque::new())), cap }
    }
    pub fn push_tick(&self, tick: u32, date: NaiveDate, events: Vec<SimEvent>) {
        let events = Arc::new(events);
        {
            let mut h = self.history.lock().unwrap();
            h.push_back((tick, date, events.clone()));
            while h.len() > self.cap {
                h.pop_front();
            }
        }
        let _ = self.tx.send(ServerEvent::Tick { tick, date, events: Arc::unwrap_or_clone(events) });
    }
    pub fn latest_n(&self, n: usize) -> Vec<(u32, NaiveDate, Arc<Vec<SimEvent>>)> {
        let h = self.history.lock().unwrap();
        h.iter().rev().take(n).cloned().collect::<Vec<_>>().into_iter().rev().collect()
    }
    pub fn get(&self, tick: u32) -> Option<Arc<Vec<SimEvent>>> {
        self.history.lock().unwrap().iter().find(|(t, _, _)| *t == tick).map(|(_, _, v)| v.clone())
    }
    pub fn clear(&self) {
        self.history.lock().unwrap().clear();
    }
}
