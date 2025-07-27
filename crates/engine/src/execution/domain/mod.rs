pub mod banking;
pub mod production;
pub mod trading;

use crate::execution::effects::ExecutionResult;
use crate::state::SimState;
use shared::*;
use std::collections::HashMap;

pub trait ExecutionDomain: Send + Sync {
    fn name(&self) -> &'static str;
    fn can_handle(&self, action: &SimAction) -> bool;
    fn validate(&self, action: &SimAction, state: &SimState) -> bool;
    fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult;
}

pub struct DomainRegistry {
    domains: Vec<Box<dyn ExecutionDomain>>,
}

impl DomainRegistry {
    pub fn new() -> Self {
        Self {
            domains: vec![
                Box::new(banking::BankingDomain::new()),
                Box::new(trading::TradingDomain::new()),
                Box::new(production::ProductionDomain::new()),
            ],
        }
    }

    pub fn execute(&self, action: &SimAction, state: &SimState) -> ExecutionResult {
        for domain in &self.domains {
            if domain.can_handle(action) {
                if domain.validate(action, state) {
                    return domain.execute(action, state);
                } else {
                    return ExecutionResult {
                        success: false,
                        effects: vec![],
                        errors: vec![format!("Validation failed for action: {}", action.name())],
                    };
                }
                return domain.execute(action, state);
            }
        }

        ExecutionResult {
            success: false,
            effects: vec![],
            errors: vec![format!("No domain can handle action: {}", action.name())],
        }
    }
}

#[cfg(test)]
mod test_migrate {
    use super::*;
    use crate::state::SimState;
    use shared::SimAction;
    use rand::prelude::*;
    use crate::{TransactionExecutor, AgentFactory};
    
    fn t_econ() -> SimState {
        let mut rng = StdRng::from_os_rng();
        let mut ss = SimState::default();
        let mut factory = AgentFactory::new(&mut ss, &mut rng);
        
        let mut banks = HashMap::new();
        for _ in 0..2 {
            let bank = factory.create_bank();
            banks.insert(bank.id.clone(), bank);
        }
        for i in 0..2 {
            factory.create_firm(banks.iter().nth(i).unwrap().1.id.clone());
        }
        for i in 0..2 {
            factory.create_consumer(banks.iter().nth(i).unwrap().1.id.clone());
        }
        ss
    }
    fn t_econ_w_liquidity() -> SimState {
        let mut state = t_econ();
        let e = TransactionExecutor::execute_inject_liquidity(&state);
        TransactionExecutor::apply_effects(&e.effects, &mut state);
        state
    }
    fn get_first_agents(state: &SimState) -> (&Consumer, &Firm, &Bank) {
        let consumer = state.consumers.first().unwrap();
        let firm = state.firms.first().unwrap();
        let bank = state.financial_system.commercial_banks.values().next().unwrap();
        (consumer, firm, bank)
    }
    fn get_second_agents(state: &SimState) -> (&Consumer, &Firm, &Bank) {
        let consumer = state.consumers.get(1).unwrap();
        let firm = state.firms.get(1).unwrap();
        let bank = state.financial_system.commercial_banks.values().nth(1).unwrap();
        (consumer, firm, bank)
    }

    #[test]
    fn test_deposit_valid(){
        let state = t_econ_w_liquidity();
        let (consumer, _, bank) = get_first_agents(&state);
        let (consumer2, firm2, bank2) = get_second_agents(&state);
        
        let action = SimAction::Deposit {
            agent_id: consumer.id.clone(),
            bank: bank.id.clone(),
            amount: 100.0,
        };
        let action2 = SimAction::Deposit {
            agent_id: consumer2.id.clone(),
            bank: bank2.id.clone(),
            amount: 501.0,
        };
        let domain = banking::BankingDomain::new();
        assert!(domain.validate(&action, &state), "Deposit action should be valid");
        assert!(!domain.validate(&action2, &state), "Deposit action should be invalid due to insufficient funds");
    }

}