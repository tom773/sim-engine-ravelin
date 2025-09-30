use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use sim_core::actions::monetary::{MonetaryAction, OMOType};
use sim_core::prelude::*;
use sim_core::types::markets::market::{FinancialProduct, MarketGeneric, MarketType, listing_key_from_instrument};
use sim_core::types::markets::orderbook::MarketDepthSummary;
use std::collections::HashMap;
use uuid::Uuid;

use super::agent_digest::instrument_label;
use super::state_digest::{InstrumentRegistryDigest, rate_to_f64};

pub(crate) const ORDERBOOK_DEPTH_LEVELS: usize = 10;
pub(crate) const OMO_HISTORY_LIMIT: usize = 25;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketsDigest {
    pub snapshots: Vec<MarketDigest>,
    pub most_active: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infrastructure: Option<MarketInfrastructureDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketInfrastructureDigest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub listings: Vec<MarketListingDigest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub omo_actions: Vec<OmoActionDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csd: Option<CsdDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtgs: Option<RtgsDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit_registry: Option<CreditRegistryDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overnight_markets: Option<OvernightMarketsDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labour_markets: Option<LabourMarketsDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketListingDigest {
    pub symbol: String,
    pub kind: MarketKindDigest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instrument_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub good_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labour_market_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listing_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OmoActionDigest {
    pub action_id: Uuid,
    pub tick: u32,
    pub agent_id: Uuid,
    pub description: String,
    pub amount: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterparty: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_bps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketDigest {
    pub market_id: String,
    pub label: String,
    pub kind: MarketKindDigest,
    pub last_price: Option<f64>,
    pub mid_price: Option<f64>,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub spread: Option<f64>,
    pub volume: f64,
    pub turnover: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<DepthDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_mid_bps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_bid_bps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_ask_bps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_last_bps: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MarketKindDigest {
    #[default]
    Financial,
    Goods,
    Labour,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DepthDigest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bids: Vec<DepthLevel>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub asks: Vec<DepthLevel>,
    pub bid_size_at_best: f64,
    pub ask_size_at_best: f64,
    pub total_bid_levels: usize,
    pub total_ask_levels: usize,
    pub total_bid_volume: f64,
    pub total_ask_volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DepthLevel {
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CsdDigest {
    pub custody_accounts_count: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pending_settlements: Vec<SettlementDigest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent_settlements: Vec<CompletedSettlementDigest>,
    pub registered_securities_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementDigest {
    pub trade_id: String,
    pub seller: Uuid,
    pub buyer: Uuid,
    pub instrument_id: String,
    pub quantity: f64,
    pub cash_amount: f64,
    pub settlement_date: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedSettlementDigest {
    pub trade_id: String,
    pub settlement_time: String,
    pub instrument_id: String,
    pub quantity: f64,
    pub seller: Uuid,
    pub buyer: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RtgsDigest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pending_payments: Vec<PaymentDigest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent_settled: Vec<PaymentDigest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent_rejected: Vec<RejectedPaymentDigest>,
    pub enabled: bool,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentDigest {
    pub payment_id: String,
    pub from_bank: Uuid,
    pub to_bank: Uuid,
    pub payer: Uuid,
    pub payee: Uuid,
    pub amount: f64,
    pub priority: String,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedPaymentDigest {
    pub payment: PaymentDigest,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreditRegistryDigest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pending_applications: Vec<LoanApplicationDigest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub active_facilities: Vec<CreditFacilityDigest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub active_loans: Vec<LoanDigest>,
    pub total_applications: usize,
    pub total_facilities: usize,
    pub total_loans: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanApplicationDigest {
    pub application_id: String,
    pub borrower_id: Uuid,
    pub requested_amount: f64,
    pub purpose: String,
    pub status: String,
    pub application_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditFacilityDigest {
    pub facility_id: String,
    pub lender: Uuid,
    pub borrower: Uuid,
    pub facility_type: String,
    pub commitment_amount: f64,
    pub drawn_amount: f64,
    pub available_amount: f64,
    pub utilization_ratio: f64,
    pub spread_bps: f64,
    pub expiry_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanDigest {
    pub loan_id: String,
    pub lender: Uuid,
    pub borrower: Uuid,
    pub loan_type: String,
    pub principal: f64,
    pub outstanding: f64,
    pub rate_bps: f64,
    pub maturity_date: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OvernightMarketsDigest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fedfunds_quotes: Vec<OvernightQuoteDigest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub repo_quotes: Vec<OvernightQuoteDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvernightQuoteDigest {
    pub agent: Uuid,
    pub side: String,
    pub notional: f64,
    pub limit_rate_bps: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub haircut: Option<f64>,
    pub min_fill: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LabourMarketsDigest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub markets: Vec<LabourMarketDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabourMarketDigest {
    pub market_id: String,
    pub symbol: String,
}

pub(crate) fn compute_markets(state: &SimState) -> MarketsDigest {
    let mut snapshots: Vec<MarketDigest> = Vec::new();

    for (symbol, market) in &state.financial_system.exchange.markets {
        match market {
            MarketType::Financial(fin_market) => {
                let inst_id = &fin_market.key;
                let view = state.market_view(symbol).unwrap_or_default();
                let (yield_mid, yield_last, yield_bid, yield_ask) = calculate_yields(inst_id, fin_market);
                let depth = depth_from_summary(fin_market.book.depth_summary(), ORDERBOOK_DEPTH_LEVELS);
                let label = state
                    .financial_system
                    .instruments
                    .instruments
                    .get(inst_id)
                    .map(|i| instrument_label(i, &state.financial_system))
                    .unwrap_or_else(|| "Financial Market".into());

                snapshots.push(MarketDigest {
                    market_id: symbol.to_string(),
                    label,
                    kind: MarketKindDigest::Financial,
                    last_price: view.last,
                    mid_price: view.mid,
                    best_bid: fin_market.book.best_bid().map(|m| m.to_f64()),
                    best_ask: fin_market.book.best_ask().map(|m| m.to_f64()),
                    spread: fin_market.book.spread().map(|m| m.to_f64()),
                    volume: view.volume,
                    turnover: view.turnover,
                    depth,
                    yield_mid_bps: yield_mid,
                    yield_last_bps: yield_last,
                    yield_bid_bps: yield_bid,
                    yield_ask_bps: yield_ask,
                });
            }
            MarketType::Goods(goods_market) => {
                let view = state.market_view(symbol).unwrap_or_default();
                let depth = depth_from_summary(goods_market.book.depth_summary(), ORDERBOOK_DEPTH_LEVELS);
                let label = state
                    .financial_system
                    .goods
                    .goods
                    .get(&goods_market.key)
                    .map(|g| g.name.clone())
                    .unwrap_or_else(|| "Goods Market".into());

                snapshots.push(MarketDigest {
                    market_id: symbol.to_string(),
                    label,
                    kind: MarketKindDigest::Goods,
                    last_price: view.last,
                    mid_price: view.mid,
                    best_bid: goods_market.book.best_bid().map(|m| m.to_f64()),
                    best_ask: goods_market.book.best_ask().map(|m| m.to_f64()),
                    spread: goods_market.book.spread().map(|m| m.to_f64()),
                    volume: view.volume,
                    turnover: view.turnover,
                    depth,
                    yield_mid_bps: None,
                    yield_last_bps: None,
                    yield_bid_bps: None,
                    yield_ask_bps: None,
                });
            }
            MarketType::Labour(_labour_market) => {
                snapshots.push(MarketDigest {
                    market_id: symbol.to_string(),
                    label: "Labour Market".into(),
                    kind: MarketKindDigest::Labour,
                    last_price: None,
                    mid_price: None,
                    best_bid: None,
                    best_ask: None,
                    spread: None,
                    volume: 0.0,
                    turnover: 0.0,
                    depth: None,
                    yield_mid_bps: None,
                    yield_last_bps: None,
                    yield_bid_bps: None,
                    yield_ask_bps: None,
                });
            }
        }
    }

    snapshots.sort_by(|a, b| b.volume.partial_cmp(&a.volume).unwrap_or(std::cmp::Ordering::Equal));
    let most_active: Vec<String> = snapshots.iter().take(5).map(|m| m.market_id.clone()).collect();

    MarketsDigest { snapshots, most_active, infrastructure: None }
}

fn calculate_yields(inst_id: &InstrumentId, market: &MarketGeneric<FinancialProduct>) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    let mid = market
        .book
        .mid_price()
        .and_then(|price| market.pricer.yield_from_price(inst_id, price))
        .and_then(|r| r.to_f64());
    let last = market
        .book
        .last_price
        .and_then(|price| market.pricer.yield_from_price(inst_id, price))
        .and_then(|r| r.to_f64());
    let bid = market
        .book
        .best_bid()
        .and_then(|price| market.pricer.yield_from_price(inst_id, price))
        .and_then(|r| r.to_f64());
    let ask = market
        .book
        .best_ask()
        .and_then(|price| market.pricer.yield_from_price(inst_id, price))
        .and_then(|r| r.to_f64());
    (mid, last, bid, ask)
}

fn depth_from_summary(summary: MarketDepthSummary, max_levels: usize) -> Option<DepthDigest> {
    if summary.bid_levels.is_empty() && summary.ask_levels.is_empty() {
        return None;
    }

    fn ordered_levels(levels: &HashMap<Decimal, f64>, descending: bool, limit: usize) -> Vec<DepthLevel> {
        let mut pairs: Vec<_> = levels.iter().map(|(price, qty)| (*price, *qty)).collect();
        if descending {
            pairs.sort_by(|a, b| b.0.cmp(&a.0));
        } else {
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
        }
        pairs
            .into_iter()
            .take(limit)
            .map(|(price, quantity)| DepthLevel { price: price.to_f64().unwrap_or_default(), quantity })
            .collect()
    }

    let MarketDepthSummary { bid_levels, ask_levels, bid_size_at_best, ask_size_at_best, .. } = summary;

    let bids = ordered_levels(&bid_levels, true, max_levels);
    let asks = ordered_levels(&ask_levels, false, max_levels);

    let total_bid_volume: f64 = bid_levels.values().copied().sum();
    let total_ask_volume: f64 = ask_levels.values().copied().sum();

    Some(DepthDigest {
        bids,
        asks,
        bid_size_at_best,
        ask_size_at_best,
        total_bid_levels: bid_levels.len(),
        total_ask_levels: ask_levels.len(),
        total_bid_volume,
        total_ask_volume,
    })
}

pub(crate) fn build_market_infrastructure(
    state: &SimState, markets: &MarketsDigest, registry: &InstrumentRegistryDigest,
) -> MarketInfrastructureDigest {
    let exchange = &state.financial_system.exchange;
    let mut listings: Vec<MarketListingDigest> = Vec::with_capacity(exchange.markets.len());

    for (symbol, market) in &exchange.markets {
        let symbol_str = symbol.to_string();
        let mut listing = MarketListingDigest {
            symbol: symbol_str.clone(),
            kind: match market {
                MarketType::Financial(_) => MarketKindDigest::Financial,
                MarketType::Goods(_) => MarketKindDigest::Goods,
                MarketType::Labour(_) => MarketKindDigest::Labour,
            },
            instrument_id: None,
            good_id: None,
            labour_market_id: None,
            label: None,
            listing_key: None,
        };

        match market {
            MarketType::Financial(fin_market) => {
                let inst_id = fin_market.key;
                listing.instrument_id = Some(inst_id.to_string());
                if let Some(inst) = state.financial_system.instruments.instruments.get(&inst_id) {
                    listing.label = Some(inst.label().to_string());
                    listing.listing_key = Some(format!("{:?}", listing_key_from_instrument(inst)));
                } else if let Some(label) = registry
                    .instruments
                    .iter()
                    .find(|meta| meta.instrument_id == inst_id.to_string())
                    .map(|meta| meta.label.clone())
                {
                    listing.label = Some(label);
                }
            }
            MarketType::Goods(goods_market) => {
                let good_id = goods_market.key;
                listing.good_id = Some(good_id.to_string());
                listing.label = registry
                    .goods
                    .iter()
                    .find(|good| good.good_id == good_id.to_string())
                    .map(|good| good.name.clone())
                    .or_else(|| state.financial_system.goods.goods.get(&good_id).map(|g| g.name.clone()));
            }
            MarketType::Labour(_labour_market) => {
                if let Some((labour_id, _)) =
                    exchange.labour_to_symbol.iter().find(|(_, registered_symbol)| *registered_symbol == symbol)
                {
                    listing.labour_market_id = Some(labour_id.to_string());
                }
                if listing.label.is_none() {
                    listing.label = Some("Labour Market".into());
                }
            }
        }

        if listing.label.is_none() {
            listing.label = markets
                .snapshots
                .iter()
                .find(|snapshot| snapshot.market_id == listing.symbol)
                .map(|snapshot| snapshot.label.clone());
        }

        listings.push(listing);
    }

    listings.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    let omo_actions = collect_recent_omo_actions(&state.history, OMO_HISTORY_LIMIT);

    let csd = build_csd_digest(&state.financial_system.clearing_house.csd);
    let rtgs = build_rtgs_digest(&state.financial_system.rtgs, &state.financial_system.rtgs_policy);
    let credit_registry = build_credit_registry_digest(&state.financial_system.credit_registry);
    let overnight_markets = build_overnight_markets_digest(&state.financial_system.funding_markets);
    let labour_markets = build_labour_markets_digest(exchange);

    MarketInfrastructureDigest {
        listings,
        omo_actions,
        csd: Some(csd),
        rtgs: Some(rtgs),
        credit_registry: Some(credit_registry),
        overnight_markets: Some(overnight_markets),
        labour_markets: Some(labour_markets),
    }
}

pub(crate) fn collect_recent_omo_actions(history: &SimHistory, limit: usize) -> Vec<OmoActionDigest> {
    let mut actions: Vec<OmoActionDigest> = Vec::new();

    for record in history.tick_records.iter().rev() {
        for action in &record.actions {
            if let SimAction::Monetary(MonetaryAction::OpenMarketOperation { cb_id, operation_type, amount }) =
                &action.action
            {
                let (rate_bps, term_days) = match operation_type {
                    OMOType::QuantitativeEasing | OMOType::QuantitativeTightening => (None, None),
                    OMOType::Repo { rate_bps, term_days } | OMOType::ReverseRepo { rate_bps, term_days } => {
                        (Some(rate_to_f64(*rate_bps)), Some(*term_days))
                    }
                };

                actions.push(OmoActionDigest {
                    action_id: action.id,
                    tick: record.tick_number,
                    agent_id: cb_id.0,
                    description: operation_type.description(),
                    amount: *amount,
                    counterparty: None,
                    rate_bps,
                    term_days,
                });
            }
        }

        if actions.len() >= limit {
            break;
        }
    }

    actions.sort_by(|a, b| a.tick.cmp(&b.tick));
    if actions.len() > limit {
        actions = actions.split_off(actions.len() - limit);
    }
    actions
}

fn build_csd_digest(csd: &sim_core::types::system::csd::CentralSecuritiesDepository) -> CsdDigest {
    let pending_settlements: Vec<SettlementDigest> = csd
        .pending_settlements
        .values()
        .map(|s| SettlementDigest {
            trade_id: s.trade_id.to_string(),
            seller: s.seller.0,
            buyer: s.buyer.0,
            instrument_id: s.instrument_id.to_string(),
            quantity: s.quantity,
            cash_amount: s.cash_amount,
            settlement_date: s.settlement_date.format("%Y-%m-%d").to_string(),
            status: format!("{:?}", s.status),
        })
        .collect();

    let recent_settlements: Vec<CompletedSettlementDigest> = csd
        .settlement_history
        .iter()
        .rev()
        .take(50)
        .map(|s| CompletedSettlementDigest {
            trade_id: s.trade_id.to_string(),
            settlement_time: s.settlement_time.format("%Y-%m-%d").to_string(),
            instrument_id: s.instrument_id.to_string(),
            quantity: s.quantity,
            seller: s.seller.0,
            buyer: s.buyer.0,
        })
        .collect();

    CsdDigest {
        custody_accounts_count: csd.custody_accounts.len(),
        pending_settlements,
        recent_settlements,
        registered_securities_count: csd.registered_securities.len(),
    }
}

fn build_rtgs_digest(
    rtgs: &sim_core::types::system::rtgs_types::RtgsQueue,
    policy: &sim_core::types::system::rtgs_types::RtgsPolicy,
) -> RtgsDigest {
    let pending_payments: Vec<PaymentDigest> = rtgs
        .pending
        .iter()
        .map(|p| PaymentDigest {
            payment_id: p.id.to_string(),
            from_bank: p.from_bank.0,
            to_bank: p.to_bank.0,
            payer: p.payer.0,
            payee: p.payee.0,
            amount: p.amount,
            priority: format!("{:?}", p.priority),
            context: format!("{:?}", p.context),
        })
        .collect();

    let recent_settled: Vec<PaymentDigest> = rtgs
        .settled
        .iter()
        .rev()
        .take(50)
        .map(|p| PaymentDigest {
            payment_id: p.id.to_string(),
            from_bank: p.from_bank.0,
            to_bank: p.to_bank.0,
            payer: p.payer.0,
            payee: p.payee.0,
            amount: p.amount,
            priority: format!("{:?}", p.priority),
            context: format!("{:?}", p.context),
        })
        .collect();

    let recent_rejected: Vec<RejectedPaymentDigest> = rtgs
        .rejected
        .iter()
        .rev()
        .take(50)
        .map(|(p, reason)| RejectedPaymentDigest {
            payment: PaymentDigest {
                payment_id: p.id.to_string(),
                from_bank: p.from_bank.0,
                to_bank: p.to_bank.0,
                payer: p.payer.0,
                payee: p.payee.0,
                amount: p.amount,
                priority: format!("{:?}", p.priority),
                context: format!("{:?}", p.context),
            },
            reason: reason.clone(),
        })
        .collect();

    RtgsDigest {
        pending_payments,
        recent_settled,
        recent_rejected,
        enabled: policy.enabled,
        mode: format!("{:?}", policy.mode),
    }
}

fn build_credit_registry_digest(registry: &sim_core::types::instrument::credit::CreditRegistry) -> CreditRegistryDigest {
    use sim_core::types::instrument::credit::ApplicationStatus;

    let pending_applications: Vec<LoanApplicationDigest> = registry
        .applications
        .values()
        .filter(|app| matches!(app.status, ApplicationStatus::Pending | ApplicationStatus::UnderReview))
        .take(100)
        .map(|app| LoanApplicationDigest {
            application_id: app.application_id.to_string(),
            borrower_id: app.borrower_id.0,
            requested_amount: app.requested_amount,
            purpose: format!("{:?}", app.purpose),
            status: format!("{:?}", app.status),
            application_date: app.application_date.format("%Y-%m-%d").to_string(),
        })
        .collect();

    let active_facilities: Vec<CreditFacilityDigest> = registry
        .facilities
        .values()
        .take(100)
        .map(|fac| {
            let utilization = if fac.state.commitment_amount > Money::ZERO {
                (fac.state.drawn_amount / fac.state.commitment_amount).to_f64().unwrap_or(0.0)
            } else {
                0.0
            };
            CreditFacilityDigest {
                facility_id: fac.state.facility_id.to_string(),
                lender: fac.state.lender.0,
                borrower: fac.state.borrower.0,
                facility_type: format!("{:?}", fac.state.facility_type),
                commitment_amount: fac.state.commitment_amount.to_f64(),
                drawn_amount: fac.state.drawn_amount.to_f64(),
                available_amount: fac.state.available_amount.to_f64(),
                utilization_ratio: utilization,
                spread_bps: rate_to_f64(fac.state.spread_bps),
                expiry_date: fac.state.expiry_date.format("%Y-%m-%d").to_string(),
            }
        })
        .collect();

    let active_loans: Vec<LoanDigest> = registry
        .loans
        .values()
        .take(100)
        .map(|loan| LoanDigest {
            loan_id: loan.state.loan_id.to_string(),
            lender: loan.state.lender.0,
            borrower: loan.state.borrower.0,
            loan_type: format!("{:?}", loan.state.loan_type),
            principal: loan.state.archetype.principal.to_f64(),
            outstanding: loan.state.outstanding_principal.to_f64(),
            rate_bps: rate_to_f64(loan.state.archetype.rate_structure.spread_bps),
            maturity_date: loan.state.maturity_date.format("%Y-%m-%d").to_string(),
            status: format!("{:?}", loan.status),
        })
        .collect();

    CreditRegistryDigest {
        pending_applications,
        active_facilities,
        active_loans,
        total_applications: registry.applications.len(),
        total_facilities: registry.facilities.len(),
        total_loans: registry.loans.len(),
    }
}

fn build_overnight_markets_digest(markets: &sim_core::types::markets::overnight::OvernightFundingBooks) -> OvernightMarketsDigest {
    let fedfunds_quotes: Vec<OvernightQuoteDigest> = markets
        .fedfunds_on
        .iter()
        .map(|q| OvernightQuoteDigest {
            agent: q.agent.0,
            side: format!("{:?}", q.side),
            notional: q.notional,
            limit_rate_bps: rate_to_f64(q.limit_rate_bps),
            haircut: q.haircut.map(|h| rate_to_f64(h)),
            min_fill: q.min_fill,
        })
        .collect();

    let repo_quotes: Vec<OvernightQuoteDigest> = markets
        .repo_gc1d
        .iter()
        .map(|q| OvernightQuoteDigest {
            agent: q.agent.0,
            side: format!("{:?}", q.side),
            notional: q.notional,
            limit_rate_bps: rate_to_f64(q.limit_rate_bps),
            haircut: q.haircut.map(|h| rate_to_f64(h)),
            min_fill: q.min_fill,
        })
        .collect();

    OvernightMarketsDigest {
        fedfunds_quotes,
        repo_quotes,
    }
}

fn build_labour_markets_digest(exchange: &sim_core::types::markets::market::Exchange) -> LabourMarketsDigest {
    let markets: Vec<LabourMarketDigest> = exchange
        .labour_to_symbol
        .iter()
        .map(|(labour_id, symbol)| LabourMarketDigest {
            market_id: labour_id.to_string(),
            symbol: symbol.to_string(),
        })
        .collect();

    LabourMarketsDigest { markets }
}

