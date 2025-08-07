use crate::*;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::collections::HashMap;
use chrono::{Datelike, NaiveDate};
pub struct SimulationEngine {
    pub state: SimState,
    pub domain_registry: DomainRegistry,
    pub decision_models: HashMap<AgentId, Box<dyn DecisionModel>>,
}

impl SimulationEngine {
    pub fn new(state: SimState) -> Self {
        Self { state, domain_registry: DomainRegistry::new(), decision_models: HashMap::new() }
    }

    pub fn run_initialization(&mut self) {
        for agent_id in self.state.agents.banks.keys() {
            if let Some(bs) = self.state.financial_system.get_bs_by_id(agent_id) {
                println!("[INITIALIZATION] Bank {:?} has balance sheet: {:#?}", agent_id, bs);
            }
        }
    }

    fn collect_actions(&self, rng: &mut dyn RngCore) -> Vec<SimAction> {
        let mut all_actions = Vec::new();

        for agent_id in self.state.agents.all_agent_ids() {
            if let Some(model) = self.decision_models.get(&agent_id) {
                if let Some(agent) = self.state.agents.get_agent_as_any(&agent_id) {
                    all_actions.extend(model.decide(agent, &self.state, rng));
                }
            }
        }

        let government = &self.state.financial_system.government;
        if let Some(model) = self.decision_models.get(&government.id) {
            all_actions.extend(model.decide(government, &self.state, rng));
        }
        all_actions
    }

    pub fn tick(&mut self, rng: &mut dyn RngCore) -> TickResult {
        let mut actions = self.process_financial_updates();
        actions.extend(self.collect_actions(rng));

        let effects = self.execute_actions(&actions);
        if let Err(e) = self.state.apply_effects(&effects) {
            println!("[ERROR] applying action effects: {}", e);
        }
        let trades = self.state.financial_system.exchange.clear_markets();

        let settlement_effects = self.settle_trades(&trades);
        if let Err(e) = self.state.apply_effects(&settlement_effects) {
            println!("[ERROR] applying settlement effects: {}", e);
        }

        self.state.advance_time();

        TickResult { tick_number: self.state.ticknum, actions, effects, trades }
    }

    fn execute_actions(&self, actions: &[SimAction]) -> Vec<StateEffect> {
        let mut all_effects = Vec::new();
        for action in actions {
            let effects = self.domain_registry.execute(action, &self.state);
            all_effects.extend(effects);
        }
        all_effects
    }
    fn settle_trades(&self, trades: &[Trade]) -> Vec<StateEffect> {
        let mut all_effects = Vec::new();
        for trade in trades {
            let result = self.domain_registry.settle_financial_trade(trade, &self.state);

            if result.success {
                all_effects.extend(result.effects);
            } else {
                println!("[Executor] Trade settlement failed: {:?}", result.errors);
            }
        }
        all_effects
    }
    fn process_financial_updates(&self) -> Vec<SimAction> {
        let mut actions = Vec::new();
        let current_date = self.state.current_date;

        for (instrument_id, instrument) in &self.state.financial_system.instruments {
            if self.is_interest_bearing(instrument) {
                actions.push(SimAction::Settlement(SettlementAction::AccrueInterest {
                    instrument_id: *instrument_id,
                }));
            }

            if self.is_interest_payment_date(current_date) && instrument.accrued_interest > 0.0 {
                actions.push(SimAction::Settlement(SettlementAction::PayInterest {
                    instrument_id: *instrument_id,
                }));
            }
            
            if let Some(bond_details) = instrument.details.as_any().downcast_ref::<BondDetails>() {
                if self.is_coupon_payment_date(current_date, instrument, bond_details) {
                     actions.push(SimAction::Settlement(SettlementAction::ProcessCouponPayment { instrument_id: *instrument_id }));
                }
            }
        }

        actions
    }

    fn is_interest_bearing(&self, instrument: &FinancialInstrument) -> bool {
        instrument.details.as_any().is::<DemandDepositDetails>()
            || instrument.details.as_any().is::<SavingsDepositDetails>()
            || instrument.details.as_any().is::<BondDetails>()
    }

    fn is_interest_payment_date(&self, date: NaiveDate) -> bool {
        let next_day = date + chrono::Duration::days(1);
        date.month() != next_day.month()
    }
    
    fn is_coupon_payment_date(&self, date: NaiveDate, instrument: &FinancialInstrument, bond_details: &BondDetails) -> bool {
        let months_between_payments = (12 / bond_details.frequency) as u32;
        let months_since_origination = (date.year() - instrument.originated_date.year()) * 12 + (date.month() as i32 - instrument.originated_date.month() as i32);
        
        instrument.originated_date.day() == date.day() &&
        months_since_origination > 0 &&
        months_since_origination as u32 % months_between_payments == 0
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TickResult {
    pub tick_number: u32,
    pub actions: Vec<SimAction>,
    pub effects: Vec<StateEffect>,
    pub trades: Vec<Trade>,
}
