use crate::{
    Any, Domain, DomainResult, ResolutionContext, ResolutionPhase,
    ResolutionResult, inventory,
};
use chrono::NaiveDate;
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use sim_core::*;

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

    fn resolve_intention(
        &self,
        intention: &SimIntention,
        context: &ResolutionContext,
    ) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::MarketMakeTreasuries {
                agent_id,
                maturity_date,
                quantity,
                bid_yield_bps,
                ask_yield_bps,
            } => self.resolve_treasury_market_making(
                *agent_id,
                *maturity_date,
                *quantity,
                *bid_yield_bps,
                *ask_yield_bps,
                context,
            ),

            SimIntention::PostGoodToMarket { .. }
            | SimIntention::PurchaseInputs { .. }
            | SimIntention::SpendOnGood { .. } => {
                return None;
            }

            _ => return None,
        };

        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::MarketMakeTreasuries { .. } => Some(ResolutionPhase::Market),
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
            TradingAction::PostBid {
                agent_id,
                market_id,
                quantity,
                price,
            } => self.execute_post_bid(*agent_id, market_id.clone(), *quantity, *price),
            TradingAction::PostAsk {
                agent_id,
                market_id,
                quantity,
                price,
            } => self.execute_post_ask(*agent_id, market_id.clone(), *quantity, *price, state),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl TradingDomain {
    fn find_treasuries_by_maturity(
        &self,
        maturity_date: NaiveDate,
        fs: &FinancialSystem,
    ) -> Vec<InstrumentId> {
        let mut matching_ids = Vec::new();
        if let Some(treasury_ids) = fs.exchange.index.by_bond_type.get(&BondType::Government) {
            for id in treasury_ids {
                if let Some(inst) = fs.instruments.get(id) {
                    if let InstrumentType::Bond(details) = &inst.instrument_type {
                        if details.maturity_date == maturity_date {
                            matching_ids.push(*id);
                        }
                    }
                }
            }
        }
        matching_ids
    }

    fn resolve_treasury_market_making(
        &self,
        agent_id: AgentId,
        maturity_date: NaiveDate,
        quantity_units: f64,
        bid_yield_bps: BasisPoints,
        ask_yield_bps: BasisPoints,
        context: &ResolutionContext,
    ) -> Vec<SimAction> {
        let fs = &context.state.financial_system;
        let current_date = context.state.current_date;

        if maturity_date <= current_date {
            return vec![];
        }

        let target_instrument_ids = self.find_treasuries_by_maturity(maturity_date, fs);

        if target_instrument_ids.is_empty() {
            return vec![];
        }

        let ytm = pricing::years_to_maturity(current_date, maturity_date);
        const FREQUENCY: usize = 2;

        let mut actions = Vec::new();

        for instrument_id in target_instrument_ids {
            if let Some(InstrumentType::Bond(details)) = fs
                .instruments
                .get(&instrument_id)
                .map(|i| &i.instrument_type)
            {
                let face_value = details.face_value.to_f64();
                let coupon_rate = bps_to_decimal(details.coupon_rate_bps)
                    .to_f64()
                    .unwrap_or_default();

                let bid_price = pricing::bond_price(
                    Money::from(face_value as i64),
                    Rate::from_f64(coupon_rate).unwrap_or(Rate::ZERO),
                    Rate::from(bps_to_decimal(bid_yield_bps)),
                    ytm,
                    FREQUENCY,
                );

                let ask_price = pricing::bond_price(
                    Money::from(face_value as i64),
                    Rate::from_f64(coupon_rate).unwrap_or(Rate::ZERO),
                    Rate::from(bps_to_decimal(ask_yield_bps)),
                    ytm,
                    FREQUENCY,
                );

                let quantity = quantity_units;

                actions.push(SimAction::Trading(TradingAction::PostBid {
                    agent_id,
                    market_id: MarketId::Financial(instrument_id),
                    quantity,
                    price: bid_price,
                }));
                actions.push(SimAction::Trading(TradingAction::PostAsk {
                    agent_id,
                    market_id: MarketId::Financial(instrument_id),
                    quantity,
                    price: ask_price,
                }));
            }
        }

        actions
    }

}

impl TradingDomain {
    fn validate(&self, action: &TradingAction, state: &SimState) -> Result<(), String> {
        match action {
            TradingAction::PostBid {
                agent_id,
                quantity,
                price,
                ..
            }
            | TradingAction::PostAsk {
                agent_id,
                quantity,
                price,
                ..
            } => {
               Validator::positive_amount(*quantity)?;
               Validator::positive_amount(price.to_f64())?;
               Validator::agent_exists(*agent_id, state)?;
                Ok(())
            }
        }
    }

    fn execute_post_bid(
        &self,
        agent_id: AgentId,
        market_id: MarketId,
        quantity: f64,
        price: Money,
    ) -> DomainResult {
        let order = Order {
            id: uuid::Uuid::new_v4(),
            agent_id,
            side: Side::Bid,
            quantity,
            price: Some(price),
            order_type: OrderType::Limit,
        };

        let effect = StateEffect::Market(MarketEffect::PlaceOrderInBook { market_id, order });

        DomainResult::success(vec![effect])
    }

    fn validate_market_making_position(&self, agent_id: AgentId, instrument_id: InstrumentId, quantity: f64, state: &SimState) -> Result<(), String> {
        if state.financial_system.clearing_house.csd.is_security(&instrument_id) {
            let available = state.financial_system.clearing_house.csd
                .get_position(&agent_id, &instrument_id)
                .unwrap_or(0.0);
            
            if available < quantity {
                return Err(format!(
                    "Insufficient securities for market making: {} available, {} required",
                    available, quantity
                ));
            }
        }
        Ok(())
    }
    
    fn execute_post_ask(&self, agent_id: AgentId, market_id: MarketId, quantity: f64, price: Money, state: &SimState) -> DomainResult {
        if let MarketId::Financial(inst_id) = &market_id {
            if let Err(e) = self.validate_market_making_position(agent_id, *inst_id, quantity, state) {
                return DomainResult::failure(vec![e]);
            }
        }
        
        let order = Order {
            id: uuid::Uuid::new_v4(),
            agent_id,
            side: Side::Ask,
            quantity,
            price: Some(price),
            order_type: OrderType::Limit,
        };

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