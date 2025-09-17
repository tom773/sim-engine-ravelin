use crate::*;
use domains::prelude::*;
use rand::distr::{Distribution as RandDist, weighted::WeightedIndex};
use rand::{Rng, SeedableRng, rngs::StdRng};
use rand_distr::{LogNormal, Normal, Pareto};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Deserialize, Clone)]
pub struct ClampRange {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Distribution {
    #[serde(rename = "normal")]
    Normal { mean: f64, stdev: f64 },
    #[serde(rename = "log_normal")]
    LogNormal { mu: f64, sigma: f64 },
    #[serde(rename = "pareto")]
    Pareto { xm: f64, alpha: f64 },
    #[serde(rename = "pct_of_income")]
    PctOfIncome { pct: f64 },
}

impl Distribution {
    pub fn sample(&self, rng: &mut impl Rng) -> f64 {
        match self {
            Distribution::Normal { mean, stdev } => {
                let d = Normal::new(*mean, stdev.max(1e-9)).unwrap();
                d.sample(rng)
            }
            Distribution::LogNormal { mu, sigma } => {
                let d = LogNormal::new(*mu, sigma.max(1e-9)).unwrap();
                d.sample(rng)
            }
            Distribution::Pareto { xm, alpha } => {
                let d = Pareto::new(xm.max(1e-9), alpha.max(1e-9)).unwrap();
                d.sample(rng)
            }
            Distribution::PctOfIncome { pct } => *pct,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DistributionWithClamp {
    pub dist: Distribution,
    #[serde(default)]
    pub clamp: Option<ClampRange>,
}
impl DistributionWithClamp {
    pub fn sample(&self, rng: &mut impl Rng) -> f64 {
        let v = self.dist.sample(rng);
        if let Some(c) = &self.clamp {
            let lo = c.min.unwrap_or(f64::NEG_INFINITY);
            let hi = c.max.unwrap_or(f64::INFINITY);
            v.max(lo).min(hi)
        } else {
            v
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Scenario {
    pub name: String,
    pub description: String,
    pub config: ScenarioConfig,
    goods: Vec<GoodConfig>,
    recipes: Vec<RecipeConfig>,
    banks: Vec<BankConfig>,
    firms: Vec<FirmConfig>,
    consumers: Vec<ConsumerConfig>,
    #[serde(default)]
    consumer_groups: Vec<ConsumerGroup>,
    #[serde(default)]
    firm_groups: Vec<FirmGroup>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScenarioConfig {
    pub iterations: u32,
    #[serde(default = "default_seed")]
    pub seed: u64,
}
fn default_seed() -> u64 {
    42
}

#[derive(Debug, Deserialize, Clone)]
pub struct GoodConfig {
    pub id: String,
    pub name: String,
    pub unit: String,
    pub category: String,
    pub cpi_weight: f64,
}
#[derive(Debug, Deserialize, Clone)]
pub struct RecipeIOConfig {
    pub good_id: String,
    pub quantity: f64,
}
#[derive(Debug, Deserialize, Clone)]
pub struct RecipeConfig {
    pub id: String,
    pub name: String,
    pub inputs: Vec<RecipeIOConfig>,
    pub outputs: Vec<RecipeIOConfig>,
    pub labour_hours: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ReservesConfig {
    #[serde(rename = "ratio_of_deposits")]
    RatioOfDeposits { ratio: f64, noise: f64 },
}

#[derive(Debug, Deserialize, Clone)]
pub struct BankConfig {
    pub id: String,
    pub name: String,
    pub initial_assets: Vec<AssetConfig>,
    pub initial_liabilities: Vec<LiabilityConfig>,
    pub reserves: Option<ReservesConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum DecisionModelConfig {
    ProductionFirm,
    InvestmentFirm { min_liquidity: f64, quote_qty: f64 },
}
impl DecisionModelConfig {
    pub fn into_decision_model(self) -> Box<dyn DecisionModel> {
        match self {
            DecisionModelConfig::ProductionFirm => Box::new(ProductionFirmDecisionModel::default()),
            DecisionModelConfig::InvestmentFirm { min_liquidity, quote_qty } => {
                Box::new(InvestmentFirmDecisionModel { min_liquidity, quote_qty })
            }
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct FirmConfig {
    pub id: String,
    pub name: String,
    pub bank_id: String,
    #[serde(default)]
    pub recipe_name: Option<String>,
    #[serde(default)]
    pub desired_markup: Option<f64>,
    #[serde(default)]
    pub initial_assets: Vec<AssetConfig>,
    #[serde(default)]
    pub initial_liabilities: Vec<LiabilityConfig>,
    pub decision_model: DecisionModelConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConsumerConfig {
    pub id: String,
    pub bank_id: String,
    pub income: f64,
    pub initial_assets: Vec<AssetConfig>,
    pub initial_liabilities: Vec<LiabilityConfig>,
    pub consumption_basket: HashMap<String, f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConsumerGroup {
    pub id_prefix: String,
    pub count: u32,
    pub bank_mix: HashMap<String, f64>,
    pub income: Distribution,
    pub deposit: Distribution,
    pub consumption_basket: HashMap<String, f64>,
    #[serde(default)]
    pub archetype: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InventorySpec {
    pub good_slug: String,
    pub quantity: Distribution,
    pub unit_cost: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FirmGroup {
    pub id_prefix: String,
    pub count: u32,
    pub bank_mix: HashMap<String, f64>,
    pub recipe_name: Option<String>,
    pub deposit: Distribution,
    pub inventory: Option<InventorySpec>,
    pub desired_markup: Option<DistributionWithClamp>,
    pub decision_model: DecisionModelConfig,
    #[serde(default)]
    pub archetype: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum AssetConfig {
    Cash { amount: f64 },
    Deposit { bank_id: String, amount: f64 },
    Reserves { amount: f64 },
    Bond { tenor: String, quantity: u32 },
    Inventory { good_slug: String, quantity: f64, unit_cost: f64 },
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum LiabilityConfig {
    Deposit {
        creditor_id: String,
        amount: f64,
        #[serde(default)]
        rate_bps: BasisPoints,
    },
    Loan {
        creditor_id: String,
        principal: f64,
        #[serde(default)]
        rate_bps: BasisPoints,
        #[serde(default)]
        maturity_days: u32,
    },
}

impl Scenario {
    pub fn from_toml_str(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str::<Scenario>(s)
    }

    fn expand_groups(&self, rng: &mut StdRng) -> (Vec<ConsumerConfig>, Vec<FirmConfig>) {
        let mut generated_consumers = Vec::new();
        for group in &self.consumer_groups {
            let bank_ids: Vec<String> = group.bank_mix.keys().cloned().collect();
            let weights: Vec<f64> = group.bank_mix.values().copied().collect();
            let chooser = WeightedIndex::new(&weights).unwrap();

            for i in 0..group.count {
                let bank_choice = &bank_ids[chooser.sample(rng)];
                let income = group.income.sample(rng);
                let deposit_ratio = match &group.deposit {
                    Distribution::PctOfIncome { pct } => *pct,
                    d => d.sample(rng),
                };
                let deposit_amt = (income * deposit_ratio).max(0.0);

                generated_consumers.push(ConsumerConfig {
                    id: format!("{}{:04}", group.id_prefix, i),
                    bank_id: bank_choice.clone(),
                    income,
                    initial_assets: vec![AssetConfig::Deposit { bank_id: bank_choice.clone(), amount: deposit_amt }],
                    initial_liabilities: vec![],
                    consumption_basket: group.consumption_basket.clone(),
                });
            }
        }

        let mut generated_firms = Vec::new();
        for group in &self.firm_groups {
            let bank_ids: Vec<String> = group.bank_mix.keys().cloned().collect();
            let weights: Vec<f64> = group.bank_mix.values().copied().collect();
            let chooser = WeightedIndex::new(&weights).unwrap();

            for i in 0..group.count {
                let bank_choice = &bank_ids[chooser.sample(rng)];
                let deposit_amt = group.deposit.sample(rng);
                let mut initial_assets =
                    vec![AssetConfig::Deposit { bank_id: bank_choice.clone(), amount: deposit_amt }];

                if let Some(inv) = &group.inventory {
                    initial_assets.push(AssetConfig::Inventory {
                        good_slug: inv.good_slug.clone(),
                        quantity: inv.quantity.sample(rng),
                        unit_cost: inv.unit_cost,
                    });
                }

                let desired_markup = group.desired_markup.as_ref().map(|dm| dm.sample(rng));

                generated_firms.push(FirmConfig {
                    id: format!("{}{:04}", group.id_prefix, i),
                    name: format!("{} {}", group.id_prefix, i),
                    bank_id: bank_choice.clone(),
                    recipe_name: group.recipe_name.clone(),
                    desired_markup,
                    initial_assets,
                    initial_liabilities: vec![],
                    decision_model: group.decision_model.clone(),
                });
            }
        }
        (generated_consumers, generated_firms)
    }

    pub fn initialize_engine(&self) -> SimulationEngine {
        let mut state = SimState::default();
        state.config.iterations = self.config.iterations;
        let cb_id = state.financial_system.central_bank.id;
        let gov_id = state.financial_system.government.id;
        let mut rng = StdRng::seed_from_u64(self.config.seed);

        let mut good_ids: HashMap<String, GoodId> = HashMap::new();
        for g in &self.goods {
            let gid = GoodId(Uuid::new_v4());
            good_ids.insert(g.id.clone(), gid);
            state.financial_system.goods.goods.insert(
                gid,
                Good {
                    id: gid,
                    name: g.name.clone(),
                    unit: g.unit.clone(),
                    category: match g.category.as_str() {
                        "RawMaterial" => GoodCategory::RawMaterial,
                        "IntermediateGood" => GoodCategory::IntermediateGood,
                        "CapitalGood" => GoodCategory::CapitalGood,
                        _ => GoodCategory::ConsumerGood,
                    },
                    cpi_weight: g.cpi_weight,
                },
            );
            state.financial_system.exchange.ensure_goods_market(gid, &g.name);
        }
        for r in &self.recipes {
            let rid = RecipeId(r.id.clone());
            let inputs = r
                .inputs
                .iter()
                .map(|x| RecipeIO { good_id: *good_ids.get(&x.good_id).expect("bad good_id"), quantity: x.quantity })
                .collect();
            let outputs = r
                .outputs
                .iter()
                .map(|x| RecipeIO { good_id: *good_ids.get(&x.good_id).expect("bad good_id"), quantity: x.quantity })
                .collect();
            state.financial_system.goods.recipes.insert(
                rid.clone(),
                ProductionRecipe { id: rid, name: r.name.clone(), inputs, outputs, labour_hours: r.labour_hours },
            );
        }

        let (mut more_consumers, mut more_firms) = self.expand_groups(&mut rng);
        let mut all_consumers = self.consumers.clone();
        let mut all_firms = self.firms.clone();
        all_consumers.append(&mut more_consumers);
        all_firms.append(&mut more_firms);

        let mut agent_ids: HashMap<String, AgentId> = HashMap::new();
        let mut factory = factory::AgentFactory::new(&mut state, &mut rng);

        factory.initialize_treasury_general_account().expect("TGA init failed");
        let mut bond_instruments_by_tenor = std::collections::HashMap::new();

        // Create banks without seed_sales
        for b in &self.banks {
            let bank = factory.create_bank(b, cb_id, &mut bond_instruments_by_tenor);
            agent_ids.insert(b.id.clone(), bank.id);
        }

        // Create consumers without collecting seed_sales
        for (i, c) in all_consumers.iter().enumerate() {
            let bank_id = *agent_ids.get(&c.bank_id).expect("unknown bank id");
            let consumer =
                factory.create_consumer(c, bank_id, cb_id, &agent_ids, i, &mut bond_instruments_by_tenor);
            agent_ids.insert(c.id.clone(), consumer.id);
        }

        // Create firms without collecting seed_sales
        for f in &all_firms {
            let bank_id = *agent_ids.get(&f.bank_id).expect("unknown bank id");
            let firm = factory.create_firm(f, bank_id, cb_id, &agent_ids, &mut bond_instruments_by_tenor);
            for a in &f.initial_assets {
                if let AssetConfig::Inventory { good_slug, quantity, unit_cost } = a {
                    let gid = *good_ids.get(good_slug).expect("inventory good unknown");
                    factory.state.financial_system.add_to_inventory(
                        &firm.id,
                        &gid,
                        *quantity,
                        Money::from_f64(*unit_cost).unwrap_or(Money::ZERO),
                    )
                }
            }
            agent_ids.insert(f.id.clone(), firm.id);
        }

        // Add reserves based on deposits
        for b_config in &self.banks {
            if let Some(bank_id) = agent_ids.get(&b_config.id) {
                if let Some(reserves_config) = &b_config.reserves {
                    let total_deposits = factory.state.financial_system.get_bank_deposits(bank_id);
                    let mut rng_cl = rand::rng();
                    let reserves_to_create = match reserves_config {
                        ReservesConfig::RatioOfDeposits { ratio, noise } => {
                            let noisy_ratio = ratio + (rng_cl.random::<f64>() - 0.5) * 2.0 * noise;
                            total_deposits * noisy_ratio.max(0.0)
                        }
                    };

                    if reserves_to_create > 0.0 {
                        factory.seed_reserves_to_bank_from_tga(*bank_id, reserves_to_create).unwrap_or_else(|e| {
                            tracing::warn!("TGA->bank reserves seed failed: {} (falling back to CB creation)", e);
                            factory
                                .seed_reserves_via_cb_creation(*bank_id, reserves_to_create)
                                .expect("Final reserve creation failed");
                        });
                    }
                }
            }
        }

        // NOTE: The entire bond issuance and settlement block has been removed
        // Bonds are now directly credited during asset creation

        // Create equity for government if needed
        {
            let gbs = factory.state.financial_system.balance_sheets.get(&gov_id).expect("gov BS");
            let tot_a: Money = gbs.assets.values().map(|p| p.book_value_per_unit * p.quantity).sum();
            let tot_l: Money = gbs.liabilities.values().map(|p| p.book_value_per_unit * p.quantity).sum();
            let nw = tot_a - tot_l;
            if nw.to_f64().abs() > 1e-6 {
                let eq = Instrument {
                    id: InstrumentId(Uuid::new_v4()),
                    instrument_type: InstrumentType::Equity(EquityDetails { issuer: gov_id, outstanding_shares: 1 }),
                    instrument_market: InstrumentMarket::CapitalMarket(CapitalMarketSegment::Equity),
                    listability: Listability::Unlisted,
                };
                let id = eq.id;
                factory.state.financial_system.instruments.insert(id, eq);
                let gbs_mut = factory.state.financial_system.balance_sheets.get_mut(&gov_id).unwrap();
                gbs_mut
                    .liabilities
                    .insert(id, Position { quantity: 1.0, book_value_per_unit: nw, cost_basis_per_unit: nw });
            }
        }

        factory.state.financial_system.exchange.ensure_labour_market(LabourMarketId(Uuid::new_v4()), "General");
        factory.state.financial_system.attach_default_pricing_feeds(factory.state.current_date);

        for (iid, inst) in factory.state.financial_system.instruments.clone() {
            factory.state.financial_system.exchange.ensure_listed(iid, &inst);
        }

        let mut engine = SimulationEngine::new_with_scheduler(std::mem::take(&mut factory.state));
        let mut decisions: HashMap<AgentId, Box<dyn DecisionModel>> = HashMap::new();
        decisions.insert(gov_id, Box::new(BasicGovernmentDecisionModel::default()));
        for f in &all_firms {
            if let Some(id) = agent_ids.get(&f.id) {
                decisions.insert(*id, f.decision_model.clone().into_decision_model());
            }
        }
        for (bank_id, _bank) in engine.state.agents.banks.iter() {
            decisions.insert(*bank_id, Box::new(BasicBankDecisionModel::default()));
        }

        for c in &all_consumers {
            if let Some(cid) = agent_ids.get(&c.id) {
                let basket: std::collections::HashMap<GoodId, f64> = c
                    .consumption_basket
                    .iter()
                    .filter_map(|(slug, w)| good_ids.get(slug).copied().map(|gid| (gid, *w)))
                    .collect();

                decisions.insert(*cid, Box::new(SimpleConsumerDecisionModel { mpc: 0.7, consumption_basket: basket }));
            }
        }
        engine.decision_models = decisions;
        engine
    }
}