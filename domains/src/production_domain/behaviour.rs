use rand::prelude::*;
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::any::Any;
use tracing::{debug, info, trace, warn};

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

        //self.handle_hiring(firm, state, &mut intentions);

        self.force_handle_hiring(firm, state, &mut intentions);
        self.handle_wages(firm, state, &mut intentions);
        intentions
    }
}

impl ProductionFirmDecisionModel {
    pub fn calculate_marginal_value_of_labor(&self, firm: &Firm, state: &SimState) -> f64 {
        let fs = &state.financial_system;

        let Some(recipe_id) = firm.recipe.as_ref() else {
            return 0.0;
        };

        let Some(recipe) = fs.goods.recipes.get(recipe_id) else {
            return 0.0;
        };

        let Some(output_good) = recipe.outputs.get(0) else {
            return 0.0;
        };

        let expected_price = state
            .market_view(&MarketId::Goods(output_good.good_id))
            .and_then(|v| v.last_or_mid())
            .unwrap_or_else(|| self.target_markup * self.calculate_unit_cost(recipe, state));

        let current_hours: f64 = firm.employees.values().map(|c| c.hours).sum::<f64>().max(1.0);

        let base_productivity = output_good.quantity / recipe.labour_hours.max(1.0);

        let mpl = base_productivity * firm.productivity / current_hours.sqrt();

        let unit_input_cost = self.calculate_unit_cost(recipe, state);
        let marginal_input_cost = unit_input_cost * mpl;

        let vj = (expected_price * mpl) - marginal_input_cost;

        vj.max(0.0)
    }

