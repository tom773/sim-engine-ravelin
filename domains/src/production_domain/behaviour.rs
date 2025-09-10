use rand::prelude::*;
use rust_decimal::prelude::*;
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
    fn decide(&self, agent: &dyn Any, state: &SimState, rng: &mut dyn RngCore) -> Vec<SimIntention> {
        let firm = match agent.downcast_ref::<Firm>() {
            Some(f) => f,
            None => return vec![],
        };

        let mut intentions = Vec::new();

        self.handle_hiring(firm, state, &mut intentions);
        self.handle_separations(firm, state, &mut intentions, rng);
        self.handle_production(firm, state, &mut intentions);
        self.handle_wages(firm, state, &mut intentions);
        self.handle_sales(firm, state, &mut intentions);
        self.handle_input_purchases(firm, state, &mut intentions);
        self.consider_financing(firm, state, &mut intentions);
        intentions
    }
}

impl ProductionFirmDecisionModel {
    fn handle_hiring(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        let current = firm.employees.len();
        if current >= self.target_employees {
            return;
        }
        if state.financial_system.find_general_labour_market().is_none() {
            return;
        }
        let open_roles = (self.target_employees - current) as u32;
        intentions.push(SimIntention::Production(ProductionIntention::HireWorkers {
            agent_id: firm.id,
            count: open_roles,
            wage_rate: self.base_wage,
        }));
    }

    fn handle_production(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
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

            warn!(target: "sim.prod", firm_id = ?firm.id, firm_name, missing_inputs = ?missing, "skip_production_inputs_unavailable");
            return;
        }

        let total_hours: f64 = firm.employees.values().map(|c| c.hours).sum();
        let labour_batches =
            if recipe.labour_hours > 1e-9 { (total_hours / recipe.labour_hours).floor().max(0.0) as u32 } else { 1 };
        let capacity_batches = labour_batches.min(max_batches_by_inputs).max(1);

        debug!(target: "sim.prod",
            firm_id=?firm.id, firm_name,
            total_hours, recipe_labour_hours = recipe.labour_hours,
            labour_batches, max_batches_by_inputs, capacity_batches,
            "capacity_computed"
        );

        let input_cost_est: f64 = recipe
            .inputs
            .iter()
            .map(|inp| {
                let unit = inventory
                    .get(&inp.good_id)
                    .map(|it| it.unit_cost.to_f64())
                    .or_else(|| state.market_view(&MarketId::Goods(inp.good_id)).and_then(|v| v.last_or_mid()))
                    .or_else(|| state.financial_system.exchange.fair_price_for_good(&inp.good_id).map(|m| m.to_f64()))
                    .unwrap_or(0.0);
                unit * inp.quantity
            })
            .sum();

        let avg_output_px: f64 = {
            let mut sum = 0.0;
            let mut n = 0.0;
            for out in &recipe.outputs {
                let px_market = state.market_view(&MarketId::Goods(out.good_id)).and_then(|v| v.last_or_mid());
                let px_struct = state.financial_system.exchange.fair_price_for_good(&out.good_id).map(|m| m.to_f64());
                if let Some(px) = px_market.or(px_struct) {
                    sum += px;
                    n += 1.0;
                }
            }
            if n > 0.0 { sum / n } else { 0.0 }
        };
        let labour_cost_est = recipe.labour_hours * firm.wage_rate;
        let total_out_qty: f64 = recipe.outputs.iter().map(|o| o.quantity).sum();
        let unit_cost_est = if total_out_qty > 0.0 { (input_cost_est + labour_cost_est) / total_out_qty } else { 0.0 };

        let low_inventory = recipe.outputs.iter().any(|out| {
            let have = inventory.get(&out.good_id).map_or(0.0, |it| it.quantity);
            have < out.quantity
        });
        let expected_margin_ok = avg_output_px > unit_cost_est * 0.99;

        debug!(target: "sim.prod",
            firm_id=?firm.id, firm_name,
            input_cost_est, labour_cost_est, unit_cost_est, avg_output_px,
            low_inventory, expected_margin_ok,
            "pricing_signal"
        );

