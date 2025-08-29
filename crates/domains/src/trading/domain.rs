
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

    fn settle_trade(&self, trade: &Trade, state: &SimState) -> DomainResult {
        
        let settlement_effects = self.create_settlement_effects(trade, state);
        
        let mut all_effects = settlement_effects;
        all_effects.push(StateEffect::Market(MarketEffect::ExecuteTrade(trade.clone())));
        
        DomainResult::success(all_effects)
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
    
    fn create_settlement_effects(&self, trade: &Trade, state: &SimState) -> Vec<StateEffect> {
        let mut effects = vec![];
        
        match &trade.market_id {
            MarketId::Goods(good_id) => {
                effects.extend(self.settle_goods_trade(trade, *good_id));
            }
            MarketId::Financial(FinancialMarketId::Treasury { tenor }) => {
                effects.extend(self.settle_treasury_trade(trade, *tenor, state));
            }
            MarketId::Financial(FinancialMarketId::FederalFundsOvernight) => {
                effects.extend(self.settle_fed_funds_trade(trade, state));
            }
            MarketId::Financial(FinancialMarketId::TreasuryRepoOvernight) => {
                effects.extend(self.settle_repo_trade(trade, state));
            }
            MarketId::Financial(FinancialMarketId::DiscountWindow)
            | MarketId::Financial(FinancialMarketId::StandingRepoFacility)
            | MarketId::Financial(FinancialMarketId::OvernightReverseRepo) => {
                println!("[TRADING] Central bank facility trade executed (settlement logic TBD): {:?}", trade.market_id);
            }
            _ => {
                println!("[TRADING] Unknown market type for settlement: {:?}", trade.market_id);
            }
        }
        
        effects
    }
    
    fn settle_goods_trade(&self, trade: &Trade, good_id: GoodId) -> Vec<StateEffect> {
        let mut effects = vec![];
        let total_payment = trade.price * trade.quantity;
        
        effects.extend(self.create_payment_transfer_effects(trade.buyer, trade.seller, total_payment));
        
        effects.push(StateEffect::Inventory(InventoryEffect::RemoveInventory {
            owner: trade.seller,
            good_id,
            quantity: trade.quantity,
        }));
        
        effects.push(StateEffect::Inventory(InventoryEffect::AddInventory {
            owner: trade.buyer,
            good_id,
            quantity: trade.quantity,
            unit_cost: trade.price,
        }));
        
        effects
    }
    
    fn settle_treasury_trade(&self, trade: &Trade, tenor: Tenor, state: &SimState) -> Vec<StateEffect> {
        let mut effects = vec![];
        
        if trade.seller == state.financial_system.government.id {
            effects.extend(self.settle_primary_treasury_issuance(trade, tenor, state));
        } else {
            effects.extend(self.settle_secondary_treasury_trade(trade, tenor, state));
        }
        
        effects
    }
    
    fn settle_primary_treasury_issuance(&self, trade: &Trade, tenor: Tenor, state: &SimState) -> Vec<StateEffect> {
        let mut effects = vec![];
        let total_cost = trade.price * trade.quantity;
        
        effects.extend(self.create_payment_transfer_effects(trade.buyer, trade.seller, total_cost));
        
        const FACE_VALUE: f64 = 1000.0;
        let coupon_rate = state.financial_system.central_bank.policy_rate_bps;
        let maturity_date = tenor.add_to_date(state.current_date);
        let principal = total_cost;

        let mut new_bond = bond!(
            trade.buyer,
            trade.seller,
            principal,
            coupon_rate,
            maturity_date,
            FACE_VALUE,
            BondType::Government,
            2,
            tenor,
            state.current_date
        );

        if let Some(details) = new_bond.details.as_any_mut().downcast_mut::<BondDetails>() {
            details.quantity = trade.quantity as u64;
        }

        effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(new_bond)));
        println!("[PRIMARY ISSUE] Sale Effects: {:#?}", effects); 
        effects
    }
    
    fn settle_secondary_treasury_trade(&self, trade: &Trade, tenor: Tenor, state: &SimState) -> Vec<StateEffect> {
        let mut effects = vec![];
        
        if let Some(seller_bs) = state.financial_system.get_bs_by_id(&trade.seller) {
            for (inst_id, inst) in &seller_bs.assets {
                if let Some(bond_details) = inst.details.as_any().downcast_ref::<BondDetails>() {
                    if bond_details.bond_type == BondType::Government
                        && bond_details.tenor == tenor
                        && bond_details.quantity >= trade.quantity as u64
                    {
                        effects.push(StateEffect::Financial(FinancialEffect::SplitAndTransferInstrument {
                            id: *inst_id,
                            buyer: trade.buyer,
                            quantity: trade.quantity as u64,
                        }));
                        
                        let total_payment = trade.price * trade.quantity;
                        effects.extend(self.create_payment_transfer_effects(trade.buyer, trade.seller, total_payment));
                        
                        break;
                    }
                }
            }
        }
        println!("[SECONDARY TRADE] Settlement Effects: {:#?}", effects); 
        effects
    }
    
    fn settle_fed_funds_trade(&self, trade: &Trade, state: &SimState) -> Vec<StateEffect> {
        let mut effects = vec![];
        let loan_amount = trade.quantity;
        let overnight_rate_bps = state.financial_system.central_bank.policy_rate_bps;
        
        effects.extend(self.create_reserves_transfer_effects(trade.seller, trade.buyer, loan_amount, state));
        
        let fed_funds_loan = FinancialInstrument {
            id: InstrumentId(uuid::Uuid::new_v4()),
            creditor: trade.seller,
            debtor: trade.buyer,
            principal: loan_amount,
            details: Box::new(LoanDetails {
                loan_type: LoanType::FederalFunds,
                interest_rate_bps: overnight_rate_bps,
                maturity_date: state.current_date + chrono::Duration::days(1),
                collateral: None,
            }),
            originated_date: state.current_date,
            accrued_interest: 0.0,
            last_accrual_date: state.current_date,
        };
        
        effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(fed_funds_loan)));
        
        println!("[TRADING] Federal funds trade executed: ${:.2} from {} to {}", 
                loan_amount, trade.seller, trade.buyer);
        
        effects
    }
    
    fn settle_repo_trade(&self, trade: &Trade, state: &SimState) -> Vec<StateEffect> {
        let mut effects = vec![];
        let repo_amount = trade.quantity;
        let repo_rate_bps = state.financial_system.central_bank.policy_rate_bps - 10.0;
        
        effects.extend(self.create_payment_transfer_effects(trade.seller, trade.buyer, repo_amount));
        
        let repo_agreement = FinancialInstrument {
            id: InstrumentId(uuid::Uuid::new_v4()),
            creditor: trade.seller,
            debtor: trade.buyer,
            principal: repo_amount,
            details: Box::new(LoanDetails {
                loan_type: LoanType::Repo,
                interest_rate_bps: repo_rate_bps,
                maturity_date: state.current_date + chrono::Duration::days(1),
                collateral: Some(CollateralInfo {
                    collateral_type: "US Treasury".to_string(),
                    value: repo_amount * 1.02,
                }),
            }),
            originated_date: state.current_date,
            accrued_interest: 0.0,
            last_accrual_date: state.current_date,
        };
        
        effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(repo_agreement)));
        
        println!("[TRADING] Treasury repo trade executed: ${:.2} from {} to {}", 
                repo_amount, trade.seller, trade.buyer);
        
        effects
    }
    
    fn create_reserves_transfer_effects(&self, from: AgentId, to: AgentId, amount: f64, state: &SimState) -> Vec<StateEffect> {
        let mut effects = vec![];
        let cb_id = state.financial_system.central_bank.id;
        
        if let Some(from_bs) = state.financial_system.get_bs_by_id(&from) {
            if let Some((reserves_id, reserves_inst)) =
                from_bs.assets.iter().find(|(_, inst)| inst.details.as_any().is::<CentralBankReservesDetails>())
            {
                let new_reserves = reserves_inst.principal - amount;
                if new_reserves < 1e-6 {
                    effects.push(StateEffect::Financial(FinancialEffect::RemoveInstrument(*reserves_id)));
                } else {
                    effects.push(StateEffect::Financial(FinancialEffect::UpdateInstrument {
                        id: *reserves_id,
                        new_principal: new_reserves,
                    }));
                }
            }
        }
        
        effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument(reserves!(
            to,
            cb_id,
            amount,
            state.current_date,
            state.financial_system.central_bank.policy_rate_bps + 15.0
        ))));
        
        effects
    }
    
    fn create_payment_transfer_effects(&self, from: AgentId, to: AgentId, amount: f64) -> Vec<StateEffect> {
        vec![StateEffect::Financial(FinancialEffect::TransferFunds { from, to, amount })]
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