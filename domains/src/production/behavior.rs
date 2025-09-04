use chrono::Datelike;
use rand::prelude::*;
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::any::Any;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionFirmDecisionModel {
    pub target_markup: f64,
    pub base_wage: f64,
    pub target_employees: usize,
}

impl Default for ProductionFirmDecisionModel {
    fn default() -> Self {
        Self {
            target_markup: 1.25,
            base_wage: 25.0,
            target_employees: 3,
        }
    }
}

#[typetag::serde]
impl DecisionModel for ProductionFirmDecisionModel {
    fn name(&self) -> &str {
        "Production Firm"
    }
    fn decide(
        &self,
        agent: &dyn Any,
        state: &SimState,
        _rng: &mut dyn RngCore,
    ) -> Vec<SimIntention> {
        let firm = match agent.downcast_ref::<Firm>() {
            Some(f) => f,
            None => return vec![],
        };

        let mut intentions = Vec::new();

        self.handle_hiring(firm, state, &mut intentions);
        self.handle_production(firm, state, &mut intentions);
        self.handle_wages(firm, state, &mut intentions);
        self.handle_sales(firm, state, &mut intentions);
        self.handle_input_purchases(firm, state, &mut intentions);

        intentions
    }
}

impl ProductionFirmDecisionModel {
    fn handle_hiring(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        let current_employees = firm.employees.len();
        if current_employees < self.target_employees {
            if state.financial_system.find_general_labour_market().is_none() {
                return;
            }

            let positions_to_fill = self.target_employees - current_employees;

            intentions.push(SimIntention::HireWorkers {
                agent_id: firm.id,
                count: positions_to_fill as u32,
                wage_rate: self.base_wage,
            });
        }
    }

