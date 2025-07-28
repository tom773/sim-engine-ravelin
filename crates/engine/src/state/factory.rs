use fake::Fake;
use fake::faker::company::en::*;
use rand::prelude::*;
use shared::*;
use uuid::Uuid;

use crate::SimState;

pub struct AgentFactory<'a> {
    pub ss: &'a mut SimState,
    pub rng: &'a mut StdRng,
}

impl<'a> AgentFactory<'a> {
    pub fn new(ss: &'a mut SimState, rng: &'a mut StdRng) -> Self {
        Self { ss, rng }
    }

    pub fn create_consumer(&mut self, bank_id: AgentId) -> Consumer {
        let agent_id = AgentId(Uuid::new_v4());

        self.ss
            .financial_system
            .balance_sheets
            .insert(agent_id.clone(), BalanceSheet::new(agent_id.clone()));

        let age = 35;
        let annual_income = 60_000.0;
        let propensity_to_consume = 0.7;

        let decision_model = Box::new(BasicDecisionModel {
            propensity_to_consume,
        });

        let mut c = Consumer::new(age, agent_id.clone(), bank_id.clone(), decision_model);
        c.income = annual_income / 52.0;

        self.ss.consumers.push(c.clone());
        c
    }

    pub fn create_bank(&mut self) -> Bank {
        let bank_names = ["Bank", "Financial", "Savings & Loan", "Credit Union"];
        let name = format!(
            "{} {}",
            CompanyName().fake::<String>(),
            bank_names[self.rng.random_range(0..bank_names.len())]
        );
        let lending_spread = 250.0;
        let deposit_spread = 50.0;

        let bank = Bank::new(name, lending_spread, deposit_spread);
        self.ss
            .financial_system
            .balance_sheets
            .insert(bank.id.clone(), BalanceSheet::new(bank.id.clone()));

        let initial_reserves = 1_000.0;
        let reserves = reserves!(
            bank.id.clone(),
            self.ss.financial_system.central_bank.id.clone(),
            initial_reserves,
            self.ss.financial_system.central_bank.policy_rate - 50.0,
            0 // originated_date = 0 for initial setup
        );

        // Use the financial system's create_instrument method
        self.ss
            .financial_system
            .create_instrument(reserves)
            .expect("Failed to create initial reserves");

        self.ss
            .financial_system
            .commercial_banks
            .insert(bank.id.clone(), bank.clone());
        bank
    }

    pub fn create_firm(&mut self, bank_id: AgentId) -> Firm {
        let firm_id = AgentId(Uuid::new_v4());
        let firm_name = CompanyName().fake::<String>();

        self.ss
            .financial_system
            .balance_sheets
            .insert(firm_id.clone(), BalanceSheet::new(firm_id.clone()));

        self.ss
            .financial_system
            .exchange
            .financial_market_mut(&FinancialMarketId::SecuredOvernightFinancing)
            .unwrap()
            .post_ask(firm_id.clone(), 2000.0, 300.0);

        let mut f = Firm::new(firm_id, bank_id, firm_name);
        f.employees = 5;
        f.wage_rate = 20.0;
        f.productivity = 2.0;

        self.ss.firms.push(f.clone());
        f
    }
}

#[cfg(test)]
mod test_init {
    use crate::initialize_economy;
    use super::*;
    use crate::{TransactionExecutor, SimConfig};

    #[test]
    fn test_bank(){
        let ss = initialize_economy(&SimConfig::default(), &mut StdRng::from_os_rng());
        ss.financial_system.commercial_banks
            .values()
            .for_each(|b| {
                assert!(b.get_reserves(&ss.financial_system) == 1000.0, "Initial reserves should be 1000.0");
            });
    }

    #[test]
    fn test_deposit_over_reserves(){
        let mut ss = initialize_economy(&SimConfig::default(), &mut StdRng::from_os_rng());
        let bank_id = ss.financial_system.commercial_banks.values().next().unwrap().id.clone();
        let consumer_id = ss.consumers.first().unwrap().id.clone();

        let cash = cash!(consumer_id.clone(), 30000.0, ss.financial_system.central_bank.id.clone(), 0);
        ss.financial_system.create_instrument(cash).unwrap();
        assert!(ss.financial_system.get_cash_assets(&consumer_id) == 30000.0, "Consumer should have 2000 cash");

        let action = SimAction::Deposit {
            agent_id: consumer_id.clone(),
            bank: bank_id.clone(),
            amount: 30000.0,
        };
        let er = TransactionExecutor::execute_action(&action, &mut ss);
        assert!(er.success, "Deposit should succeed even if it exceeds initial reserves");
        println!("Deposit executed successfully, Effects: {:?}", er.effects.iter().map(|e| e.name().to_string()).collect::<Vec<_>>());
    }
}
