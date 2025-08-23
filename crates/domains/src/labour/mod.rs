use serde::{Deserialize, Serialize};
use sim_core::*;
use sim_macros::SimDomain;

#[derive(Clone, Debug, Serialize, Deserialize, Default, SimDomain)]
pub struct LabourDomain {}

#[derive(Debug, Clone)]
pub struct LabourResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl LabourDomain {
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&self, action: &LabourAction, _state: &SimState) -> LabourResult {

        match action {
            LabourAction::ApplyForJob { market_id, application } => self.execute_apply(market_id.clone(), application.clone()),
            LabourAction::PostJobOffer { market_id, offer } => self.execute_post_offer(market_id.clone(), offer.clone()),
            LabourAction::Fire { firm_id, employee_id } => self.execute_fire(*firm_id, *employee_id),
        }
    }

    fn execute_apply(&self, market_id: LabourMarketId, application: JobApplication) -> LabourResult {
        let effect = StateEffect::Market(MarketEffect::UpdateLabourMarket {
            market_id,
            update: LabourMarketUpdate::AddApplication(application),
        });
        LabourResult { success: true, effects: vec![effect], errors: vec![] }
    }

    fn execute_post_offer(&self, market_id: LabourMarketId, offer: JobOffer) -> LabourResult {
        let effect = StateEffect::Market(MarketEffect::UpdateLabourMarket {
            market_id,
            update: LabourMarketUpdate::AddOffer(offer),
        });
        LabourResult { success: true, effects: vec![effect], errors: vec![] }
    }

    fn execute_fire(&self, firm_id: AgentId, employee_id: AgentId) -> LabourResult {
        let effect = StateEffect::Agent(AgentEffect::TerminateEmployment {
            firm_id,
            consumer_id: employee_id,
        });
        LabourResult { success: true, effects: vec![effect], errors: vec![] }
    }
}