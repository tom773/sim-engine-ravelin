use crate::{Any, Domain, DomainResult, ResolutionContext, ResolutionPhase, ResolutionResult, inventory};
use serde::{Deserialize, Serialize};
use sim_core::*;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionsDomain {}

impl TransactionsDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for TransactionsDomain {
    fn name(&self) -> &'static str {
        "Transactions"
    }

    fn resolve_intention(&self, intention: &SimIntention, context: &ResolutionContext) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::PayWages { employer, employee, amount } => {
                vec![SimAction::Transaction(TransactionAction::InitiatePayment {
                    from: *employer,
                    to: *employee,
                    amount: *amount,
                    context: TransactionContext::WagePayment {
                        employer: *employer,
                        employee: *employee,
                        amount: *amount,
                    },
                })]
            }
            SimIntention::CollectTaxes { government_id, target, amount } => {
                vec![SimAction::Transaction(TransactionAction::InitiatePayment {
                    from: *target,
                    to: *government_id,
                    amount: *amount,
                    context: TransactionContext::TaxPayment {
                        payer: *target,
                        amount: *amount,
                    },
                })]
            }

            SimIntention::MarketMakeTreasuries { agent_id, maturity_date, quantity, bid_yield_bps, ask_yield_bps } => {
                self.resolve_treasury_market_making(
                    *agent_id,
                    *maturity_date,
                    *quantity,
                    *bid_yield_bps,
                    *ask_yield_bps,
                    context,
                )
            }
            SimIntention::PostGoodToMarket { agent_id, good_id, quantity, ask_price } => {
                vec![SimAction::Transaction(TransactionAction::PostMarketOrder {
                    agent_id: *agent_id,
                    market_id: MarketId::Goods(*good_id),
                    side: Side::Ask,
                    quantity: *quantity,
                    price: Some(Money::from_f64(*ask_price).unwrap_or_default()),
                    order_type: OrderType::Limit,
                })]
            }

            SimIntention::ApplyForJob { agent_id: _, market_id, application } => {
                vec![SimAction::Transaction(TransactionAction::PostJobApplication {
                    market_id: *market_id,
                    application: application.clone(),
                })]
            }
            SimIntention::HireWorkers { agent_id, count, wage_rate } => {
                let market_id = context
                    .state
                    .financial_system
                    .find_general_labour_market()
                    .unwrap_or(LabourMarketId(Uuid::new_v4()));
                vec![SimAction::Transaction(TransactionAction::PostJobOffer {
                    market_id,
                    offer: JobOffer {
                        offer_id: Uuid::new_v4(),
                        firm_id: *agent_id,
                        quantity: *count,
                        wage_rate: *wage_rate,
                        hours_required: 40.0,
                    },
                })]
            }

            _ => return None,
        };

        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::PayWages { .. } | SimIntention::CollectTaxes { .. } => Some(ResolutionPhase::Independent),
            SimIntention::MarketMakeTreasuries { .. } | SimIntention::PostGoodToMarket { .. } => {
                Some(ResolutionPhase::Market)
            }
            SimIntention::ApplyForJob { .. } | SimIntention::HireWorkers { .. } => Some(ResolutionPhase::Independent),
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let transaction_action = match action {
            SimAction::Transaction(action) => action,
            _ => return DomainResult::failure(vec!["Not a transaction action".to_string()]),
        };

        if let Err(error) = self.validate(transaction_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match transaction_action {
            TransactionAction::InitiatePayment { from, to, amount, context } => {
                self.execute_payment(*from, *to, *amount, context.clone(), state)
            }
            TransactionAction::PostMarketOrder { agent_id, market_id, side, quantity, price, order_type } => {
                self.execute_market_order(*agent_id, market_id.clone(), *side, *quantity, *price, order_type.clone())
            }
            TransactionAction::PostJobApplication { market_id, application } => {
                self.execute_job_application(*market_id, application.clone())
            }
            TransactionAction::PostJobOffer { market_id, offer } => self.execute_job_offer(*market_id, offer.clone()),
        }
    }

    fn settle_trade(&self, trade: &Trade, state: &SimState) -> DomainResult {
        let (_, from_bank) = match state.financial_system.find_agent_liquid_account(&trade.buyer) {
            Some((_, bank_id)) => ((), bank_id),
            None => {
                return DomainResult::failure(vec![format!("Could not find liquid account for buyer {}", trade.buyer)]);
            }
        };

        let (_, to_bank) = match state.financial_system.find_agent_liquid_account(&trade.seller) {
            Some((_, bank_id)) => ((), bank_id),
            None => {
                return DomainResult::failure(vec![format!(
                    "Could not find liquid account for seller {}",
                    trade.seller
                )]);
            }
        };

        match &trade.market_id {
            MarketId::Financial(inst_id) => {
                let mut effects = Vec::new();
                let instruction = SettlementInstruction {
                    instruction_id: Uuid::new_v4(),
                    trade_id: trade.trade_id,
                    seller: trade.seller,
                    buyer: trade.buyer,
                    instrument_id: *inst_id,
                    quantity: trade.quantity,
                    cash_amount: (trade.price * trade.quantity).to_f64(),
                    settlement_date: state.current_date,
                    status: SettlementStatus::Pending,
                };

                effects.push(StateEffect::Financial(FinancialEffect::RecordSettlementInstruction(instruction)));
                effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(PaymentInstruction {
                    id: Uuid::new_v4(),
                    from_bank, 
                    to_bank, 
                    payer: trade.buyer,
                    payee: trade.seller,
                    amount: (trade.price * trade.quantity).to_f64(),
                    context: TransactionContext::TradeSettlement { trade_id: trade.trade_id },
                    priority: PaymentPriority::Normal,
                    earliest_release_tick: state.ticknum,
                    deadline_tick: state.ticknum + 10,
                })));
                DomainResult::success(effects)
            }
            MarketId::Goods(good_id) => {
                let mut effects = vec![];

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

                let (_, buyer_bank) = state
                    .financial_system
                    .find_agent_liquid_account(&trade.buyer)
                    .ok_or_else(|| "Buyer has no liquid account".to_string())
                    .unwrap();
                let (_, seller_bank) = state
                    .financial_system
                    .find_agent_liquid_account(&trade.seller)
                    .ok_or_else(|| "Seller has no liquid account".to_string())
                    .unwrap();

                let payment = PaymentInstruction {
                    id: Uuid::new_v4(),
                    from_bank: buyer_bank,
                    to_bank: seller_bank,
                    payer: trade.buyer,
                    payee: trade.seller,
                    amount: (trade.price * trade.quantity).to_f64(),
                    context: TransactionContext::GoodsPurchase { market_id: trade.market_id.clone() },
                    priority: PaymentPriority::Normal,
                    earliest_release_tick: state.ticknum,
                    deadline_tick: state.ticknum + 10,
                };

                effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(payment)));

                DomainResult::success(effects)
            }
            MarketId::Labour(_) => DomainResult::empty(),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl TransactionsDomain {
    fn validate(&self, action: &TransactionAction, state: &SimState) -> Result<(), String> {
        match action {
            TransactionAction::InitiatePayment { from, to, amount, .. } => {
                Validator::positive_amount(*amount)?;
                Validator::agent_exists(*from, state)?;
                Validator::agent_exists(*to, state)?;
                Ok(())
            }
            TransactionAction::PostMarketOrder { agent_id, quantity, .. } => {
                Validator::positive_amount(*quantity)?;
                Validator::agent_exists(*agent_id, state)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn execute_payment(
        &self, from: AgentId, to: AgentId, amount: f64, context: TransactionContext, state: &SimState,
    ) -> DomainResult {
        if amount <= 1e-9 {
            return DomainResult::empty();
        }

        let (_, from_bank) = match state.financial_system.find_agent_liquid_account(&from) {
            Some(acc) => acc,
            None => return DomainResult::failure(vec![format!("Sender {} has no liquid account", from)]),
        };

        let (_, to_bank) = match state.financial_system.find_agent_liquid_account(&to) {
            Some(acc) => acc,
            None => state
                .financial_system
                .find_any_bank_account()
                .ok_or_else(|| "No banks available for account creation".to_string())
                .unwrap(),
        };

        let (from_account_id, _) = state.financial_system.find_agent_liquid_account(&from).unwrap();
        let from_bs = state.financial_system.balance_sheets.get(&from).unwrap();
        if let Some(pos) = from_bs.assets.get(&from_account_id) {
            if pos.quantity < amount {
                return DomainResult::failure(vec![format!(
                    "Insufficient funds: has {:.2}, needs {:.2}",
                    pos.quantity, amount
                )]);
            }
        }

        let payment_instruction = PaymentInstruction {
            id: Uuid::new_v4(),
            from_bank,
            to_bank,
            payer: from,
            payee: to,
            amount,
            context,
            priority: PaymentPriority::Normal,
            earliest_release_tick: state.ticknum,
            deadline_tick: state.ticknum + 10,
        };

        let effects = vec![
            StateEffect::Financial(FinancialEffect::QueuePayment(payment_instruction.clone())),
            StateEffect::Financial(FinancialEffect::RecordTransaction(Transaction {
                id: Uuid::new_v4(),
                from_agent: from,
                to_agent: to,
                amount,
                transaction_type: format!("{:?}", payment_instruction.context),
                timestamp: state.current_date,
                instrument_id: None,
                ref_id: None,
            })),
        ];

        DomainResult::success(effects)
    }

    fn execute_market_order(
        &self, agent_id: AgentId, market_id: MarketId, side: Side, quantity: f64, price: Option<Money>,
        order_type: OrderType,
    ) -> DomainResult {
        if let MarketId::Financial(_inst_id) = &market_id {
            if side == Side::Ask {}
        }

        let order = Order { id: Uuid::new_v4(), agent_id, side, quantity, price, order_type };

        let effect = StateEffect::Market(MarketEffect::PlaceOrderInBook { market_id, order });
        DomainResult::success(vec![effect])
    }

    fn execute_job_application(&self, market_id: LabourMarketId, application: JobApplication) -> DomainResult {
        let effect = StateEffect::Market(MarketEffect::UpdateLabourMarket {
            market_id,
            update: LabourMarketUpdate::AddApplication(application),
        });
        DomainResult::success(vec![effect])
    }

    fn execute_job_offer(&self, market_id: LabourMarketId, offer: JobOffer) -> DomainResult {
        let effect = StateEffect::Market(MarketEffect::UpdateLabourMarket {
            market_id,
            update: LabourMarketUpdate::AddOffer(offer),
        });
        DomainResult::success(vec![effect])
    }

    fn resolve_treasury_market_making(
        &self, agent_id: AgentId, maturity_date: chrono::NaiveDate, quantity: f64, bid_yield_bps: BasisPoints,
        ask_yield_bps: BasisPoints, context: &ResolutionContext,
    ) -> Vec<SimAction> {
        let fs = &context.state.financial_system;
        let current_date = context.state.current_date;

        if maturity_date <= current_date {
            return vec![];
        }

        let matching_bonds: Vec<InstrumentId> = fs
            .exchange
            .index
            .by_bond_type
            .get(&BondType::Government)
            .map(|ids| {
                ids.iter()
                    .filter(|id| {
                        fs.instruments
                            .get(id)
                            .and_then(|inst| match &inst.instrument_type {
                                InstrumentType::Bond(d) => Some(d.maturity_date == maturity_date),
                                _ => None,
                            })
                            .unwrap_or(false)
                    })
                    .copied()
                    .collect()
            })
            .unwrap_or_default();

        let mut actions = Vec::new();
        for inst_id in matching_bonds {
            if let Some(inst) = fs.instruments.get(&inst_id) {
                if let InstrumentType::Bond(details) = &inst.instrument_type {
                    let ytm = years_to_maturity(current_date, maturity_date);

                    let bid_price = bond_price(
                        details.face_value,
                        bps_to_decimal(details.coupon_rate_bps),
                        bps_to_decimal(bid_yield_bps),
                        ytm,
                        details.frequency as usize,
                    );

                    let ask_price = bond_price(
                        details.face_value,
                        bps_to_decimal(details.coupon_rate_bps),
                        bps_to_decimal(ask_yield_bps),
                        ytm,
                        details.frequency as usize,
                    );

                    actions.push(SimAction::Transaction(TransactionAction::PostMarketOrder {
                        agent_id,
                        market_id: MarketId::Financial(inst_id),
                        side: Side::Bid,
                        quantity,
                        price: Some(bid_price),
                        order_type: OrderType::Limit,
                    }));

                    actions.push(SimAction::Transaction(TransactionAction::PostMarketOrder {
                        agent_id,
                        market_id: MarketId::Financial(inst_id),
                        side: Side::Ask,
                        quantity,
                        price: Some(ask_price),
                        order_type: OrderType::Limit,
                    }));
                }
            }
        }

        actions
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Transactions",
        constructor: || Box::new(TransactionsDomain::new()),
    }
}