    pub fn calculate_unit_cost(&self, recipe: &ProductionRecipe, state: &SimState) -> f64 {
        let total_input_cost: f64 = recipe
            .inputs
            .iter()
            .map(|input| {
                let input_price =
                    state.market_view(&MarketId::Goods(input.good_id)).and_then(|v| v.last_or_mid()).unwrap_or(1.0);

                input.quantity * input_price
            })
            .sum();

        let output_qty = recipe.outputs.get(0).map_or(1.0, |o| o.quantity.max(1.0));

        total_input_cost / output_qty
    }
    pub fn force_handle_hiring(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        let has_open_offer = state
            .financial_system
            .exchange
            .labour_markets
            .values()
            .any(|market| market.job_offers.iter().any(|offer| offer.firm_id == firm.id && offer.quantity > 0));

        if has_open_offer {
            return;
        }

        let avg_daily_sales: f64 = 0.0;
        let desired_employees = ((avg_daily_sales / 100.0).ceil() as usize).max(self.target_employees);

        let current_employees = firm.employees.len();

        if current_employees < desired_employees {
            let open_roles = (desired_employees - current_employees) as u32;

            let posted_wage = self.base_wage;

            intentions.push(SimIntention::Production(ProductionIntention::HireWorkers {
                agent_id: firm.id,
                count: open_roles,
                wage_rate: posted_wage,
                max_wage: posted_wage * 1.1, // Set max_wage slightly higher to accept offers
            }));
        }
    }
    pub fn handle_hiring(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        let has_open_offer = state
            .financial_system
            .exchange
            .labour_markets
            .values()
            .any(|market| market.job_offers.iter().any(|offer| offer.firm_id == firm.id && offer.quantity > 0));

        if has_open_offer {
            return;
        }
        let vj = self.calculate_marginal_value_of_labor(firm, state);

        if vj <= 1e-6 {
            return;
        }

        let avg_daily_sales: f64 = firm.behaviour.per_good.values().map(|m| m.sales_ema).sum();

        let desired_employees = ((avg_daily_sales / 100.0).ceil() as usize).max(self.target_employees);

        let current_employees = firm.employees.len();

        if current_employees < desired_employees {
            let open_roles = (desired_employees - current_employees) as u32;

            let posted_wage = (firm.wage_rate + vj) / 2.0;
            intentions.push(SimIntention::Production(ProductionIntention::HireWorkers {
                agent_id: firm.id,
                count: open_roles,
                wage_rate: posted_wage,
                max_wage: vj,
            }));
        }
    }
    pub fn handle_production(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        let fs = &state.financial_system;
        let firm_name = &firm.name;
        trace!(target: "sim.prod", firm_id = ?firm.id, firm_name, "consider_production");

        if firm.employees.is_empty() {
            debug!(target: "sim.prod", firm_id = ?firm.id, firm_name, reason = "no_employees", "skip_production");
            return;
        }

        let Some(recipe_id) = firm.recipe.clone() else {
            debug!(target: "sim.prod", firm_id = ?firm.id, firm_name, reason = "no_recipe", "skip_production");
            return;
        };
        let Some(recipe) = fs.goods.recipes.get(&recipe_id) else {
            warn!(target: "sim.prod", firm_id = ?firm.id, firm_name, ?recipe_id, reason = "recipe_missing_from_registry", "skip_production");
            return;
        };

        let inventory = fs.get_agent_inventory(&firm.id);

        let max_batches_by_inputs = recipe
            .inputs
            .iter()
            .map(|inp| {
                let have = inventory.get(&inp.good_id).map_or(0.0, |it| it.quantity);
                if inp.quantity <= 0.0 { f64::INFINITY } else { (have / inp.quantity).floor().max(0.0) }
            })
            .fold(f64::INFINITY, f64::min) as u32;

        if max_batches_by_inputs == 0 {
            let missing: Vec<(String, f64, f64)> = recipe
                .inputs
                .iter()
                .filter_map(|inp| {
                    let have = inventory.get(&inp.good_id).map_or(0.0, |it| it.quantity);
                    let need_one = inp.quantity;
                    if need_one > 0.0 && have < need_one {
                        let name = fs
                            .goods
                            .goods
                            .get(&inp.good_id)
                            .map(|g| g.name.clone())
                            .unwrap_or_else(|| format!("{:?}", inp.good_id));
                        Some((name, need_one, have))
                    } else {
                        None
                    }
                })
                .collect();
            if !missing.is_empty() {
                debug!(target: "sim.prod", firm_id=?firm.id, firm_name, ?recipe_id, ?missing, "skip_production_insufficient_inputs");
            }
            return;
        }

        let total_hours: f64 = firm.employees.values().map(|c| c.hours).sum();
        let labour_batches = if recipe.labour_hours > 1e-9 {
            (total_hours / recipe.labour_hours).floor().max(0.0) as u32
        } else {
            u32::MAX
        };

        let capacity_batches = labour_batches.min(max_batches_by_inputs);
        if capacity_batches == 0 {
            debug!(target: "sim.prod", firm_id = ?firm.id, firm_name, reason = "zero_capacity_batches", "skip_production");
            return;
        }

        let gm = &state.financial_system.pricing_feeds.goods.read().unwrap();
        let target_days = 7.0;

        let Some(out) = recipe.outputs.get(0) else {
            return;
        };

        let have = inventory.get(&out.good_id).map_or(0.0, |it| it.quantity);
        let avg_sales = gm.per_good.get(&out.good_id).map(|m| m.avg_daily_sales).unwrap_or(0.0);

        let desired_inventory = target_days * avg_sales;
        let needed_quantity = (desired_inventory - have).max(0.0);

        if needed_quantity <= 1e-6 {
            debug!(target: "sim.prod", firm_id = ?firm.id, firm_name, reason = "inventory_target_met", "skip_production");
            return;
        }

        let out_per_batch = out.quantity.max(1e-9);
        let desired_batches = (needed_quantity / out_per_batch).ceil() as u32;

        let batches = desired_batches.min(capacity_batches);

        if batches > 0 {
            info!(target: "sim.prod",
                "Firm ID: {} | Recipe: {} | Batches: {} (Desired: {}, Capacity: {})",
                firm.id.0.to_string()[0..4].to_string(),
                recipe.name,
                batches,
                desired_batches,
                capacity_batches
            );
            intentions.push(SimIntention::Production(ProductionIntention::Produce {
                agent_id: firm.id,
                recipe_id,
                batches,
            }));
        }
    }

    pub fn handle_wages(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        for (emp_id, c) in &firm.employees {
            if state.current_date >= c.next_pay_date {
                let amount = c.wage_rate * c.hours * (c.pay_interval_days as f64 / 7.0);
                if amount > 0.0 {
                    intentions.push(SimIntention::Transaction(TransactionIntention::PayWages {
                        employer: firm.id,
                        employee: *emp_id,
                        amount,
                    }));
                }
            }
        }
    }