        if low_inventory || expected_margin_ok {
            let batches = capacity_batches.max(1).min(4);
            info!(target: "sim.prod",
                firm_id=?firm.id, firm_name, ?recipe_id, batches,
                reason = if low_inventory { "replenish_inventory" } else { "positive_expected_margin" },
                "produce_intention"
            );
            intentions.push(SimIntention::Production(ProductionIntention::Produce {
                agent_id: firm.id,
                recipe_id,
                batches,
            }));
        } else {
            debug!(target: "sim.prod",
                firm_id=?firm.id, firm_name, ?recipe_id,
                reason = "negative_expected_margin",
                "skip_production"
            );
        }
    }

    fn handle_wages(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
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

    fn consider_financing(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
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

    fn handle_sales(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
        let fs = &state.financial_system;

        let (rev, cogs) = fs
            .balance_sheets
            .get(&firm.id)
            .map(|bs| (bs.income_statement.revenue.to_f64(), bs.income_statement.cost_of_goods_sold.to_f64()))
            .unwrap_or((0.0, 0.0));
        let gm = if rev > 1e-9 { ((rev - cogs) / rev).clamp(-0.99, 0.99) } else { 0.0 };

        let target_gm = 0.25;
        let learn = 0.10;
        let theoretical_markup = if target_gm < 0.99 { 1.0 / (1.0 - target_gm) } else { self.target_markup };
        let mut markup = self.target_markup * (1.0 + learn * (target_gm - gm));
        markup = markup.clamp(1.0, theoretical_markup * 1.5);

        let inventory = fs.get_agent_inventory(&firm.id);
        for (good_id, item) in inventory {
            if item.quantity <= 1e-6 {
                continue;
            }
            let unit_cost = item.unit_cost.to_f64();

            let ref_structural = state.financial_system.exchange.fair_price_for_good(&good_id).map(|m| m.to_f64());
            let px_hint = state.market_view(&MarketId::Goods(good_id)).and_then(|v| v.last_or_mid());
            let anchor = if let Some(px_hint) = px_hint.or(ref_structural) {
                0.7 * unit_cost * (1.0 + markup) + 0.3 * px_hint
            } else {
                unit_cost * (1.0 + markup)
            };

            let ask_price = anchor.max(0.0);

            let frac = (0.20 + 0.30 * (gm - target_gm)).clamp(0.05, 0.50);
            let qty = (item.quantity * frac).max(1.0).min(item.quantity);

            intentions.push(SimIntention::Production(ProductionIntention::PostGoodToMarket {
                agent_id: firm.id,
                good_id,
                quantity: qty,
                ask_price,
            }));
        }
    }

    fn handle_separations(
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

    fn handle_input_purchases(&self, firm: &Firm, state: &SimState, intentions: &mut Vec<SimIntention>) {
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
            let max_price = (ref_px * 1.05).max(0.0); // small premium to clear
            tracing::debug!(
                target: "sim.prod",
                firm_id = ?firm.id, firm_name = ?firm.name,
                ?input.good_id, have, target, buy_qty, ref_px, max_price,
                "consider_input_purchase"
            );
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

                        let ytm = pricing::years_to_maturity(current_date, details.maturity_date);

                        if self.should_make_market_for_ytm(ytm) {
                            let (bid_bps, ask_bps) = self.calculate_yield_quotes(ytm, fs, rng);

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

        let auction_budget = (liquid_assets - self.min_liquidity) * 0.25;
        if auction_budget < 1000.0 {
            return;
        }

        for auction in fs.exchange.open_auctions.values() {
            if auction.status != AuctionStatus::Open {
                continue;
            }

            if let Some(instrument) = fs.instruments.get(&auction.instrument_id) {
                if let InstrumentType::Debt(DebtInstrument::Bond(details)) = &instrument.instrument_type {
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

    #[inline]
    fn calculate_yield_quotes(
        &self, ytm: f64, fs: &FinancialSystem, rng: &mut dyn RngCore,
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
        let mid = policy_bps + Decimal::from_f64(term_premium * rng.random_range(0.9..1.1)).unwrap_or_default();
        let bid = mid + Decimal::from_f64(spread_bps / 2.0).unwrap_or_default();
        let ask = mid - Decimal::from_f64(spread_bps / 2.0).unwrap_or_default();
        (bid, ask)
    }
}
