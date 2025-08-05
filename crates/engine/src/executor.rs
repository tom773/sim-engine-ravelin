use serde::{Deserialize, Serialize};
use sim_prelude::*;
use crate::registry::DomainRegistry;
use rand::RngCore;

pub struct SimulationEngine {
    pub state: SimState,
    pub domain_registry: DomainRegistry,
}

impl SimulationEngine {
    pub fn new(state: SimState) -> Self {
        Self { state, domain_registry: DomainRegistry::new() }
    }

    pub fn tick(&mut self, rng: &mut dyn RngCore) -> TickResult {
        let decisions = self.collect_decisions(rng);

        let actions = self.convert_decisions_to_actions(&decisions);

        let effects = self.execute_actions(&actions);
        if let Err(e) = self.state.apply_effects(&effects) {
            println!("[ERROR] applying effects: {}", e);
        }

        let trades = self.state.financial_system.exchange.clear_markets();
        let settlement_effects = self.settle_trades(&trades);
        if let Err(e) = self.state.apply_effects(&settlement_effects) {
            println!("[ERROR] applying settlement effects: {}", e);
        }

        self.state.advance_time();

        TickResult {
            tick_number: self.state.ticknum,
            decisions_count: decisions.len(),
            actions_count: actions.len(),
            effects_count: effects.len() + settlement_effects.len(),
        }
    }

    fn collect_decisions(&self, rng: &mut dyn RngCore) -> Vec<AgentDecision> {
        let mut all_decisions = Vec::new();
        let fs = &self.state.financial_system;

        let bank_model = BasicBankDecisionModel::default();
        for bank in self.state.agents.banks.values() {
            let decisions = bank_model.decide(bank, fs, rng);
            for decision in decisions {
                all_decisions.push(AgentDecision::Bank { agent_id: bank.id, decision });
            }
        }

        let consumer_model = BasicConsumerDecisionModel::default();
        for consumer in self.state.agents.consumers.values() {
            let decisions = consumer_model.decide(consumer, fs, rng);
            for decision in decisions {
                all_decisions.push(AgentDecision::Consumer { agent_id: consumer.id, decision });
            }
        }

        let firm_model = BasicFirmDecisionModel::default();
        for firm in self.state.agents.firms.values() {
            let decisions = firm_model.decide(firm, fs, rng);
            for decision in decisions {
                all_decisions.push(AgentDecision::Firm { agent_id: firm.id, decision });
            }
        }

        all_decisions
    }

    fn convert_decisions_to_actions(&self, decisions: &[AgentDecision]) -> Vec<SimAction> {
        let mut actions = Vec::new();

        for agent_decision in decisions {
            match agent_decision {
                AgentDecision::Bank { decision, .. } => match decision {
                    BankDecision::LendOvernight { amount_dollars, min_annual_rate_bps } => {
                        let daily_rate = self.state.financial_system.exchange.financial_markets
                            [&FinancialMarketId::SecuredOvernightFinancing]
                            .market_id
                            .annual_bps_to_daily_rate(*min_annual_rate_bps);
                        let price = 1.0 / (1.0 + daily_rate);
                        actions.push(SimAction::Trading(TradingAction::PostAsk {
                            agent_id: agent_decision.agent_id(),
                            market_id: MarketId::Financial(FinancialMarketId::SecuredOvernightFinancing),
                            quantity: *amount_dollars,
                            price,
                        }));
                    }
                    BankDecision::BorrowOvernight { amount_dollars, max_annual_rate_bps } => {
                        let daily_rate = self.state.financial_system.exchange.financial_markets
                            [&FinancialMarketId::SecuredOvernightFinancing]
                            .market_id
                            .annual_bps_to_daily_rate(*max_annual_rate_bps);
                        let price = 1.0 / (1.0 + daily_rate);
                        actions.push(SimAction::Trading(TradingAction::PostBid {
                            agent_id: agent_decision.agent_id(),
                            market_id: MarketId::Financial(FinancialMarketId::SecuredOvernightFinancing),
                            quantity: *amount_dollars,
                            price,
                        }));
                    }
                    _ => {}
                },
                AgentDecision::Consumer { decision, .. } => match decision {
                    ConsumerDecision::Spend { seller_id, amount, good_id, .. } => {
                        actions.push(SimAction::Consumption(ConsumptionAction::Purchase {
                            agent_id: agent_decision.agent_id(),
                            seller: *seller_id,
                            good_id: *good_id,
                            amount: *amount,
                        }));
                    }
                    ConsumerDecision::Save { agent_id, amount } => {
                        if let Some(consumer) = self.state.agents.consumers.get(agent_id) {
                            actions.push(SimAction::Banking(BankingAction::Deposit {
                                agent_id: *agent_id,
                                bank: consumer.bank_id,
                                amount: *amount,
                            }));
                        }
                    }
                    _ => {}
                },
                AgentDecision::Firm { decision, .. } => match decision {
                    FirmDecision::Produce { recipe_id, batches } => {
                        actions.push(SimAction::Production(ProductionAction::Produce {
                            agent_id: agent_decision.agent_id(),
                            recipe_id: *recipe_id,
                            batches: *batches,
                        }));
                    }
                    FirmDecision::Hire { count } => {
                        actions.push(SimAction::Production(ProductionAction::Hire {
                            agent_id: agent_decision.agent_id(),
                            count: *count,
                        }));
                    }
                    FirmDecision::PayWages { employee, amount } => {
                        actions.push(SimAction::Banking(BankingAction::PayWages {
                            agent_id: agent_decision.agent_id(),
                            employee: *employee,
                            amount: *amount,
                        }));
                    }
                    FirmDecision::SellInventory { good_id, quantity } => {
                        let price = 100.0;
                        actions.push(SimAction::Trading(TradingAction::PostAsk {
                            agent_id: agent_decision.agent_id(),
                            market_id: MarketId::Goods(*good_id),
                            quantity: *quantity,
                            price,
                        }));
                    }
                    _ => {}
                },
            }
        }
        actions
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
        let mut effects = Vec::new();
        for trade in trades {
            effects.push(StateEffect::Market(MarketEffect::ExecuteTrade(trade.clone())));
        }
        effects
    }
}

#[derive(Debug, Clone)]
pub enum AgentDecision {
    Bank { agent_id: AgentId, decision: BankDecision },
    Consumer { agent_id: AgentId, decision: ConsumerDecision },
    Firm { agent_id: AgentId, decision: FirmDecision },
}

impl AgentDecision {
    pub fn agent_id(&self) -> AgentId {
        match self {
            AgentDecision::Bank { agent_id, .. } => *agent_id,
            AgentDecision::Consumer { agent_id, .. } => *agent_id,
            AgentDecision::Firm { agent_id, .. } => *agent_id,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TickResult {
    pub tick_number: u32,
    pub decisions_count: usize,
    pub actions_count: usize,
    pub effects_count: usize,
}