    pub fn consider_financing(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        let fs = &state.financial_system;
        let liquid = fs.get_liquid_assets(&firm.id);

        let wage_bill = firm.employees.values().map(|c| c.wage_rate * c.hours).sum::<f64>() * 2.0;

        let input_budget: f64 = if let Some(recipe_id) = firm.recipe.clone() {
            if let Some(recipe) = fs.goods.recipes.get(&recipe_id) {
                recipe
                    .inputs
                    .iter()
                    .map(|inp| {
                        let px = state
                            .market_view(&MarketId::Goods(inp.good_id))
                            .and_then(|v| v.last_or_mid())
                            .unwrap_or(0.0);
                        px * inp.quantity
                    })
                    .sum()
            } else {
                0.0
            }
        } else {
            0.0
        };

        let planned_need = wage_bill + input_budget;
        if planned_need <= 1.0 {
            return;
        }
        if liquid + 0.01 < planned_need {
            let shortfall = planned_need - liquid;
            let requested = (shortfall * 1.10).max(1_000.0);
            intentions.push(SimIntention::Banking(BankingIntention::RequestLoan {
                agent_id: firm.id,
                bank_id: firm.bank_id,
                amount: requested,
                purpose: LoanPurpose::WorkingCapital,
                collateral: None,
            }));
        }
    }

    pub fn handle_sales(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        let fs = &state.financial_system;
        let goods_config = &state.config.goods;

        let (rev, cogs) = fs
            .balance_sheets
            .get(&firm.id)
            .map(|bs| (bs.income_statement.revenue.to_f64(), bs.income_statement.cost_of_goods_sold.to_f64()))
            .unwrap_or((0.0, 0.0));
        let gm = if rev > 1e-9 { ((rev - cogs) / rev).clamp(-0.99, 0.99) } else { 0.0 };

        let inventory = fs.get_agent_inventory(&firm.id);

        for (good_id, item) in inventory {
            if item.quantity <= 1e-6 {
                continue;
            }

            let metrics = firm.behaviour.per_good.get(&good_id).cloned().unwrap_or_default();
            let avg_daily_sales = metrics.sales_ema.max(1e-6);
            let days_of_cover = (item.quantity / avg_daily_sales).min(365.0);

            let sell_through = metrics.sell_through_ema;
            let target_sell_through = 0.75;
            let sell_through_adjustment = 0.20 * (sell_through - target_sell_through);

            let mut markup = self.target_markup * (1.0 + 0.10 * (0.25 - gm));
            markup *= 1.0 + sell_through_adjustment;
            markup = markup.clamp(1.01, 5.0);

            let unit_cost = item.unit_cost.to_f64();
            let ref_structural = state.financial_system.exchange.fair_price_for_good(&good_id).map(|m| m.to_f64());
            let px_hint = state.market_view(&MarketId::Goods(good_id)).and_then(|v| v.last_or_mid());
            let anchor = if let Some(px) = px_hint.or(ref_structural) {
                0.7 * unit_cost * markup + 0.3 * px
            } else {
                unit_cost * markup
            };
            let ask_price = anchor.max(0.01);

            let doc_target = if firm.behaviour.doc_target_days > 0.0 {
                firm.behaviour.doc_target_days
            } else {
                goods_config.doc_target_days
            };
            let doc_ratio = days_of_cover / doc_target;
            let quantity_factor = (doc_ratio.powf(0.5)).clamp(0.1, 2.0);
            let base_sell_qty = avg_daily_sales.max(1.0);
            let quantity_to_sell = (base_sell_qty * quantity_factor).min(item.quantity);

            if quantity_to_sell > 1e-6 {
                intentions.push(SimIntention::Production(ProductionIntention::PostGoodToMarket {
                    agent_id: firm.id,
                    good_id,
                    quantity: quantity_to_sell,
                    ask_price,
                }));
            }
        }
    }

    pub fn handle_separations(
        &self, firm: &Firm, _state: &SimState, intentions: &mut Vec<SimIntention>, rng: &mut dyn RngCore,
    ) {
        let quit_rate = 0.005;
        let mut departures = Vec::new();
        for emp in firm.get_employees() {
            if rng.random::<f64>() < quit_rate {
                departures.push(emp);
            }
        }
        if !departures.is_empty() {
            intentions.push(SimIntention::Production(ProductionIntention::FireWorkers {
                agent_id: firm.id,
                employee_ids: departures,
            }));
        }
    }

