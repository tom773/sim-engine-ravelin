use crate::{Any, Domain, DomainResult, ResolutionContext, ResolutionPhase, ResolutionResult, inventory};
use serde::{Deserialize, Serialize};
use sim_core::*;
use uuid::Uuid;
use rust_decimal_macros::dec;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonetaryDomain {}

impl MonetaryDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for MonetaryDomain {
    fn name(&self) -> &'static str {
        "Monetary"
    }

    fn resolve_intention(&self, intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::ConductOMO { cb_id, operation_type, amount } => {
                vec![SimAction::Monetary(MonetaryAction::OpenMarketOperation {
                    cb_id: *cb_id,
                    operation_type: *operation_type,
                    amount: *amount,
                })]
            }
            SimIntention::SetPolicyRate { cb_id, new_rate_bps } => {
                vec![SimAction::Monetary(MonetaryAction::SetPolicyRate {
                    cb_id: *cb_id,
                    rate_bps: *new_rate_bps,
                })]
            }
            SimIntention::AdjustReserveRequirement { cb_id, new_ratio } => {
                vec![SimAction::Monetary(MonetaryAction::SetReserveRequirement {
                    cb_id: *cb_id,
                    ratio: *new_ratio,
                })]
            }
            SimIntention::ProvideLiquidityFacility { cb_id, bank_id, amount, collateral } => {
                vec![SimAction::Monetary(MonetaryAction::ProvideLiquidityAssistance {
                    cb_id: *cb_id,
                    bank_id: *bank_id,
                    amount: *amount,
                    collateral: collateral.clone(),
                })]
            }
            _ => return None,
        };

        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::ConductOMO { .. } => Some(ResolutionPhase::Market),
            SimIntention::SetPolicyRate { .. } | 
            SimIntention::AdjustReserveRequirement { .. } => Some(ResolutionPhase::Independent),
            SimIntention::ProvideLiquidityFacility { .. } => Some(ResolutionPhase::Independent),
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let monetary_action = match action {
            SimAction::Monetary(action) => action,
            _ => return DomainResult::failure(vec!["Not a monetary action".to_string()]),
        };

        if let Err(error) = self.validate(monetary_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match monetary_action {
            MonetaryAction::OpenMarketOperation { cb_id, operation_type, amount } => {
                self.execute_omo(*cb_id, *operation_type, *amount, state)
            }
            MonetaryAction::SetPolicyRate { cb_id, rate_bps } => {
                self.execute_rate_change(*cb_id, *rate_bps, state)
            }
            MonetaryAction::SetReserveRequirement { cb_id, ratio } => {
                self.execute_reserve_requirement_change(*cb_id, *ratio, state)
            }
            MonetaryAction::ProvideLiquidityAssistance { cb_id, bank_id, amount, collateral } => {
                self.execute_liquidity_assistance(*cb_id, *bank_id, *amount, collateral, state)
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl MonetaryDomain {
    fn validate(&self, action: &MonetaryAction, state: &SimState) -> Result<(), String> {
        match action {
            MonetaryAction::OpenMarketOperation { cb_id, amount, .. } => {
                if *cb_id != state.financial_system.central_bank.id {
                    return Err("Invalid central bank ID".to_string());
                }
                Validator::positive_amount(*amount)
            }
            MonetaryAction::SetPolicyRate { cb_id, rate_bps } => {
                if *cb_id != state.financial_system.central_bank.id {
                    return Err("Invalid central bank ID".to_string());
                }
                if *rate_bps < dec!(0) || *rate_bps > dec!(10000) {
                    return Err("Policy rate must be between 0 and 10000 bps".to_string());
                }
                Ok(())
            }
            MonetaryAction::SetReserveRequirement { cb_id, ratio } => {
                if *cb_id != state.financial_system.central_bank.id {
                    return Err("Invalid central bank ID".to_string());
                }
                if *ratio < 0.0 || *ratio > 1.0 {
                    return Err("Reserve requirement must be between 0 and 1".to_string());
                }
                Ok(())
            }
            MonetaryAction::ProvideLiquidityAssistance { cb_id, bank_id, amount, .. } => {
                if *cb_id != state.financial_system.central_bank.id {
                    return Err("Invalid central bank ID".to_string());
                }
                Validator::positive_amount(*amount)?;
                Validator::bank_exists(*bank_id, state)
            }
        }
    }

    fn execute_omo(&self, cb_id: AgentId, operation_type: OMOType, amount: f64, state: &SimState) -> DomainResult {
        let mut effects = Vec::new();
        let fs = &state.financial_system;
        
        match operation_type {
            OMOType::QuantitativeEasing => {
                // Central bank buying treasuries - injecting liquidity
                // Find available government bonds in the market
                if let Some(treasury_ids) = fs.exchange.index.by_bond_type.get(&BondType::Government) {
                    let mut remaining_amount = amount;
                    
                    for inst_id in treasury_ids {
                        if remaining_amount <= 0.0 { break; }
                        
                        // Check market for available asks
                        if let Some(market) = fs.exchange.financial_market(inst_id) {
                            if let Some(best_ask) = market.best_ask() {
                                let quantity = (remaining_amount / best_ask.to_f64()).min(10.0); // Max 10 bonds per order
                                
                                let order = Order {
                                    id: Uuid::new_v4(),
                                    agent_id: cb_id,
                                    side: Side::Bid,
                                    quantity,
                                    price: Some(best_ask * 1.01), // Slightly above market to ensure fill
                                    order_type: OrderType::Limit,
                                };
                                
                                effects.push(StateEffect::Market(MarketEffect::PlaceOrderInBook {
                                    market_id: MarketId::Financial(*inst_id),
                                    order,
                                }));
                                
                                remaining_amount -= quantity * best_ask.to_f64();
                            }
                        }
                    }
                }
            }
            OMOType::QuantitativeTightening => {
                // Central bank selling treasuries - draining liquidity
                // Check CB's security holdings in CSD
                let cb_holdings = fs.clearing_house.csd.get_all_positions(&cb_id);
                
                for (inst_id, quantity_held) in cb_holdings {
                    if let Some(inst) = fs.instruments.get(&inst_id) {
                        if let InstrumentType::Bond(details) = &inst.instrument_type {
                            if details.bond_type == BondType::Government && quantity_held > 0.0 {
                                // Sell a portion of holdings
                                let sell_quantity = quantity_held.min(amount / details.face_value.to_f64());
                                
                                if let Some(market) = fs.exchange.financial_market(&inst_id) {
                                    if let Some(best_bid) = market.best_bid() {
                                        let order = Order {
                                            id: Uuid::new_v4(),
                                            agent_id: cb_id,
                                            side: Side::Ask,
                                            quantity: sell_quantity,
                                            price: Some(best_bid * 0.99), // Slightly below market to ensure fill
                                            order_type: OrderType::Limit,
                                        };
                                        
                                        effects.push(StateEffect::Market(MarketEffect::PlaceOrderInBook {
                                            market_id: MarketId::Financial(inst_id),
                                            order,
                                        }));
                                        
                                        break; // One sale for now
                                    }
                                }
                            }
                        }
                    }
                }
            }
            OMOType::Repo { term_days, rate_bps } => {
                tracing::warn!("Repo operations not yet implemented (Args: {} days at {} bps)", term_days, rate_bps);
            },
            OMOType::ReverseRepo { term_days, rate_bps } => {
                tracing::warn!("Reverse Repo operations not yet implemented (Args: {} days at {} bps)", term_days, rate_bps);
            }
        }
        
        DomainResult::success(effects)
    }

    fn execute_rate_change(&self, _cb_id: AgentId, _rate_bps: BasisPoints, _state: &SimState) -> DomainResult {
        // This would update the CB's policy rate
        // But we can't directly modify state here, so we'd need a new effect type
        // For now, just record the intention
        DomainResult::empty()
    }

    fn execute_reserve_requirement_change(&self, _cb_id: AgentId, _ratio: f64, _state: &SimState) -> DomainResult {
        // Similar to rate change - would need new effect type
        DomainResult::empty()
    }

    fn execute_liquidity_assistance(
        &self, cb_id: AgentId, bank_id: AgentId, amount: f64, 
        collateral: &Option<Vec<InstrumentId>>, state: &SimState
    ) -> DomainResult {
        let mut effects = Vec::new();
        
        // Validate collateral if provided
        if let Some(collateral_ids) = collateral {
            let mut total_collateral_value = 0.0;
            
            for inst_id in collateral_ids {
                // Check bank owns the collateral in CSD
                let quantity = state.financial_system.clearing_house.csd
                    .get_position(&bank_id, inst_id)
                    .unwrap_or(0.0);
                    
                if quantity <= 0.0 {
                    return DomainResult::failure(vec![
                        format!("Bank {} doesn't own collateral {}", bank_id, inst_id)
                    ]);
                }
                
                // Get collateral value (simplified - would use haircuts in reality)
                if let Some(inst) = state.financial_system.instruments.get(inst_id) {
                    let value = inst.face_value().unwrap_or(Money::from(1000)) * quantity;
                    total_collateral_value += value.to_f64();
                }
            }
            
            if total_collateral_value < amount {
                return DomainResult::failure(vec![
                    format!("Insufficient collateral: {} < {}", total_collateral_value, amount)
                ]);
            }
        }
        
        // Create liquidity loan (overnight facility)
        let loan_instrument = match Instrument::bond(
            InstrumentId(Uuid::new_v4()),
            bank_id,
            BondType::InterbankLoan, // CB liquidity facility
            Money::from((amount as u64).max(1)),
            state.current_date,
            state.current_date + chrono::Duration::days(1),
        )
        .coupon_bps(state.financial_system.central_bank.policy_rate_bps + dec!(100)) // Penalty rate
        .frequency(0)
        .rating(CreditRating::AAA)
        .build()
        {
            Ok(inst) => inst,
            Err(e) => return DomainResult::failure(vec![format!("Failed to create loan: {}", e)]),
        };
        
        let loan_id = loan_instrument.id;
        
        // Create the loan (CB as creditor, bank as debtor)
        effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument {
            instrument: loan_instrument,
            creditor: cb_id,
            debtor: bank_id,
            quantity: 1.0,
        }));
        
        // Queue payment to the bank (inject reserves)
        let payment = PaymentInstruction {
            id: Uuid::new_v4(),
            from_bank: cb_id,
            to_bank: bank_id,
            payer: cb_id,
            payee: bank_id,
            amount,
            context: TransactionContext::GenericTransfer,
            priority: PaymentPriority::Urgent,
            earliest_release_tick: state.ticknum,
            deadline_tick: state.ticknum + 1,
        };
        
        effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(payment)));
        
        // Record the transaction
        effects.push(StateEffect::Financial(FinancialEffect::RecordTransaction(Transaction {
            id: Uuid::new_v4(),
            from_agent: cb_id,
            to_agent: bank_id,
            amount,
            transaction_type: "LiquidityAssistance".to_string(),
            timestamp: state.current_date,
            instrument_id: Some(loan_id),
            ref_id: None,
        })));
        
        DomainResult::success(effects)
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Monetary",
        constructor: || Box::new(MonetaryDomain::new()),
    }
}

