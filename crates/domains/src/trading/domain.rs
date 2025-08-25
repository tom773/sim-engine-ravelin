use serde::{Deserialize, Serialize};
use sim_core::*;
use crate::{Any, inventory, Domain, DomainResult, DomainValidator, ResolutionContext, ResolutionResult, ResolutionPhase};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradingDomain {}

impl TradingDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for TradingDomain {
    fn name(&self) -> &'static str { 
        "Trading" 
    }

    fn resolve_intention(&self, intention: &SimIntention, context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::MarketMakeTreasuries { agent_id, tenor, quantity, bid_yield_bps, ask_yield_bps } => {
                self.resolve_treasury_market_making(*agent_id, *tenor, *quantity, *bid_yield_bps, *ask_yield_bps, context)
            },

            SimIntention::SellInventory { agent_id, good_id, quantity, desired_markup } => {
                self.resolve_inventory_sale(*agent_id, *good_id, *quantity, *desired_markup, context)
            },

            SimIntention::PurchaseInputs { agent_id, good_id, quantity, max_price } => {
                vec![SimAction::Trading(TradingAction::PostBid {
                    agent_id: *agent_id,
                    market_id: MarketId::Goods(*good_id),
                    quantity: *quantity,
                    price: *max_price,
                })]
            },

            SimIntention::SpendOnGood { agent_id, good_id, max_notional } => {
                self.resolve_consumer_spending(*agent_id, *good_id, *max_notional, context)
            },

            _ => return None,
        };

        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::MarketMakeTreasuries { .. } |
            SimIntention::SellInventory { .. } |
            SimIntention::PurchaseInputs { .. } |
            SimIntention::SpendOnGood { .. } => Some(ResolutionPhase::Market),
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let trading_action = match action {
            SimAction::Trading(action) => action,
            _ => return DomainResult::failure(vec!["Not a trading action".to_string()]),
        };

        if let Err(error) = self.validate(trading_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match trading_action {
            TradingAction::PostBid { agent_id, market_id, quantity, price } => {
                self.execute_post_bid(*agent_id, market_id.clone(), *quantity, *price)
            },
            TradingAction::PostAsk { agent_id, market_id, quantity, price } => {
                self.execute_post_ask(*agent_id, market_id.clone(), *quantity, *price)
            },
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl TradingDomain {
    fn resolve_treasury_market_making(
        &self, 
        agent_id: AgentId, 
        tenor: Tenor, 
        quantity: f64, 
        bid_yield_bps: BasisPoints, 
        ask_yield_bps: BasisPoints,
        context: &ResolutionContext
    ) -> Vec<SimAction> {
        const FACE_VALUE: f64 = 1000.0;
        const FREQUENCY: usize = 2;
        
        let benchmark_coupon_bps = context.state.financial_system.central_bank.policy_rate_bps;
        
        let bid_price = pricing::bond_price(
            FACE_VALUE,
            bps_to_decimal(benchmark_coupon_bps),
            bps_to_decimal(bid_yield_bps),
            tenor.to_years(),
            FREQUENCY
        );

        let ask_price = pricing::bond_price(
            FACE_VALUE,
            bps_to_decimal(benchmark_coupon_bps),
            bps_to_decimal(ask_yield_bps),
            tenor.to_years(),
            FREQUENCY
        );

        let market_id = MarketId::Financial(FinancialMarketId::Treasury { tenor });

        vec![
            SimAction::Trading(TradingAction::PostBid {
                agent_id,
                market_id: market_id.clone(),
                quantity,
                price: bid_price,
            }),
            SimAction::Trading(TradingAction::PostAsk {
                agent_id,
                market_id,
                quantity,
                price: ask_price,
            }),
        ]
    }

    fn resolve_inventory_sale(
        &self, 
        agent_id: AgentId, 
        good_id: GoodId, 
        quantity: f64, 
        desired_markup: f64,
        context: &ResolutionContext
    ) -> Vec<SimAction> {
        let unit_cost = context.state.financial_system
            .get_bs_by_id(&agent_id)
            .and_then(|bs| bs.get_inventory())
            .and_then(|inv| inv.get(&good_id))
            .map(|item| item.unit_cost)
            .unwrap_or(1.0);

        let price = unit_cost * desired_markup;

        vec![SimAction::Trading(TradingAction::PostAsk {
            agent_id,
            market_id: MarketId::Goods(good_id),
            quantity,
            price,
        })]
    }

    fn resolve_consumer_spending(
        &self,
        agent_id: AgentId,
        good_id: GoodId,
        max_notional: f64,
        context: &ResolutionContext
    ) -> Vec<SimAction> {
        let state = context.state;
        
        let market = match state.financial_system.exchange.goods_market(&good_id) {
            Some(m) => m,
            None => return vec![],
        };

        let mut remaining_notional = max_notional;
        let mut actions = Vec::new();

        let mut asks = market.order_book.asks.clone();
        asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));

        for ask in asks {
            if remaining_notional <= 1e-6 {
                break;
            }

            let cost_at_ask_price = ask.quantity * ask.price;
            let bid_quantity = if cost_at_ask_price <= remaining_notional {
                remaining_notional -= cost_at_ask_price;
                ask.quantity
            } else {
                let qty = remaining_notional / ask.price;
                remaining_notional = 0.0;
                qty
            };

            if bid_quantity > 1e-6 {
                actions.push(SimAction::Trading(TradingAction::PostBid {
                    agent_id,
                    market_id: MarketId::Goods(good_id),
                    quantity: bid_quantity,
                    price: ask.price,
                }));
            }
        }

        actions
    }
}

impl TradingDomain {
    fn validate(&self, action: &TradingAction, state: &SimState) -> Result<(), String> {
        match action {
            TradingAction::PostBid { agent_id, quantity, price, .. } |
            TradingAction::PostAsk { agent_id, quantity, price, .. } => {
                DomainValidator::positive_amount(*quantity)?;
                DomainValidator::positive_amount(*price)?;
                DomainValidator::agent_exists(*agent_id, state)?;
                Ok(())
            }
        }
    }

    fn execute_post_bid(&self, agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64) -> DomainResult {
        let bid = Bid { agent_id, quantity, price };
        let order = Order::Bid(bid);
        let effect = StateEffect::Market(MarketEffect::PlaceOrderInBook { market_id, order });
        
        DomainResult::success(vec![effect])
    }

    fn execute_post_ask(&self, agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64) -> DomainResult {
        let ask = Ask { agent_id, quantity, price };
        let order = Order::Ask(ask);
        let effect = StateEffect::Market(MarketEffect::PlaceOrderInBook { market_id, order });
        
        DomainResult::success(vec![effect])
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Trading",
        constructor: || Box::new(TradingDomain::new()),
    }
}