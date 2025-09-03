use serde::{Deserialize, Serialize};
use sim_core::*;
use crate::{Any, inventory, Domain, DomainResult, DomainValidator, ResolutionContext, ResolutionResult, ResolutionPhase};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumptionDomain {}

impl ConsumptionDomain {
    pub fn new() -> Self {
        Self {}
    }

    // Helper function to retrieve agent inventory HashMap based on the new structure.
    fn get_agent_inventory(fs: &FinancialSystem, agent_id: &AgentId) -> HashMap<GoodId, InventoryItem> {
        if let Some(bs) = fs.balance_sheets.get(agent_id) {
            // Iterate over assets to find the RealAsset::Inventory instrument.
            for inst_id in bs.assets.keys() {
                if let Some(inst) = fs.instruments.get(inst_id) {
                    if let InstrumentType::RealAsset(RealAssetType::Inventory { goods, .. }) = &inst.instrument_type {
                        return goods.clone();
                    }
                }
            }
        }
        HashMap::new()
    }
}

impl Domain for ConsumptionDomain {
    fn name(&self) -> &'static str { 
        "Consumption" 
    }

    fn resolve_intention(&self, intention: &SimIntention, _context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            // Resolves to LabourAction (cross-domain).
            SimIntention::ApplyForJob { agent_id: _, market_id, application } => {
                vec![SimAction::Labour(LabourAction::ApplyForJob { 
                    market_id: *market_id, 
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

            // Resolves SpendOnGood into a market action (PurchaseAtBest).
            SimIntention::SpendOnGood { agent_id, good_id, max_notional } => {
                 vec![SimAction::Consumption(ConsumptionAction::PurchaseAtBest {
                    agent_id: *agent_id,
                    good_id: *good_id,
                    max_notional: *max_notional,
                })]
            }
            
            _ => return None,
        };
        
        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::ApplyForJob { .. } => Some(ResolutionPhase::Independent),
            SimIntention::ConsumeGood { .. } => Some(ResolutionPhase::Independent),
            SimIntention::SpendOnGood { .. } => Some(ResolutionPhase::Market), // Interacts with the market
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
                // Direct, bilateral purchase.
                self.execute_purchase(*agent_id, *seller, *good_id, *amount, state)
            },
            ConsumptionAction::PurchaseAtBest { agent_id, good_id, max_notional } => {
                // Interacting with the market.
                self.execute_purchase_at_best(*agent_id, *good_id, *max_notional, state)
            },
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                self.execute_consume(*agent_id, *good_id, *amount)
            },
            ConsumptionAction::NoAction { .. } => {
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
            ConsumptionAction::Purchase { .. } => {
                // Validation for direct purchase is complex and often discouraged in favor of market interactions.
                Err("Direct Purchase action (ConsumptionAction::Purchase) is not fully supported. Use PurchaseAtBest.".to_string())
            },
            
            ConsumptionAction::PurchaseAtBest { agent_id, good_id: _, max_notional } => {
                DomainValidator::non_negative_amount(*max_notional)?;
                DomainValidator::agent_exists(*agent_id, state)?;
                
                // We allow the action even if funds are insufficient for the full max_notional, 
                // assuming the agent intends to spend UP TO that amount. Settlement will handle actual fund availability.
                Ok(())
            },
            
            ConsumptionAction::Consume { agent_id, good_id, amount } => {
                DomainValidator::positive_amount(*amount)?;
                DomainValidator::agent_exists(*agent_id, state)?;

                // Check inventory holdings.
                let fs = &state.financial_system;
                let inventory = Self::get_agent_inventory(fs, agent_id);
                
                let available = inventory.get(good_id).map_or(0.0, |item| item.quantity);

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

    fn execute_purchase(&self, _buyer: AgentId, _seller: AgentId, _good_id: GoodId, _amount: f64, _state: &SimState) -> DomainResult {
        // Implementation for direct purchase (DVP for goods).
        DomainResult::failure(vec!["Direct Purchase execution not implemented.".to_string()])
    }

    // Execute PurchaseAtBest by placing aggressive orders on the market.
    fn execute_purchase_at_best(&self, buyer: AgentId, good_id: GoodId, max_notional: f64, state: &SimState) -> DomainResult {
        
        if max_notional <= 1e-9 {
            return DomainResult::empty();
        }

        let market = match state.financial_system.exchange.goods_market(&good_id) {
            Some(m) => m,
            None => return DomainResult::failure(vec![format!("Goods market for {:?} not found.", good_id)]),
        };

        // We want to buy as much as possible up to max_notional. This is best represented by a Market Order.
        // However, Market Orders don't have a price limit, only quantity. 
        
        // To simulate "buy up to max_notional", we need to determine the quantity affordable by walking the book.

        let mut remaining_notional = max_notional;
        let mut total_quantity_to_buy = 0.0;

        // Iterate over asks (sorted by price ascending).
        for (price_nn, orders) in &market.book.asks {
            let price = price_nn.into_inner();
            if remaining_notional < 1e-9 { break; }

            let quantity_at_level = orders.iter().map(|o| o.quantity).sum::<f64>();
            let cost_at_level = quantity_at_level * price;

            if cost_at_level <= remaining_notional {
                total_quantity_to_buy += quantity_at_level;
                remaining_notional -= cost_at_level;
            } else {
                let affordable_qty = remaining_notional / price;
                total_quantity_to_buy += affordable_qty;
                let _remaining_notional = 0.0;
                break;
            }
        }

        if total_quantity_to_buy > 1e-6 {
            // Place a Market Order for the calculated quantity.
            let order = Order {
                id: uuid::Uuid::new_v4(),
                agent_id: buyer,
                side: Side::Bid,
                quantity: total_quantity_to_buy,
                price: None, // Market order
                order_type: OrderType::Market,
            };

            // Place the order in the goods market.
            let effect = StateEffect::Market(MarketEffect::PlaceOrderInBook {
                market_id: MarketId::Goods(good_id),
                order,
            });

            DomainResult::success(vec![effect])
        } else {
            // Nothing affordable or available.
            DomainResult::empty()
        }
    }

    fn execute_consume(&self, agent_id: AgentId, good_id: GoodId, amount: f64) -> DomainResult {
        // Consumption destroys the goods (removes from inventory).
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