use serde::{Deserialize, Serialize};
use sim_core::*;
use crate::{Any, Domain, DomainResult, DomainValidator, ResolutionContext, ResolutionResult, ResolutionPhase};
extern crate inventory;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankingDomain {}

impl BankingDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for BankingDomain {
    fn name(&self) -> &'static str { 
        "Banking" 
    }

    fn resolve_intention(&self, intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::DepositFunds { agent_id, bank, amount } => {
                vec![SimAction::Banking(BankingAction::Deposit { 
                    agent_id: *agent_id, bank: *bank, amount: *amount 
                })]
            },
            
            SimIntention::WithdrawFunds { agent_id, bank, amount } => {
                vec![SimAction::Banking(BankingAction::Withdraw { 
                    agent_id: *agent_id, bank: *bank, amount: *amount 
                })]
            },
            
            SimIntention::PayWages { employer, employee, amount } => {
                vec![SimAction::Banking(BankingAction::PayWages { 
                    agent_id: *employer, employee: *employee, amount: *amount 
                })]
            },
            
            SimIntention::CollectTaxes { government_id, target, amount } => {
                vec![SimAction::Banking(BankingAction::Transfer { 
                    from: *target, to: *government_id, amount: *amount 
                })]
            },
            
            SimIntention::InjectLiquidity => {
                vec![SimAction::Banking(BankingAction::InjectLiquidity)]
            },
            
            SimIntention::LendExcessReserves { agent_id, amount, target_rate_bps } => {
                self.resolve_reserve_lending(*agent_id, *amount, *target_rate_bps)
            },

            SimIntention::BorrowReserves { agent_id, amount, target_rate_bps } => {
                self.resolve_reserve_borrowing(*agent_id, *amount, *target_rate_bps)
            },
            
            _ => return None,
        };
        
        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::DepositFunds { .. } |
            SimIntention::WithdrawFunds { .. } |
            SimIntention::PayWages { .. } |
            SimIntention::CollectTaxes { .. } |
            SimIntention::InjectLiquidity => Some(ResolutionPhase::Independent),
            
            SimIntention::LendExcessReserves { .. } |
            SimIntention::BorrowReserves { .. } => Some(ResolutionPhase::Market),
            
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let banking_action = match action {
            SimAction::Banking(action) => action,
            _ => return DomainResult::failure(vec!["Not a banking action".to_string()]),
        };

        if let Err(error) = self.validate(banking_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match banking_action {
            BankingAction::Deposit { agent_id, bank, amount } => {
                self.execute_deposit(*agent_id, *bank, *amount)
            },
            BankingAction::Withdraw { agent_id, bank, amount } => {
                self.execute_withdraw(*agent_id, *bank, *amount)
            },
            BankingAction::Transfer { from, to, amount } => {
                self.execute_transfer(*from, *to, *amount)
            },
            BankingAction::PayWages { agent_id, employee, amount } => {
                self.execute_pay_wages(*agent_id, *employee, *amount)
            },
            BankingAction::UpdateReserves { bank: _, amount_change: _ } => {
                DomainResult::failure(vec!["Reserve updates not yet implemented".to_string()])
            },
            BankingAction::InjectLiquidity => {
                self.execute_inject_liquidity(state)
            },
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl BankingDomain {
    fn resolve_reserve_lending(&self, agent_id: AgentId, amount: f64, target_rate_bps: BasisPoints) -> Vec<SimAction> {
        let market_id = FinancialMarketId::FederalFundsOvernight;
        let daily_rate = market_id.annual_bps_to_daily_rate(target_rate_bps);
        let price = 1.0 / (1.0 + daily_rate);

        vec![SimAction::Trading(TradingAction::PostAsk {
            agent_id,
            market_id: MarketId::Financial(market_id),
            quantity: amount,
            price,
        })]
    }

    fn resolve_reserve_borrowing(&self, agent_id: AgentId, amount: f64, target_rate_bps: BasisPoints) -> Vec<SimAction> {
        let market_id = FinancialMarketId::FederalFundsOvernight;
        let daily_rate = market_id.annual_bps_to_daily_rate(target_rate_bps);
        let price = 1.0 / (1.0 + daily_rate);

        vec![SimAction::Trading(TradingAction::PostBid {
            agent_id,
            market_id: MarketId::Financial(market_id),
            quantity: amount,
            price,
        })]
    }
}

impl BankingDomain {
    fn validate(&self, action: &BankingAction, state: &SimState) -> Result<(), String> {
        match action {
            BankingAction::Deposit { agent_id, bank, amount } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::agent_exists(*agent_id, state)?;
                DomainValidator::bank_exists(*bank, state)?;

                let available_cash = state.financial_system.get_cash_assets(agent_id);
                if available_cash < *amount {
                    return Err(format!(
                        "Insufficient cash for deposit: agent has ${:.2}, needs ${:.2}",
                        available_cash, amount
                    ));
                }
                Ok(())
            }
            BankingAction::Withdraw { agent_id, bank, amount } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::agent_exists(*agent_id, state)?;
                DomainValidator::bank_exists(*bank, state)?;
                
                let available_deposits = state.financial_system.get_deposits_at_bank(agent_id, bank);
                if available_deposits < *amount {
                    return Err(format!(
                        "Insufficient deposits for withdrawal: agent has ${:.2}, needs ${:.2}",
                        available_deposits, amount
                    ));
                }
                Ok(())
            }
            BankingAction::Transfer { from, to, amount }
            | BankingAction::PayWages { agent_id: from, employee: to, amount } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::agent_exists(*from, state)?;
                DomainValidator::agent_exists(*to, state)?;
                let available_funds = state.financial_system.get_liquid_assets(from);
                if available_funds < *amount {
                     return Err(format!(
                        "Insufficient liquid assets for transfer: agent has ${:.2}, needs ${:.2}",
                        available_funds, amount
                    ));
                }
                Ok(())
            }
            BankingAction::UpdateReserves { bank, amount_change: _ } => {
                DomainValidator::bank_exists(*bank, state)
            }
            BankingAction::InjectLiquidity => Ok(()),
        }
    }
}

impl BankingDomain {
    fn execute_deposit(&self, depositor: AgentId, bank: AgentId, amount: f64) -> DomainResult {
        let effect = StateEffect::Financial(FinancialEffect::DepositFunds { depositor, bank, amount });
        DomainResult::success(vec![effect])
    }

    fn execute_withdraw(&self, account_holder: AgentId, bank: AgentId, amount: f64) -> DomainResult {
        let effect = StateEffect::Financial(FinancialEffect::WithdrawFunds { account_holder, bank, amount });
        DomainResult::success(vec![effect])
    }

    fn execute_transfer(&self, from: AgentId, to: AgentId, amount: f64) -> DomainResult {
        let effect = StateEffect::Financial(FinancialEffect::TransferFunds { from, to, amount });
        DomainResult::success(vec![effect])
    }

    fn execute_pay_wages(&self, employer: AgentId, employee: AgentId, amount: f64) -> DomainResult {
        let effect = StateEffect::Financial(FinancialEffect::PayWages { employer, employee, amount });
        DomainResult::success(vec![effect])
    }

    fn execute_inject_liquidity(&self, state: &SimState) -> DomainResult {
        let recipients: Vec<AgentId> = state.agents.consumers.keys().cloned().collect();
        let amount_per_recipient = 1000.0;

        let effect = StateEffect::Financial(FinancialEffect::InjectLiquidity { recipients, amount_per_recipient });
        DomainResult::success(vec![effect])
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Banking",
        constructor: || Box::new(BankingDomain::new()),
    }
}