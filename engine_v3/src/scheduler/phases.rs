use serde::{Deserialize, Serialize};
use sim_core::types::core_utils::time::Session;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Phase {
    Ingest,
    Decide,
    Submit,
    Match,
    Settle,
    MarkRisk,
    Report,

    Cutoff,
    PaymentsFinality,
    ReserveCalc,
    CorridorFacilities,
    AccrualRoll,
    Close,
}

impl Phase {
    pub fn phases_for_session(session: Session) -> Vec<Phase> {
        match session {
            Session::AM | Session::PM => vec![
                Phase::Ingest,
                Phase::Decide,
                Phase::Submit,
                Phase::Match,
                Phase::Settle,
                Phase::MarkRisk,
                Phase::Report,
            ],
            Session::EOD => vec![
                Phase::Cutoff,
                Phase::PaymentsFinality,
                Phase::ReserveCalc,
                Phase::CorridorFacilities,
                Phase::AccrualRoll,
                Phase::Close,
            ],
        }
    }

    pub fn all() -> Vec<Phase> {
        vec![
            Phase::Ingest,
            Phase::Decide,
            Phase::Submit,
            Phase::Match,
            Phase::Settle,
            Phase::MarkRisk,
            Phase::Report,
            Phase::Cutoff,
            Phase::PaymentsFinality,
            Phase::ReserveCalc,
            Phase::CorridorFacilities,
            Phase::AccrualRoll,
            Phase::Close,
        ]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionPlan {
    pub session: Session,
    pub phases: Vec<Phase>,
}

impl SessionPlan {
    pub fn new(session: Session) -> Self {
        Self {
            phases: Phase::phases_for_session(session),
            session,
        }
    }

    pub fn full_day() -> Vec<SessionPlan> {
        vec![
            SessionPlan::new(Session::AM),
            SessionPlan::new(Session::PM),
            SessionPlan::new(Session::EOD),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_am_pm_phases() {
        let am_phases = Phase::phases_for_session(Session::AM);
        let pm_phases = Phase::phases_for_session(Session::PM);

        assert_eq!(am_phases.len(), 7);
        assert_eq!(pm_phases.len(), 7);
        assert_eq!(am_phases, pm_phases);

        assert_eq!(am_phases[0], Phase::Ingest);
        assert_eq!(am_phases[6], Phase::Report);
    }

    #[test]
    fn test_eod_phases() {
        let eod_phases = Phase::phases_for_session(Session::EOD);

        assert_eq!(eod_phases.len(), 6);
        assert_eq!(eod_phases[0], Phase::Cutoff);
        assert_eq!(eod_phases[5], Phase::Close);
    }

    #[test]
    fn test_full_day() {
        let day = SessionPlan::full_day();

        assert_eq!(day.len(), 3);
        assert_eq!(day[0].session, Session::AM);
        assert_eq!(day[1].session, Session::PM);
        assert_eq!(day[2].session, Session::EOD);
    }
}