    fn handle_production(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        if firm.employees.is_empty() {
            return;
        }

        let fs = &state.financial_system;

        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = fs.goods.recipes.get(&recipe_id) {
                let inventory = fs.get_agent_inventory(&firm.id);

                let can_produce = recipe.inputs.iter().all(|input| {
                    inventory
                        .get(&input.good_id)
                        .map_or(false, |item| item.quantity >= input.quantity)
                });

                if can_produce {
                    intentions.push(SimIntention::Produce {
                        agent_id: firm.id,
                        recipe_id,
                        batches: 1,
                    });
                }
            }
        }
    }

    fn handle_wages(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        if state.current_date.day() == 3 || state.current_date.day() == 17 {
            for (employee_id, contract) in &firm.employees {
                let fortnightly_wage = (contract.wage_rate * contract.hours) * 2.0;
                if fortnightly_wage > 0.0 {
                    intentions.push(SimIntention::PayWages {
                        employer: firm.id,
                        employee: *employee_id,
                        amount: fortnightly_wage,
                    });
                }
            }
        }
    }

    fn handle_sales(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        let fs = &state.financial_system;
        let inventory = fs.get_agent_inventory(&firm.id);

        for (good_id, item) in inventory {
            if item.quantity > 0.1 {
                let ask_price = item.unit_cost.to_f64() * self.target_markup;
                intentions.push(SimIntention::PostGoodToMarket {
                    agent_id: firm.id,
                    good_id,
                    quantity: item.quantity * 0.2,
                    ask_price,
                });
            }
        }
    }

    fn handle_input_purchases(
        &self,
        firm: &Firm,
        state: &SimState,
        intentions: &mut Vec<SimIntention>,
    ) {
        let fs = &state.financial_system;

        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = fs.goods.recipes.get(&recipe_id) {
                let inventory = fs.get_agent_inventory(&firm.id);

                for input in &recipe.inputs {
                    let current_qty = inventory
                        .get(&input.good_id)
                        .map_or(0.0, |item| item.quantity);

                    let target_qty = input.quantity * 2.0;
                    if current_qty < target_qty {
                        let buy_qty = target_qty - current_qty;

                        let max_price = 100.0;

                        intentions.push(SimIntention::PurchaseInputs {
                            agent_id: firm.id,
                            good_id: input.good_id,
                            quantity: buy_qty,
                            max_price,
                        });
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvestmentFirmDecisionModel {
    pub min_liquidity: f64,
    pub quote_qty: f64,
}

impl Default for InvestmentFirmDecisionModel {
    fn default() -> Self {
        Self {
            min_liquidity: 20_000.0,
            quote_qty: 5.0,
        }
    }
}

#[typetag::serde]
impl DecisionModel for InvestmentFirmDecisionModel {
    fn name(&self) -> &str {
        "Investment Firm"
    }

    fn decide(
        &self,
        agent: &dyn std::any::Any,
        state: &SimState,
        rng: &mut dyn RngCore,
    ) -> Vec<SimIntention> {
        let firm = match agent.downcast_ref::<Firm>() {
            Some(f) => f,
            None => return vec![],
        };

        let fs = &state.financial_system;

        if fs.get_liquid_assets(&firm.id) < self.min_liquidity {
            return vec![];
        }
        let liquid_assets = fs.get_liquid_assets(&firm.id);
        let mut intentions = Vec::new();
        self.handle_market_making_treasury(state, firm, &mut intentions, rng);
        self.handle_debt_auctions(state, firm, liquid_assets, &mut intentions, rng);

        intentions
    }
}

impl InvestmentFirmDecisionModel {
    fn handle_market_making_treasury(
        &self,
        state: &SimState,
        firm: &Firm,
        intentions: &mut Vec<SimIntention>,
        rng: &mut dyn RngCore,
    ) {
        let fs = &state.financial_system;
        let current_date = state.current_date;

        if let Some(treasury_ids) = fs.exchange.index.by_bond_type.get(&BondType::Government) {
            for inst_id in treasury_ids {
                if let Some(instrument) = fs.instruments.get(inst_id) {
                    if let InstrumentType::Bond(details) = &instrument.instrument_type {
                        if details.maturity_date <= current_date {
                            continue;
                        }

                        let ytm = pricing::years_to_maturity(current_date, details.maturity_date);

                        if self.should_make_market_for_ytm(ytm) {
                            let (bid_bps, ask_bps) = self.calculate_yield_quotes(ytm, fs, rng);

                            intentions.push(SimIntention::MarketMakeTreasuries {
                                agent_id: firm.id,
                                maturity_date: details.maturity_date,
                                quantity: self.quote_qty,
                                bid_yield_bps: bid_bps,
                                ask_yield_bps: ask_bps,
                            });
                        }
                    }
                }
            }
        }
    }
    fn handle_debt_auctions(
        &self,
        state: &SimState,
        firm: &Firm,
        liquid_assets: f64,
        intentions: &mut Vec<SimIntention>,
        rng: &mut dyn RngCore,
    ) {
        let fs = &state.financial_system;

        let auction_budget = (liquid_assets - self.min_liquidity) * 0.25;
        if auction_budget < 1000.0 {
            return;
        }

        for auction in fs.exchange.open_auctions.values() {
            if auction.status != AuctionStatus::Open {
                continue;
            }

            if let Some(instrument) = fs.instruments.get(&auction.instrument_id) {
                if let InstrumentType::Bond(details) = &instrument.instrument_type {
                    let ytm = pricing::years_to_maturity(state.current_date, details.maturity_date);
                    let (bid_yield_bps, _ask_yield_bps) = self.calculate_yield_quotes(ytm, fs, rng);

                    let bid_price = pricing::bond_price(
                        details.face_value,
                        bps_to_decimal(details.coupon_rate_bps),
                        bps_to_decimal(bid_yield_bps),
                        ytm,
                        details.frequency as usize,
                    );

                    if bid_price <= Money::ZERO {
                        continue;
                    }

                    let quantity_to_bid = (auction_budget / bid_price.to_f64()).floor() as u32;

                    if quantity_to_bid > 0 {
                        intentions.push(SimIntention::BidInDebtAuction {
                            agent_id: firm.id,
                            auction_id: auction.auction_id,
                            quantity: quantity_to_bid,
                            bid_price,
                        });
                    }
                }
            }
        }
    }
    #[inline]
    fn should_make_market_for_ytm(&self, _ytm: f64) -> bool {
        true
    }

    #[inline]
    fn calculate_yield_quotes(
        &self,
        ytm: f64,
        fs: &FinancialSystem,
        rng: &mut dyn RngCore,
    ) -> (BasisPoints, BasisPoints) {
        let policy_bps = fs.central_bank.policy_rate_bps;

        let term_premium = if ytm <= 0.083 {
            2.0
        } else if ytm <= 0.25 {
            7.0
        } else if ytm <= 1.0 {
            12.0
        } else if ytm <= 5.0 {
            35.0
        } else if ytm <= 10.0 {
            50.0
        } else {
            65.0
        };

        let spread_bps = rng.random_range(10.0..25.0);
        let mid = policy_bps
            + Decimal::from_f64(term_premium * rng.random_range(0.9..1.1)).unwrap_or_default();
        let bid = mid + Decimal::from_f64(spread_bps / 2.0).unwrap_or_default();
        let ask = mid - Decimal::from_f64(spread_bps / 2.0).unwrap_or_default();
        (bid, ask)
    }
}
