use crate::{
    Any, Domain, DomainResult, ResolutionContext, ResolutionPhase, ResolutionResult, inventory,
};
use serde::{Deserialize, Serialize};
use sim_core::*;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FiscalDomain {}

impl FiscalDomain {
    pub fn new() -> Self {
        Self {}
    }
}

impl Domain for FiscalDomain {
    fn name(&self) -> &'static str {
        "Fiscal"
    }

    fn resolve_intention(
        &self,
        intention: &SimIntention,
        context: &ResolutionContext,
    ) -> Option<ResolutionResult> {
        let actions = match intention {
            SimIntention::AnnounceDebtAuction {
                government_id,
                maturity_date,
                coupon_rate,
                quantity_to_issue,
            } => {
                const FACE_VALUE: f64 = 1000.0;
                let issue_date = context.state.current_date;

                let _new_bond = match Instrument::bond(
                    InstrumentId(Uuid::new_v4()),
                    *government_id,
                    BondType::Government,
                    Money::from(FACE_VALUE as u64),
                    issue_date,
                    *maturity_date,
                )
                .coupon_bps(*coupon_rate)
                .frequency(2)
                .rating(CreditRating::AAA)
                .auto_market()
                .build()
                {
                    Ok(bond) => bond,
                    Err(e) => return Some(ResolutionResult::failure(vec![e.to_string()])),
                };

                vec![SimAction::Fiscal(FiscalAction::AnnounceDebtAuction {
                    government_id: *government_id,
                    maturity: *maturity_date,
                    quantity: *quantity_to_issue,
                    coupon_rate: *coupon_rate,
                })]
            }
            SimIntention::BidInDebtAuction {
                agent_id,
                auction_id,
                quantity,
                bid_price,
            } => {
                vec![SimAction::Fiscal(FiscalAction::BidInDebtAuction {
                    agent_id: *agent_id,
                    auction_id: *auction_id,
                    quantity: *quantity,
                    bid_price: *bid_price,
                })]
            }
            _ => return None,
        };
        Some(ResolutionResult::success(actions))
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::AnnounceDebtAuction { .. } => Some(ResolutionPhase::Independent),
            SimIntention::BidInDebtAuction { .. } => Some(ResolutionPhase::Market),
            _ => None,
        }
    }

    fn execute(&self, action: &SimAction, state: &SimState) -> DomainResult {
        let fiscal_action = match action {
            SimAction::Fiscal(action) => action,
            _ => return DomainResult::failure(vec!["Not a fiscal action".to_string()]),
        };

        if let Err(error) = self.validate(fiscal_action, state) {
            return DomainResult::failure(vec![error]);
        }

        match fiscal_action {
            FiscalAction::AnnounceDebtAuction {
                government_id,
                quantity,
                maturity,
                coupon_rate,
                ..
            } => {
                let issue_date = state.current_date;
                let new_instrument = match Instrument::bond(
                    InstrumentId(Uuid::new_v4()),
                    *government_id,
                    BondType::Government,
                    Money::from(1000 as u64),
                    issue_date,
                    *maturity,
                )
                .coupon_bps(*coupon_rate)
                .frequency(2)
                .rating(CreditRating::AAA)
                .auto_market()
                .build() {
                    Ok(bond) => bond,
                    Err(e) => return DomainResult::failure(vec![e.to_string()])
                };

                let auction = DebtAuction {
                    auction_id: Uuid::new_v4(),
                    instrument_id: new_instrument.id,
                    quantity_offered: *quantity,
                    status: AuctionStatus::Open,
                    bids: vec![],
                };

                let effects = vec![
                    StateEffect::Financial(FinancialEffect::CreateInstrument {
                        instrument: new_instrument,
                        creditor: *government_id,
                        debtor: *government_id,
                        quantity: *quantity as f64,
                    }),
                    StateEffect::Market(MarketEffect::OpenDebtAuction { auction }),
                ];
                DomainResult::success(effects)
            }
            FiscalAction::BidInDebtAuction { agent_id, auction_id, quantity, bid_price } => {
                let bid = AuctionBid {
                    agent_id: *agent_id,
                    quantity: *quantity,
                    price: *bid_price,
                };
                let effect = StateEffect::Market(MarketEffect::SubmitAuctionBid {
                    auction_id: *auction_id,
                    bid,
                });
                DomainResult::success(vec![effect])
            }
            _ => DomainResult::empty(),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl FiscalDomain {
    fn validate(&self, action: &FiscalAction, state: &SimState) -> Result<(), String> {
        match action {
            FiscalAction::AnnounceDebtAuction { government_id, quantity, maturity, .. } => {
                if *quantity == 0 { return Err("Cannot auction zero bonds".to_string()); }
                if *maturity <= state.current_date { return Err("Maturity date must be in the future.".to_string()); }
                if *government_id != state.financial_system.government.id { return Err("Invalid government ID for debt auction.".to_string()); }
                Ok(())
            }
            FiscalAction::BidInDebtAuction { agent_id, auction_id, .. } => {
                Validator::agent_exists(*agent_id, state)?;
                if !state.financial_system.exchange.open_auctions.contains_key(auction_id) {
                    return Err(format!("Auction {} not found or not open.", auction_id));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Fiscal",
        constructor: || Box::new(FiscalDomain::new()),
    }
}