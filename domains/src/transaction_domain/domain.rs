use crate::{Any, Domain, DomainResult, ResolutionContext, ResolutionPhase, ResolutionResult, inventory};
use chrono::NaiveDate;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
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
            SimIntention::Transaction(TransactionIntention::PayWages { employer, employee, amount }) => {
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

            SimIntention::Fiscal(FiscalIntention::CollectTaxes { government_id, target, amount }) => {
                vec![SimAction::Transaction(TransactionAction::InitiatePayment {
                    from: *target,
                    to: *government_id,
                    amount: *amount,
                    context: TransactionContext::TaxPayment { payer: *target, amount: *amount },
                })]
            }

            SimIntention::Banking(BankingIntention::MarketMakeTreasuries {
                agent_id,
                maturity_date,
                bid_yield_bps,
                ask_yield_bps,
                quantity,
            }) => self.resolve_treasury_market_making(
                *agent_id,
                *maturity_date,
                *bid_yield_bps,
                *ask_yield_bps,
                *quantity,
                context.state,
            ),

            SimIntention::Production(ProductionIntention::PostGoodToMarket {
                agent_id,
                good_id,
                quantity,
                ask_price,
            }) => {
                let market_id = match context.state.financial_system.exchange.good_to_symbol.get(good_id) {
                    Some(s) => s.clone(),
                    None => return Some(ResolutionResult::failure(vec![format!("No market for good {:?}", good_id)])),
                };
                vec![SimAction::Transaction(TransactionAction::PostMarketOrder {
                    agent_id: *agent_id,
                    market_id,
                    side: Side::Ask,
                    quantity: *quantity,
                    price: Some(Money::from_f64(*ask_price).unwrap_or_default()),
                    order_type: OrderType::Limit,
                })]
            }

            SimIntention::Production(ProductionIntention::PurchaseInputs {
                agent_id,
                good_id,
                quantity,
                max_price,
            }) => {
                let market_id = match context.state.financial_system.exchange.good_to_symbol.get(good_id) {
                    Some(s) => s.clone(),
                    None => return Some(ResolutionResult::failure(vec![format!("No market for good {:?}", good_id)])),
                };
                vec![SimAction::Transaction(TransactionAction::PostMarketOrder {
                    agent_id: *agent_id,
                    market_id,
                    side: Side::Bid,
                    quantity: *quantity,
                    price: Some(Money::from_f64(*max_price).unwrap_or_default()),
                    order_type: OrderType::Limit,
                })]
            }

            SimIntention::Consumption(ConsumptionIntention::SpendOnGood { agent_id, good_id, max_notional }) => {
                let consumer = match context.state.agents.consumers.get(agent_id) {
                    Some(c) => c,
                    None => return Some(ResolutionResult::failure(vec!["Consumer not found".to_string()])),
                };

                let market_id = match context.state.financial_system.exchange.good_to_symbol.get(good_id) {
                    Some(s) => s.clone(),
                    None => return Some(ResolutionResult::failure(vec![format!("No market for good {:?}", good_id)])),
                };

                let goods_config = &context.state.config.goods;

                let anchor = context
                    .state
                    .financial_system
                    .exchange
                    .fair_price_for_good(good_id)
                    .map(|m| m.to_f64())
                    .or_else(|| context.state.market_view(&market_id).and_then(|v| v.last_or_mid()))
                    .unwrap_or(1.0);

                let reservation_price = consumer
                    .adaptive
                    .reservation
                    .get(good_id)
                    .copied()
                    .unwrap_or(anchor * (1.0 + goods_config.reservation_nudge_up));

                let cap = anchor * goods_config.reservation_cap_mult;
                let limit_price = reservation_price.min(cap).max(0.01);

                let quantity_to_buy =
                    if limit_price > 1e-9 { (*max_notional / limit_price).floor().max(1.0) } else { 1.0 };

                vec![SimAction::Transaction(TransactionAction::PostMarketOrder {
                    agent_id: *agent_id,
                    market_id,
                    side: Side::Bid,
                    quantity: quantity_to_buy,
                    price: Some(Money::from_f64(limit_price).unwrap_or_default()),
                    order_type: OrderType::Limit,
                })]
            }

            SimIntention::Production(ProductionIntention::ApplyForJob { agent_id: _, market_id, application }) => {
                vec![SimAction::Transaction(TransactionAction::PostJobApplication {
                    market_id: *market_id,
                    application: application.clone(),
                })]
            }

            SimIntention::Production(ProductionIntention::HireWorkers { agent_id, count, wage_rate, max_wage }) => {
                let Some(market_id) = context.state.financial_system.find_general_labour_market() else {
                    return Some(ResolutionResult::failure(vec!["No general labour market found".to_string()]));
                };

                vec![SimAction::Transaction(TransactionAction::PostJobOffer {
                    market_id,
                    offer: JobOffer {
                        offer_id: Uuid::new_v4(),
                        firm_id: *agent_id,
                        quantity: *count,
                        wage_rate: *wage_rate,
                        value_per_hour: *max_wage,
                        hours_required: 40.0,
                    },
                })]
            }
            SimIntention::Banking(BankingIntention::PostOvernightFundingQuote { quote }) => {
                vec![SimAction::Transaction(TransactionAction::PostOvernightFundingQuote { quote: quote.clone() })]
            }

            _ => return None,
        };

        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::Transaction(TransactionIntention::PayWages { .. })
            | SimIntention::Fiscal(FiscalIntention::CollectTaxes { .. }) => Some(ResolutionPhase::Independent),

            SimIntention::Banking(BankingIntention::MarketMakeTreasuries { .. })
            | SimIntention::Production(ProductionIntention::PostGoodToMarket { .. })
            | SimIntention::Production(ProductionIntention::PurchaseInputs { .. })
            | SimIntention::Consumption(ConsumptionIntention::SpendOnGood { .. }) => Some(ResolutionPhase::Market),

            SimIntention::Production(ProductionIntention::ApplyForJob { .. })
            | SimIntention::Production(ProductionIntention::HireWorkers { .. }) => Some(ResolutionPhase::Independent),

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
            TransactionAction::PostOvernightFundingQuote { quote } => {
                let mut quote_with_ts = quote.clone();
                quote_with_ts.ts = state.ticknum as u64; // Set timestamp at execution
                let effect = StateEffect::Market(MarketEffect::PostOvernightFundingQuote(quote_with_ts));
                DomainResult::success(vec![effect])
            }
        }
    }

    fn settle_trade(&self, trade: &Trade, state: &SimState) -> DomainResult {
        let fs = &state.financial_system;
        if let Some(inst_id) = fs.exchange.symbol_to_inst.get(&trade.market_id).copied() {
            let mut effects = Vec::new();

            if trade.buyer == state.financial_system.central_bank.id
                && trade.seller == state.financial_system.government.id
            {
                let instruction = SettlementInstruction {
                    instruction_id: Uuid::new_v4(),
                    trade_id: trade.trade_id,
                    seller: trade.seller,
                    buyer: trade.buyer,
                    instrument_id: inst_id,
                    quantity: trade.quantity,
                    cash_amount: (trade.price * trade.quantity).to_f64(),
                    settlement_date: state.current_date,
                    status: SettlementStatus::Pending,
                };
                effects.push(StateEffect::Financial(FinancialEffect::RecordSettlementInstruction(instruction)));

                let payment_amount = (trade.price * trade.quantity).to_f64();
                if let Some((_tga_id, _)) = state.financial_system.find_government_tga_account() {
                    let payment = PaymentInstruction {
                        id: Uuid::new_v4(),
                        from_bank: state.financial_system.central_bank.id,
                        to_bank: state.financial_system.central_bank.id,
                        payer: state.financial_system.central_bank.id,
                        payee: state.financial_system.government.id,
                        amount: payment_amount,
                        context: TransactionContext::TradeSettlement { trade_id: trade.trade_id },
                        priority: PaymentPriority::Urgent,
                        earliest_release_tick: state.ticknum,
                        deadline_tick: state.ticknum + 1,
                    };
                    effects.push(StateEffect::Financial(FinancialEffect::QueuePayment(payment)));
                } else {
                    let tga_instrument = Instrument::cash(
                        InstrumentId(Uuid::new_v4()),
                        state.financial_system.central_bank.id,
                        CashType::TreasuryGeneralAccount,
                        Currency::USD,
                        dec!(0),
                    )
                    .build();
                    effects.push(StateEffect::Financial(FinancialEffect::CreateInstrument {
                        instrument: tga_instrument,
                        creditor: state.financial_system.government.id,
                        debtor: state.financial_system.central_bank.id,
                        quantity: payment_amount,
                    }));
                }
                return DomainResult::success(effects);
            }

            let (_, from_bank) = match state.financial_system.find_agent_liquid_account(&trade.buyer) {
                Some((_, bank_id)) => ((), bank_id),
                None => {
                    return DomainResult::failure(vec![format!(
                        "Could not find liquid account for buyer {}",
                        trade.buyer
                    )]);
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

            let instruction = SettlementInstruction {
                instruction_id: Uuid::new_v4(),
                trade_id: trade.trade_id,
                seller: trade.seller,
                buyer: trade.buyer,
                instrument_id: inst_id,
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
        } else if let Some(good_id) = fs.exchange.symbol_to_good.get(&trade.market_id).copied() {
            let mut effects = vec![];
            let revenue = (trade.price * trade.quantity).to_f64();
            let unit_cost = state
                .financial_system
                .get_agent_inventory(&trade.seller)
                .get(&good_id)
                .map(|it| it.unit_cost.to_f64())
                .unwrap_or(0.0);
            let cogs = unit_cost * trade.quantity;

            effects.push(StateEffect::Agent(AgentEffect::UpdateRevenue { id: trade.seller, revenue }));
            effects.push(StateEffect::Agent(AgentEffect::RecordCogs { id: trade.seller, amount: cogs }));

            effects.push(StateEffect::Inventory(InventoryEffect::RemoveInventory {
                owner: trade.seller,
                good_id,
                quantity: trade.quantity,
            }));

            effects.push(StateEffect::Inventory(InventoryEffect::AddInventory {
                owner: trade.buyer,
                good_id,
                quantity: trade.quantity,
                unit_cost: trade.price.to_f64(),
            }));

            let (_, buyer_bank) = match state.financial_system.find_agent_liquid_account(&trade.buyer) {
                Some(acc) => acc,
                None => {
                    return DomainResult::failure(vec![format!("Buyer {} has no liquid account", trade.buyer)]);
                }
            };

            let (_, seller_bank) = match state.financial_system.find_agent_liquid_account(&trade.seller) {
                Some(acc) => acc,
                None => {
                    return DomainResult::failure(vec![format!("Seller {} has no liquid account", trade.seller)]);
                }
            };

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
        } else {
            DomainResult::empty()
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
        &self, agent_id: AgentId, market_id: Symbol, side: Side, quantity: f64, price: Option<Money>,
        order_type: OrderType,
    ) -> DomainResult {
        let order =
            Order { id: Uuid::new_v4(), agent_id, side, quantity, price, order_type, market: market_id.clone() };
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
        &self, agent_id: AgentId, maturity_date: NaiveDate, bid_yield_bps: BasisPoints, ask_yield_bps: BasisPoints,
        quantity: f64, state: &SimState,
    ) -> Vec<SimAction> {
        let fs = &state.financial_system;
        let mut actions = vec![];

        let on_the_run_ids: Vec<InstrumentId> = fs
            .exchange
            .index
            .by_bond_type
            .get(&BondType::Government)
            .map(|ids| {
                ids.iter()
                    .filter(|id| {
                        fs.instruments
                            .instruments
                            .get(id)
                            .and_then(|inst| match &inst.instrument_type {
                                InstrumentType::Debt(DebtInstrument::Bond(d)) => Some(d.maturity_date == maturity_date),
                                _ => None,
                            })
                            .unwrap_or(false)
                    })
                    .copied()
                    .collect()
            })
            .unwrap_or_default();

        for inst_id in on_the_run_ids {
            let market_id = match fs.exchange.inst_to_symbol.get(&inst_id) {
                Some(s) => s.clone(),
                None => continue,
            };

            if let Some(inst) = fs.instruments.instruments.get(&inst_id) {
                if let Some(details) = inst.instrument_type.as_bond() {
                    let pricer = GovTermStructurePricer::new(
                        details.clone(),
                        TermStructureMethod::default(),
                        fs.pricing_feeds.clone(),
                    );

                    let bid_price = pricer
                        .price_from_yield(&inst_id, bps_to_decimal(bid_yield_bps).to_f64().unwrap_or_default())
                        .unwrap_or(Money::ZERO);

                    let ask_price = pricer
                        .price_from_yield(&inst_id, bps_to_decimal(ask_yield_bps).to_f64().unwrap_or_default())
                        .unwrap_or(Money::ZERO);

                    if bid_price > Money::ZERO {
                        actions.push(SimAction::Transaction(TransactionAction::PostMarketOrder {
                            agent_id,
                            market_id: market_id.clone(),
                            side: Side::Bid,
                            quantity,
                            price: Some(bid_price),
                            order_type: OrderType::Limit,
                        }));
                    }
                    if ask_price > Money::ZERO {
                        actions.push(SimAction::Transaction(TransactionAction::PostMarketOrder {
                            agent_id,
                            market_id,
                            side: Side::Ask,
                            quantity,
                            price: Some(ask_price),
                            order_type: OrderType::Limit,
                        }));
                    }
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
