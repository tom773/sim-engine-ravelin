use rand::prelude::*;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::any::Any;
use chrono::Datelike;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionFirmDecisionModel {
    pub target_markup: f64,
    pub base_wage: f64,
    pub target_employees: usize,
}

impl Default for ProductionFirmDecisionModel {
    fn default() -> Self {
        Self { target_markup: 1.25, base_wage: 25.0, target_employees: 3 }
    }
}

#[typetag::serde]
impl DecisionModel for ProductionFirmDecisionModel {
    fn name(&self) -> &str {
        "Production Firm"
    }
    fn decide(&self, agent: &dyn Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let firm = match agent.downcast_ref::<Firm>() {
            Some(f) => f,
            None => return vec![],
        };

        let mut intentions = Vec::new();

        self.handle_hiring(firm, &mut intentions);
        self.handle_production(firm, state, &mut intentions);
        self.handle_wages(firm, state, &mut intentions);
        self.handle_sales(firm, state, &mut intentions);
        self.handle_input_purchases(firm, state, &mut intentions);

        intentions
    }
}

impl ProductionFirmDecisionModel {
    fn handle_hiring(&self, firm: &Firm, intentions: &mut Vec<SimIntention>) {
        let current_employees = firm.employees.len();
        if current_employees < self.target_employees {
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

        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = state.financial_system.goods.get_recipe(&recipe_id) {
                if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id).and_then(|bs| bs.get_inventory())
                {
                    let can_produce = recipe.inputs.iter().all(|input| {
                        inventory.get(&input.good_id).map_or(false, |item| item.quantity >= input.quantity)
                    });

                    if can_produce {
                        intentions.push(SimIntention::Produce { agent_id: firm.id, recipe_id, batches: 1 });
                    }
                }
            }
        }
    }

    fn handle_wages(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        println!("[WAGE PAYMENT] {} has emplyers {:#?}", firm.name, firm.employees);
        if (state.current_date.day() == 3 || state.current_date.day() == 17) == true {
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
        if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id).and_then(|bs| bs.get_inventory()) {
            for (good_id, item) in inventory {
                if item.quantity > 0.1 {
                    intentions.push(SimIntention::SellInventory {
                        agent_id: firm.id,
                        good_id: *good_id,
                        quantity: item.quantity,
                        desired_markup: self.target_markup,
                    });
                }
            }
        }
    }

    fn handle_input_purchases(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        if let Some(recipe_id) = firm.recipe {
            if let Some(recipe) = state.financial_system.goods.get_recipe(&recipe_id) {
                if let Some(inventory) = state.financial_system.get_bs_by_id(&firm.id).and_then(|bs| bs.get_inventory())
                {
                    for input in &recipe.inputs {
                        let current_qty = inventory.get(&input.good_id).map_or(0.0, |item| item.quantity);

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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvestmentFirmDecisionModel {
    pub min_liquidity: f64,
    pub quote_qty: f64,
}

impl Default for InvestmentFirmDecisionModel {
    fn default() -> Self {
        Self { min_liquidity: 20_000.0, quote_qty: 5.0 }
    }
}

#[typetag::serde]
impl DecisionModel for InvestmentFirmDecisionModel {
    fn name(&self) -> &str {
        "Investment Firm"
    }

    fn decide(&self, agent: &dyn std::any::Any, state: &SimState, _rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let firm = match agent.downcast_ref::<Firm>() {
            Some(f) => f,
            None => return vec![],
        };

        let fs = &state.financial_system;

        if fs.get_liquid_assets(&firm.id) < self.min_liquidity {
            return vec![];
        }

        let mut intentions = Vec::new();
        self.handle_market_making_treasury(fs, firm, &mut intentions);
        intentions
    }
}

impl InvestmentFirmDecisionModel {
    fn handle_market_making_treasury(&self, fs: &FinancialSystem, firm: &Firm, intentions: &mut Vec<SimIntention>) {
        for (market_id, _market) in &fs.exchange.financial_markets {
            if let FinancialMarketId::Treasury { tenor } = market_id {
                if self.should_make_market_for_tenor(*tenor) {
                    let (bid_bps, ask_bps) = self.calculate_yield_quotes(*tenor, fs);
                    intentions.push(SimIntention::MarketMakeTreasuries {
                        agent_id: firm.id,
                        tenor: *tenor,
                        quantity: self.quote_qty,
                        bid_yield_bps: bid_bps,
                        ask_yield_bps: ask_bps,
                    });
                }
            }
        }
    }
    #[inline]
    fn should_make_market_for_tenor(&self, _tenor: Tenor) -> bool {
        true
    }

    #[inline]
    fn calculate_yield_quotes(&self, tenor: Tenor, fs: &FinancialSystem) -> (BasisPoints, BasisPoints) {
        let policy_bps = fs.central_bank.policy_rate_bps;
        let term_premium = match tenor {
            Tenor::T2Y => 15.0,
            Tenor::T5Y => 35.0,
            Tenor::T10Y => 50.0,
            Tenor::T30Y => 65.0, // keep 30Y fatter
        };

        let spread_bps = rand::random_range(10.0..25.0);
        let mid = policy_bps + term_premium;
        let bid = mid + spread_bps / 2.0;
        let ask = mid - spread_bps / 2.0;
        (bid, ask)
    }
}
