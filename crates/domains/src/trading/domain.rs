use serde::{Deserialize, Serialize};
use sim_core::*;
use sim_macros::SimDomain;
use crate::banking::BankingDomain;

#[derive(Clone, Debug, Serialize, Deserialize, SimDomain)]
pub struct TradingDomain {
    payment_router: BankingDomain,
}

#[derive(Debug, Clone)]
pub struct TradingResult {
    pub success: bool,
    pub effects: Vec<StateEffect>,
    pub errors: Vec<String>,
}

impl TradingDomain {
    pub fn new() -> Self {
        Self {
            payment_router: BankingDomain::new(),
        }
    }

    pub fn can_handle(&self, action: &TradingAction) -> bool {
        matches!(action, TradingAction::PostBid { .. } | TradingAction::PostAsk { .. })
    }

    pub fn validate(&self, action: &TradingAction, state: &SimState) -> Result<(), String> {
        match action {
            TradingAction::PostBid { agent_id, quantity, price, .. } => {
                self.validate_post_bid(*agent_id, *quantity, *price, state)
            }
            TradingAction::PostAsk { agent_id, market_id, quantity, .. } => {
                self.validate_post_ask(*agent_id, market_id, *quantity, state)
            }
        }
    }

    fn validate_post_bid(&self, agent_id: AgentId, quantity: f64, price: f64, state: &SimState) -> Result<(), String> {
        Validator::positive_amount(quantity)?;
        Validator::positive_amount(price)?;

        if !state.financial_system.balance_sheets.contains_key(&agent_id) {
            return Err(format!("Bidding agent {:?} not found", agent_id));
        }

        let required_cash = quantity * price;
        let available_cash = state.financial_system.get_liquid_assets(&agent_id);
        if available_cash < required_cash {
            return Err(format!(
                "Insufficient funds for bid: agent {:?} needs ${:.2}, has ${:.2}",
                agent_id, required_cash, available_cash
            ));
        }

        Ok(())
    }