    pub fn handle_input_purchases(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        let fs = &state.financial_system;
        let Some(recipe_id) = firm.recipe.clone() else {
            return;
        };
        let Some(recipe) = fs.goods.recipes.get(&recipe_id) else {
            return;
        };

        let inventory = fs.get_agent_inventory(&firm.id);
        let total_hours: f64 = firm.employees.values().map(|c| c.hours).sum();
        let desired_batches =
            if recipe.labour_hours > 1e-9 { (total_hours / recipe.labour_hours).ceil().max(1.0) } else { 1.0 } as f64;

        for input in &recipe.inputs {
            let have = inventory.get(&input.good_id).map_or(0.0, |it| it.quantity);
            let target = input.quantity * desired_batches;
            if have + 1e-9 >= target {
                continue;
            }
            let buy_qty = (target - have).max(0.0);

            let ref_struct = state.financial_system.exchange.fair_price_for_good(&input.good_id).map(|m| m.to_f64());
            let inv_cost = inventory.get(&input.good_id).map(|it| it.unit_cost.to_f64());
            let ref_px = inv_cost.or(ref_struct).unwrap_or(0.0);
            let max_price = (ref_px * 1.05).max(0.0);
            intentions.push(SimIntention::Production(ProductionIntention::PurchaseInputs {
                agent_id: firm.id,
                good_id: input.good_id,
                quantity: buy_qty,
                max_price,
            }));
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

    fn decide(&self, agent: &dyn std::any::Any, state: &SimState, rng: &mut dyn RngCore) -> Vec<SimIntention> {
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
        &self, state: &SimState, firm: &Firm, intentions: &mut Vec<SimIntention>, rng: &mut dyn RngCore,
    ) {
        let fs = &state.financial_system;
        let current_date = state.current_date;

        if let Some(treasury_ids) = fs.exchange.index.by_bond_type.get(&BondType::Government) {
            for inst_id in treasury_ids {
                if let Some(instrument) = fs.instruments.get(inst_id) {
                    if !instrument.should_create_order_book() {
                        continue;
                    }
                    if let InstrumentType::Debt(DebtInstrument::Bond(details)) = &instrument.instrument_type {
                        if details.maturity_date <= current_date {
                            continue;
                        }

                        let tenor_years = details.remaining_tenor_years(current_date);
                        if self.should_make_market_for_ytm(tenor_years) {
                            let (bid_bps, ask_bps) =
                                quote_treasury_yields(tenor_years, fs.central_bank.policy_rate_bps, rng);
                            intentions.push(SimIntention::Banking(BankingIntention::MarketMakeTreasuries {
                                agent_id: firm.id,
                                maturity_date: details.maturity_date,
                                quantity: self.quote_qty,
                                bid_yield_bps: bid_bps,
                                ask_yield_bps: ask_bps,
                            }));
                        }
                    }
                }
            }
        }
    }

    fn handle_debt_auctions(
        &self, state: &SimState, firm: &Firm, liquid_assets: f64, intentions: &mut Vec<SimIntention>,
        rng: &mut dyn RngCore,
    ) {
        let fs = &state.financial_system;
        let current_date = state.current_date;
        let auction_budget = liquid_assets * 0.05;

        for auction in fs.exchange.open_auctions.values() {
            if let Some(instrument) = fs.instruments.get(&auction.instrument_id) {
                if let Some(details) = instrument.instrument_type.as_bond() {
                    let tenor_years = details.remaining_tenor_years(current_date);
                    let (bid_yield_bps, _ask_yield_bps) =
                        quote_treasury_yields(tenor_years, fs.central_bank.policy_rate_bps, rng);

                    let bid_price = auction_bid_price(
                        details,
                        bid_yield_bps,
                        &auction.instrument_id,
                        &fs.pricing_feeds,
                        current_date,
                    );

                    if bid_price.to_f64() < 1.0 {
                        continue;
                    }
                    let quantity_to_bid = (auction_budget / bid_price.to_f64()).floor() as u32;

                    if quantity_to_bid > 0 {
                        intentions.push(SimIntention::Fiscal(FiscalIntention::BidInDebtAuction {
                            agent_id: firm.id,
                            auction_id: auction.auction_id,
                            quantity: quantity_to_bid,
                            bid_price,
                        }));
                    }
                }
            }
        }
    }
    #[inline]
    fn should_make_market_for_ytm(&self, _ytm: f64) -> bool {
        true
    }
}
