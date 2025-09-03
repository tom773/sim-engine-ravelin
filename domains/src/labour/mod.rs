use serde::{Deserialize, Serialize};
use sim_core::*;
use crate::{Any, inventory, Domain, DomainResult, ResolutionContext, ResolutionResult, ResolutionPhase};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LabourDomain {}

impl LabourDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for LabourDomain {
    fn name(&self) -> &'static str { 
        "Labour" 
    }

    fn resolve_intention(&self, intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::ApplyForJob { agent_id: _, market_id, application } => {
                vec![SimAction::Labour(LabourAction::ApplyForJob { 
                    market_id: market_id.clone(), 
                    application: application.clone() 
                })]
            },
            

            
            _ => return None,
        };
        
        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::ApplyForJob { .. } => Some(ResolutionPhase::Independent),
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, _state: &SimState) -> DomainResult {
        let labour_action = match action {
            SimAction::Labour(action) => action,
            _ => return DomainResult::failure(vec!["Not a labour action".to_string()]),
        };

        match labour_action {
            LabourAction::ApplyForJob { market_id, application } => {
                self.execute_apply(market_id.clone(), application.clone())
            },
            LabourAction::PostJobOffer { market_id, offer } => {
                self.execute_post_offer(market_id.clone(), offer.clone())
            },
            LabourAction::Fire { firm_id, employee_id } => {
                self.execute_fire(*firm_id, *employee_id)
            },
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl LabourDomain {
    fn execute_apply(&self, market_id: LabourMarketId, application: JobApplication) -> DomainResult {
        let effect = StateEffect::Market(MarketEffect::UpdateLabourMarket {
            market_id,
            update: LabourMarketUpdate::AddApplication(application),
        });
        
        DomainResult::success(vec![effect])
    }

    fn execute_post_offer(&self, market_id: LabourMarketId, offer: JobOffer) -> DomainResult {
        let effect = StateEffect::Market(MarketEffect::UpdateLabourMarket {
            market_id,
            update: LabourMarketUpdate::AddOffer(offer),
        });
        
        DomainResult::success(vec![effect])
    }

    fn execute_fire(&self, firm_id: AgentId, employee_id: AgentId) -> DomainResult {
        let effect = StateEffect::Agent(AgentEffect::TerminateEmployment {
            firm_id,
            consumer_id: employee_id,
        });
        
        DomainResult::success(vec![effect])
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Labour",
        constructor: || Box::new(LabourDomain::new()),
    }
}