    fn validate_post_ask(
        &self, agent_id: AgentId, market_id: &MarketId, quantity: f64, state: &SimState,
    ) -> Result<(), String> {
        Validator::positive_amount(quantity)?;

        if !state.financial_system.balance_sheets.contains_key(&agent_id) {
            return Err(format!("Asking agent {:?} not found", agent_id));
        }
        let bs = state.financial_system.balance_sheets.get(&agent_id).unwrap();

        match market_id {
            MarketId::Goods(good_id) => {
                let available_inventory =
                    bs.get_inventory().and_then(|inv| inv.get(good_id)).map_or(0.0, |item| item.quantity);

                if available_inventory < quantity {
                    return Err(format!(
                        "Insufficient inventory for ask: agent {:?} needs {:.2}, has {:.2}",
                        agent_id, quantity, available_inventory
                    ));
                }
            }
            MarketId::Financial(fin_market_id) => match fin_market_id {
                FinancialMarketId::SecuredOvernightFinancing => {
                    let reserves = state.financial_system.get_bank_reserves(&agent_id).unwrap_or(0.0);
                    if reserves < quantity {
                        return Err(format!(
                            "Insufficient reserves for SOFR ask (lending): agent {:?} needs ${:.2}, has ${:.2}",
                            agent_id, quantity, reserves
                        ));
                    }
                }
                FinancialMarketId::Treasury { tenor } => {
                    let held_quantity = bs
                        .assets
                        .values()
                        .map(|inst| {
                            if let Some(bond_details) = inst.details.as_any().downcast_ref::<BondDetails>() {
                                if bond_details.bond_type == BondType::Government && bond_details.tenor == *tenor {
                                    bond_details.quantity as f64
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            }
                        })
                        .sum::<f64>();
                    if held_quantity < quantity {
                        return Err(format!(
                            "Insufficient Treasury holdings ({:?}) for ask: agent {:?} needs {:.0}, has {:.0}",
                            tenor, agent_id, quantity, held_quantity
                        ));
                    }
                }
                FinancialMarketId::CorporateBond { .. } => {}
            },
            MarketId::Labour(_) => {}
        }

        Ok(())
    }

    pub fn execute(&self, action: &TradingAction, state: &SimState) -> TradingResult {
        if let Err(error) = self.validate(action, state) {
            return TradingResult { success: false, effects: vec![], errors: vec![error] };
        }

        match action {
            TradingAction::PostBid { agent_id, market_id, quantity, price } => {
                self.execute_post_bid(*agent_id, market_id.clone(), *quantity, *price)
            }
            TradingAction::PostAsk { agent_id, market_id, quantity, price } => {
                self.execute_post_ask(*agent_id, market_id.clone(), *quantity, *price)
            }
        }
    }
    pub fn execute_post_bid(&self, agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64) -> TradingResult {
        let effects = vec![StateEffect::Market(MarketEffect::PlaceOrderInBook {
            market_id,
            order: Order::Bid(Bid { agent_id, quantity, price }),
        })];

        TradingResult { success: true, effects, errors: vec![] }
    }

    pub fn execute_post_ask(&self, agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64) -> TradingResult {
        let effects = vec![StateEffect::Market(MarketEffect::PlaceOrderInBook {
            market_id,
            order: Order::Ask(Ask { agent_id, quantity, price }),
        })];

        TradingResult { success: true, effects, errors: vec![] }
    }

    pub fn settle_trade(&self, trade: &Trade, state: &SimState) -> TradingResult {
        match &trade.market_id {
            MarketId::Financial(_) => self.settle_financial_trade(trade, state),
            MarketId::Goods(_) => self.settle_goods_trade(trade, state),
            MarketId::Labour(_) => TradingResult { success: false, effects: vec![], errors: vec!["Labour trade settlement not handled here".to_string()] },
        }
    }

    pub fn settle_goods_trade(&self, trade: &Trade, state: &SimState) -> TradingResult {
        let mut effects = vec![];
        let total_payment = trade.price * trade.quantity;

        let payment_result = self.payment_router.execute_transfer(trade.buyer, trade.seller, total_payment, state);

        if !payment_result.success {
            return TradingResult {
                success: false,
                effects: vec![],
                errors: vec![format!("Goods trade settlement failed during payment: {:?}", payment_result.errors)],
            };
        }
        effects.extend(payment_result.effects);

        let good_id = match trade.market_id {
            MarketId::Goods(id) => id,
            _ => return TradingResult { success: false, effects: vec![], errors: vec!["Invalid MarketId for goods trade".to_string()] },
        };

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

        TradingResult { success: true, effects, errors: vec![] }
    }

    pub fn settle_financial_trade(&self, trade: &Trade, state: &SimState) -> TradingResult {
        let mut effects = vec![];

        match &trade.market_id {
            MarketId::Financial(FinancialMarketId::Treasury { tenor }) => {
                if let Some(seller_bs) = state.financial_system.get_bs_by_id(&trade.seller) {
                    let mut instrument_found = false;
                    for (inst_id, inst) in &seller_bs.assets {
                        if let Some(bond_details) = inst.details.as_any().downcast_ref::<BondDetails>() {
                            if bond_details.bond_type == BondType::Government
                                && bond_details.tenor == *tenor
                                && bond_details.quantity >= trade.quantity as u64
                            {
                                effects.push(StateEffect::Financial(FinancialEffect::SplitAndTransferInstrument {
                                    id: *inst_id,
                                    buyer: trade.buyer,
                                    quantity: trade.quantity as u64,
                                }));

                                let total_payment = trade.price * trade.quantity;
                                let payment_result = self.payment_router.execute_transfer(trade.buyer, trade.seller, total_payment, state);

                                if !payment_result.success {
                                     return TradingResult {
                                        success: false,
                                        effects: vec![],
                                        errors: vec![format!("Financial trade settlement failed during payment: {:?}", payment_result.errors)],
                                    };
                                }
                                effects.extend(payment_result.effects);

                                instrument_found = true;
                                break;
                            }
                        }
                    }
                    if !instrument_found {
                         return TradingResult { success: false, effects: vec![], errors: vec!["Seller does not possess the required financial instrument".to_string()] };
                    }
                }
            }
             MarketId::Financial(FinancialMarketId::SecuredOvernightFinancing) => {
                println!("[INFO] SOFR Trade executed (Settlement logic TBD)");
            }
            _ => {}
        }

        TradingResult { success: !effects.is_empty(), effects, errors: vec![] }
    }
    
}

impl Default for TradingDomain {
    fn default() -> Self {
        Self::new()
    }
}