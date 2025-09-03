use crate::banking::BankingDomain;
use crate::{
    Any, Domain, DomainResult, DomainValidator, ResolutionContext, ResolutionPhase,
    ResolutionResult, inventory,
};
use chrono::NaiveDate;
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::collections::HashMap;

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
            } => self.execute_post_ask(*agent_id, market_id.clone(), *quantity, *price),
        }
    }

    fn settle_trade(&self, trade: &Trade, state: &SimState) -> DomainResult {
        if let Err(e) = self.validate_trade(trade, state) {
            println!("[SETTLEMENT FAILED] Trade validation failed: {}", e);
            return DomainResult::empty();
        }

        let mut effects = Vec::new();
        let total_payment = (trade.price * trade.quantity).to_f64();

        match &trade.market_id {
            MarketId::Goods(good_id) => {
                effects.push(StateEffect::Inventory(InventoryEffect::RemoveInventory {
                    owner: trade.seller,
                    good_id: *good_id,
                    quantity: trade.quantity,
                }));
                effects.push(StateEffect::Inventory(InventoryEffect::AddInventory {
                    owner: trade.buyer,
                    good_id: *good_id,
                    quantity: trade.quantity,
                    unit_cost: trade.price.to_f64(),
                }));
            }
            MarketId::Financial(instrument_id) => {
                effects.push(StateEffect::Financial(FinancialEffect::AdjustPosition {
                    owner: trade.seller,
                    instrument_id: *instrument_id,
                    delta_quantity: -trade.quantity,
                    side: PositionSide::Asset,
                    cost_per_unit: None,
                }));
                effects.push(StateEffect::Financial(FinancialEffect::AdjustPosition {
                    owner: trade.buyer,
                    instrument_id: *instrument_id,
                    delta_quantity: trade.quantity,
                    side: PositionSide::Asset,
                    cost_per_unit: Some(trade.price),
                }));
            }
            MarketId::Labour(_) => { /* No asset leg for labour trades */ }
        }

        if total_payment > 0.0 {
            let banking_domain = BankingDomain::new();
            let payment_result = banking_domain.execute_initiate_payment(
                trade.buyer,
                trade.seller,
                total_payment,
                TransactionContext::TradeSettlement {
                    trade_id: trade.trade_id,
                },
                state,
            );

            if payment_result.success {
                effects.extend(payment_result.effects);
            } else {
                return DomainResult::failure(payment_result.errors);
            }
        }

        DomainResult::success(effects)
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

    fn validate_trade(&self, trade: &Trade, state: &SimState) -> Result<(), String> {
        let fs = &state.financial_system;

        let buyer_cash = fs.get_liquid_assets(&trade.buyer);
        let required_cash = (trade.price * trade.quantity).to_f64();
        if buyer_cash < required_cash {
            return Err(format!(
                "Buyer {} has insufficient funds for trade. Needs {}, has {}",
                trade.buyer, required_cash, buyer_cash
            ));
        }

        match &trade.market_id {
            MarketId::Goods(good_id) => {
                let inventory = get_agent_inventory(fs, &trade.seller);
                let available_qty = inventory.get(good_id).map_or(0.0, |item| item.quantity);
                if available_qty < trade.quantity {
                    return Err(format!(
                        "Seller {} has insufficient inventory for trade. Needs {}, has {}",
                        trade.seller, trade.quantity, available_qty
                    ));
                }
            }
            MarketId::Financial(instrument_id) => {
                let seller_bs = fs
                    .balance_sheets
                    .get(&trade.seller)
                    .ok_or("Seller BS not found")?;
                let position = seller_bs
                    .assets
                    .get(instrument_id)
                    .ok_or("Seller does not own the instrument")?;
                if position.quantity < trade.quantity {
                    return Err(format!(
                        "Seller {} has insufficient position for trade. Needs {}, has {}",
                        trade.seller, trade.quantity, position.quantity
                    ));
                }
            }
            MarketId::Labour(_) => {}
        }
        Ok(())
    }
}

fn get_agent_inventory(fs: &FinancialSystem, agent_id: &AgentId) -> HashMap<GoodId, InventoryItem> {
    if let Some(bs) = fs.balance_sheets.get(agent_id) {
        for inst_id in bs.assets.keys() {
            if let Some(inst) = fs.instruments.get(inst_id) {
                if let InstrumentType::RealAsset(RealAssetType::Inventory { goods, .. }) =
                    &inst.instrument_type
                {
                    return goods.clone();
                }
            }
        }
    }
    HashMap::new()
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
                DomainValidator::positive_amount(*quantity)?;
                DomainValidator::positive_amount(price.to_f64())?;
                DomainValidator::agent_exists(*agent_id, state)?;
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

    fn execute_post_ask(
        &self,
        agent_id: AgentId,
        market_id: MarketId,
        quantity: f64,
        price: Money,
    ) -> DomainResult {
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
