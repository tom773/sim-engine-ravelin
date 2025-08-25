use serde::{Deserialize, Serialize};
use sim_core::*;
use crate::{Any, inventory, Domain, DomainResult, DomainValidator, ResolutionContext, ResolutionResult, ResolutionPhase};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumptionDomain {}

impl ConsumptionDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for ConsumptionDomain {
    fn name(&self) -> &'static str { 
        "Consumption" 
    }

    fn resolve_intention(&self, intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::ApplyForJob { agent_id: _, market_id, application } => {
                vec![SimAction::Labour(LabourAction::ApplyForJob { 
                    market_id: market_id.clone(), 
                    application: application.clone() 
                })]
            },
            
            SimIntention::ConsumeGood { agent_id, good_id, quantity } => {
                vec![SimAction::Consumption(ConsumptionAction::Consume { 
                    agent_id: *agent_id, 
                    good_id: *good_id, 
                    amount: *quantity 
                })]
            },
            
            _ => return None,
        };
        
        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::ApplyForJob { .. } => Some(ResolutionPhase::Independent),
            SimIntention::ConsumeGood { .. } => Some(ResolutionPhase::Independent),
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let consumption_action = match action {
            SimAction::Consumption(action) => action,
            _ => return DomainResult::failure(vec!["Not a consumption action".to_string()]),
        };

        if let Err(error) = self.validate(consumption_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match consumption_action {
            ConsumptionAction::Purchase { agent_id, seller, good_id, amount } => {
                self.execute_purchase(*agent_id, *seller, *good_id, *amount, state)
            },
            ConsumptionAction::PurchaseAtBest { agent_id, good_id, max_notional } => {
                self.execute_purchase_at_best(*agent_id, *good_id, *max_notional, state)
            },
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                self.execute_consume(*agent_id, *good_id, *amount)
            },
            ConsumptionAction::NoAction { agent_id: _ } => {
                DomainResult::empty()
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ConsumptionDomain {
    fn validate(&self, action: &ConsumptionAction, state: &SimState) -> Result<(), String> {
        match action {
            ConsumptionAction::Purchase { agent_id: buyer, seller, good_id, amount } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::agent_exists(*buyer, state)?;
                DomainValidator::agent_exists(*seller, state)?;

                let seller_bs = state.financial_system.balance_sheets.get(seller)
                    .ok_or("Seller not found")?;
                let available_inventory = seller_bs.get_inventory()
                    .and_then(|inv| inv.get(good_id))
                    .map_or(0.0, |item| item.quantity);
                    
                if available_inventory < *amount {
                    return Err(format!(
                        "Seller has insufficient inventory: needs {:.2}, has {:.2}",
                        amount, available_inventory
                    ));
                }

                let price = state.financial_system.exchange.goods_market(good_id)
                    .and_then(|m| m.best_ask())
                    .map_or(1.0, |ask| ask.price);
                let total_cost = amount * price;
                let available_funds = state.financial_system.get_liquid_assets(buyer);
                
                if available_funds < total_cost {
                    return Err(format!(
                        "Buyer has insufficient funds: needs ${:.2}, has ${:.2}", 
                        total_cost, available_funds
                    ));
                }

                Ok(())
            },
            
            ConsumptionAction::PurchaseAtBest { agent_id, good_id: _, max_notional } => {
                DomainValidator::positive_amount(*max_notional)?;
                DomainValidator::agent_exists(*agent_id, state)?;
                
                let available_funds = state.financial_system.get_liquid_assets(agent_id);
                if available_funds < *max_notional {
                    return Err(format!(
                        "Buyer has insufficient funds for max notional: needs ${:.2}, has ${:.2}", 
                        max_notional, available_funds
                    ));
                }
                Ok(())
            },
            
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::agent_exists(*agent_id, state)?;

                let bs = state.financial_system.balance_sheets.get(agent_id)
                    .ok_or(format!("Agent {:?} not found", agent_id))?;
                let available = bs.get_inventory()
                    .and_then(|inv| inv.get(good_id))
                    .map_or(0.0, |item| item.quantity);

                if available < *amount {
                    return Err(format!(
                        "Agent has insufficient goods to consume: needs {:.2}, has {:.2}", 
                        amount, available
                    ));
                }
                Ok(())
            },
            
            ConsumptionAction::NoAction { .. } => Ok(()),
        }
    }

    fn execute_purchase(&self, buyer: AgentId, seller: AgentId, good_id: GoodId, amount: f64, state: &SimState) -> DomainResult {
        let price = state.financial_system.exchange.goods_market(&good_id)
            .and_then(|m| m.best_ask())
            .map_or(1.0, |ask| ask.price);
        let total_cost = amount * price;

        let effects = vec![
            StateEffect::Financial(FinancialEffect::RecordTransaction(Transaction {
                id: uuid::Uuid::new_v4(),
                date: state.ticknum,
                qty: total_cost,
                from: buyer,
                to: seller,
                tx_type: TransactionType::Transfer { from: buyer, to: seller, amount: total_cost },
                instrument_id: None,
            })),
            StateEffect::Inventory(InventoryEffect::RemoveInventory {
                owner: seller,
                good_id,
                quantity: amount,
            }),
            StateEffect::Inventory(InventoryEffect::AddInventory {
                owner: buyer,
                good_id,
                quantity: amount,
                unit_cost: price,
            }),
        ];

        DomainResult::success(effects)
    }

    fn execute_purchase_at_best(&self, buyer: AgentId, good_id: GoodId, max_notional: f64, state: &SimState) -> DomainResult {
        let order = Order::Bid(Bid {
            agent_id: buyer,
            quantity: max_notional,
            price: state.financial_system.exchange.goods_market(&good_id).unwrap().best_ask().unwrap().price,
        });
        let effects = vec![
            StateEffect::Market(MarketEffect::PlaceOrderInBook {
                market_id: MarketId::Goods(good_id),
                order,
            }),
        ];

        DomainResult::success(effects)
    }

    fn execute_consume(&self, agent_id: AgentId, good_id: GoodId, amount: f64) -> DomainResult {
        let effects = vec![
            StateEffect::Inventory(InventoryEffect::RemoveInventory { 
                owner: agent_id, 
                good_id, 
                quantity: amount 
            })
        ];

        DomainResult::success(effects)
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Consumption",
        constructor: || Box::new(ConsumptionDomain::new()),
    }
}
