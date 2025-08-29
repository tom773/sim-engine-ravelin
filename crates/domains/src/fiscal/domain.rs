use crate::{Any, Domain, DomainResult, ResolutionContext, ResolutionPhase, ResolutionResult, inventory};
use serde::{Deserialize, Serialize};
use sim_core::*;

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

    fn resolve_intention(&self, intention: &SimIntention, context: &ResolutionContext) -> Option<ResolutionResult> {
        match intention {
            SimIntention::IssueDebtToRaise { government_id, tenor, amount_to_raise, coupon_rate } => {
                Some(self.resolve_debt_issuance(*government_id, *tenor, *amount_to_raise, *coupon_rate, context))
            }
            _ => None,
        }
    }

    fn resolution_phase(&self, intention: &SimIntention) -> Option<ResolutionPhase> {
        match intention {
            SimIntention::IssueDebtToRaise { .. } => Some(ResolutionPhase::Dependent),
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
            FiscalAction::IssueDebt { government_id, tenor, face_value, quantity, coupon_rate } => {
                self.execute_debt_issuance(*government_id, *tenor, *face_value, *quantity, *coupon_rate, state)
            }
            FiscalAction::ChangeTaxRate { .. } => DomainResult::empty(),
            FiscalAction::SetSpendingTarget { .. } => DomainResult::empty(),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl FiscalDomain {
    fn resolve_debt_issuance(
        &self, government_id: AgentId, tenor: Tenor, amount_to_raise: f64, coupon_rate: BasisPoints,
        context: &ResolutionContext,
    ) -> ResolutionResult {
        const FACE_VALUE: f64 = 1000.0;

        let fs = &context.state.financial_system;
        let market_id_enum = FinancialMarketId::Treasury { tenor };
        let market_id = MarketId::Financial(market_id_enum.clone());

        let discovered_price = self.discover_market_price(&market_id_enum, &market_id, fs, context);

        match discovered_price {
            Some(price) => {
                let quantity = (amount_to_raise / price).ceil() as u32;

                if quantity > 0 {
                    let actions = vec![SimAction::Fiscal(FiscalAction::IssueDebt {
                        government_id,
                        tenor,
                        face_value: FACE_VALUE,
                        quantity,
                        coupon_rate,
                    })];

                    ResolutionResult::success(actions)
                } else {
                    ResolutionResult::failure(vec!["Cannot determine valid quantity for debt issuance".to_string()])
                }
            }
            None => ResolutionResult::failure(vec![format!("No market price available for {} treasury bonds", tenor)]),
        }
    }

    fn discover_market_price(
        &self, market_id_enum: &FinancialMarketId, market_id: &MarketId, fs: &FinancialSystem,
        context: &ResolutionContext,
    ) -> Option<f64> {
        if let Some(market) = fs.exchange.financial_market(market_id_enum) {
            if let Some(best_bid) = market.order_book.best_bid() {
                return Some(best_bid.price);
            }
        }

        if let Some(market_view) = context.state.market_view(market_id) {
            if let Some(last_price) = market_view.last_or_mid() {
                return Some(last_price);
            }
        }

        if let Some(theoretical_price) = self.calculate_theoretical_price(market_id_enum, fs) {
            return Some(theoretical_price);
        }

        const FACE_VALUE: f64 = 1000.0;
        Some(FACE_VALUE * 0.98)
    }

    fn calculate_theoretical_price(&self, market_id: &FinancialMarketId, fs: &FinancialSystem) -> Option<f64> {
        if let FinancialMarketId::Treasury { tenor } = market_id {
            const FACE_VALUE: f64 = 1000.0;
            const FREQUENCY: usize = 2;

            let policy_rate_bps = fs.central_bank.policy_rate_bps;
            let term_premium = match tenor {
                Tenor::T1M => 2.0,
                Tenor::T2M => 3.0,
                Tenor::T3M => 7.0,
                Tenor::T6M => 10.0,
                Tenor::T1Y => 12.0,
                Tenor::T2Y => 20.0,
                Tenor::T5Y => 40.0,
                Tenor::T10Y => 60.0,
                Tenor::T30Y => 80.0,
            };

            let estimated_yield_bps = policy_rate_bps + term_premium;
            let coupon_rate_bps = policy_rate_bps;

            let price = pricing::bond_price(
                FACE_VALUE,
                bps_to_decimal(coupon_rate_bps),
                bps_to_decimal(estimated_yield_bps),
                tenor.to_years(),
                FREQUENCY,
            );

            Some(price)
        } else {
            None
        }
    }
}

impl FiscalDomain {
    fn validate(&self, action: &FiscalAction, state: &SimState) -> Result<(), String> {
        match action {
            FiscalAction::IssueDebt { government_id: _, quantity, .. } => {
                if *quantity == 0 {
                    return Err("Cannot issue zero bonds".to_string());
                }
                if let Some(g) = state.financial_system.get_government() {
                    if g.tax_rates.income_tax <= 0.0 {
                        return Err("Government must have a positive income tax rate to issue debt".to_string());
                    }
                } else {
                    return Err("Government agent not found".to_string());
                }

                Ok(())
            }
            FiscalAction::ChangeTaxRate { .. } | FiscalAction::SetSpendingTarget { .. } => Ok(()),
        }
    }

    fn execute_debt_issuance(
        &self, government_id: AgentId, tenor: Tenor, face_value: f64, quantity: u32, _coupon_rate: BasisPoints,
        state: &SimState,
    ) -> DomainResult {
        let market_id_enum = FinancialMarketId::Treasury { tenor };

        let selling_price = state
            .financial_system
            .exchange
            .financial_market(&market_id_enum)
            .and_then(|market| market.best_bid())
            .unwrap_or_else(|| {
                self.calculate_theoretical_price(&market_id_enum, &state.financial_system).unwrap_or(face_value * 0.98)
                    * 0.995 // Sell at a 0.5% discount to theoretical
            });

        let ask = Ask { agent_id: government_id, quantity: quantity as f64, price: selling_price };

        let market_effect = StateEffect::Market(MarketEffect::PlaceOrderInBook {
            market_id: MarketId::Financial(market_id_enum),
            order: Order::Ask(ask),
        });

        DomainResult::success(vec![market_effect])
    }
}

inventory::submit! {
    crate::DomainRegistration {
        name: "Fiscal",
        constructor: || Box::new(FiscalDomain::new()),
    }
}
