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

    pub fn execute(&self, action: &LabourAction, state: &SimState) -> LabourResult {

        match action {
            LabourAction::ApplyForJob { market_id, application } => self.execute_apply(market_id.clone(), application.clone()),
            LabourAction::PostJobOffer { market_id, offer } => self.execute_post_offer(market_id.clone(), offer.clone()),
            LabourAction::ClearLabourMarket { market_id } => self.execute_clear_market(market_id, state),
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

    fn execute_clear_market(&self, market_id: &LabourMarketId, state: &SimState) -> LabourResult {
        let market = match state.financial_system.exchange.labour_markets.get(market_id) {
            Some(m) => m,
            None => return LabourResult { success: false, effects: vec![], errors: vec!["Market not found".to_string()] },
        };

        let mut effects = Vec::new();
        let mut applications = market.job_applications.clone();
        let mut offers = market.job_offers.clone();

        applications.sort_by(|a, b| a.reservation_wage.partial_cmp(&b.reservation_wage).unwrap());
        offers.sort_by(|a, b| b.wage_rate.partial_cmp(&a.wage_rate).unwrap());

        let mut filled_applications = Vec::new();

        for application in applications.iter() {
            if state.agents.consumers.get(&application.consumer_id).map_or(false, |c| c.employed_by.is_some()) {
                continue;
            }

            for offer in offers.iter_mut() {
                if offer.quantity > 0 && offer.wage_rate >= application.reservation_wage {
                    let contract = EmploymentContract {
                        employee_id: application.consumer_id,
                        wage_rate: offer.wage_rate,
                        hours: application.hours_desired.min(offer.hours_required),
                        start_date: state.current_date,
                    };

                    effects.push(StateEffect::Agent(AgentEffect::EstablishEmployment {
                        firm_id: offer.firm_id,
                        consumer_id: application.consumer_id,
                        contract: contract.clone(),
                    }));

                    offer.quantity -= 1;
                    filled_applications.push(application.application_id);

                    break;
                }
            }
        }

        effects.push(StateEffect::Market(MarketEffect::ClearLabourMarketOrders {
            market_id: market_id.clone(),
            filled_applications,
        }));

        LabourResult { success: true, effects, errors: vec![] }
    }
}