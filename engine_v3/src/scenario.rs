use crate::*;
use domains::prelude::*;
use rand::distr::{Distribution as RandDist, weighted::WeightedIndex};
use rand::prelude::*;
use rand_distr::{LogNormal, Normal, Pareto, Uniform};
use serde::Deserialize;
use sim_core::prelude::Money;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Distribution {
    Uniform { min: f64, max: f64 },
    Normal { mean: f64, stdev: f64 },
    LogNormal { mu: f64, sigma: f64 },
    Pareto { xm: f64, alpha: f64 },
    Fixed { value: f64 },
    PctOfIncome { pct: f64 },
}

impl Distribution {
    pub fn sample(&self, rng: &mut impl Rng) -> f64 {
        let value = match self {
            Distribution::Uniform { min, max } => Uniform::new(*min, *max).unwrap().sample(rng),
            Distribution::Normal { mean, stdev } => Normal::new(*mean, *stdev).unwrap().sample(rng),
            Distribution::LogNormal { mu, sigma } => LogNormal::new(*mu, *sigma).unwrap().sample(rng), // No .exp()!
            Distribution::Pareto { xm, alpha } => Pareto::new(*xm, *alpha).unwrap().sample(rng),
            Distribution::Fixed { value } => *value,
            Distribution::PctOfIncome { pct } => *pct,
        };

        if !value.is_finite() {
            eprintln!("Warning: Distribution {:?} produced non-finite value, defaulting to 0", self);
            0.0
        } else {
            value
        }
    }

    pub fn sample_clamped(&self, rng: &mut impl Rng, min: f64, max: f64) -> f64 {
        self.sample(rng).max(min).min(max)
    }
}
#[derive(Debug, Deserialize, Clone)]
pub struct DistributionWithClamp {
    #[serde(flatten)]
    pub dist: Distribution,
    pub clamp: Option<ClampRange>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClampRange {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl DistributionWithClamp {
    pub fn sample(&self, rng: &mut impl Rng) -> f64 {
        let value = self.dist.sample(rng);
        match &self.clamp {
            Some(c) => {
                let min = c.min.unwrap_or(f64::NEG_INFINITY);
                let max = c.max.unwrap_or(f64::INFINITY);
                value.max(min).min(max)
            }
            None => value,
        }
    }
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
pub struct InventorySpec {
    pub good_slug: String,
    pub quantity: Distribution,
    pub unit_cost: f64,
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
pub struct BankConfig {
    pub id: String,
    pub name: String,
    pub initial_assets: Vec<AssetConfig>,
    pub initial_liabilities: Vec<LiabilityConfig>,
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
        rate_bps: Option<f64>,
    },
    Loan {
        creditor_id: String,
        amount: f64,
        rate_bps: f64,
        #[serde(default)]
        maturity_days: Option<u32>,
    },
}

impl Scenario {
    pub fn from_toml_str(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    pub fn expand_groups(
        &self, rng: &mut StdRng, _agent_ids: &HashMap<String, AgentId>,
    ) -> (Vec<ConsumerConfig>, Vec<FirmConfig>) {
        let mut generated_consumers = Vec::new();
        let mut generated_firms = Vec::new();

        for group in &self.consumer_groups {
            let bank_ids: Vec<String> = group.bank_mix.keys().cloned().collect();
            let bank_weights: Vec<f64> = group.bank_mix.values().copied().collect();
            let bank_dist = WeightedIndex::new(&bank_weights).unwrap();

            for i in 0..group.count {
                let bank_choice = &bank_ids[bank_dist.sample(rng)];
                let income = group.income.sample(rng);

                let deposit_amount = match &group.deposit {
                    Distribution::PctOfIncome { pct } => income * pct,
                    other => other.sample(rng),
                };

                let consumer_id = format!("{}{:06}", group.id_prefix, i);

                generated_consumers.push(ConsumerConfig {
                    id: consumer_id,
                    bank_id: bank_choice.clone(),
                    income,
                    initial_assets: vec![AssetConfig::Deposit { bank_id: bank_choice.clone(), amount: deposit_amount }],
                    initial_liabilities: vec![],
                    consumption_basket: group.consumption_basket.clone(),
                });
            }
        }

        for group in &self.firm_groups {
            let bank_ids: Vec<String> = group.bank_mix.keys().cloned().collect();
            let bank_weights: Vec<f64> = group.bank_mix.values().copied().collect();
            let bank_dist = WeightedIndex::new(&bank_weights).unwrap();

            for i in 0..group.count {
                let bank_choice = &bank_ids[bank_dist.sample(rng)];
                let deposit_amount = group.deposit.sample(rng);

                let firm_id = format!("{}{:04}", group.id_prefix, i);

                let mut initial_assets =
                    vec![AssetConfig::Deposit { bank_id: bank_choice.clone(), amount: deposit_amount }];

                if let Some(inv_spec) = &group.inventory {
                    initial_assets.push(AssetConfig::Inventory {
                        good_slug: inv_spec.good_slug.clone(),
                        quantity: inv_spec.quantity.sample(rng),
                        unit_cost: inv_spec.unit_cost,
                    });
                }

                let desired_markup = group.desired_markup.as_ref().map(|dm| dm.sample(rng));
                generated_firms.push(FirmConfig {
                    id: firm_id,
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

        let mut rng = StdRng::seed_from_u64(self.config.seed);

        let mut good_ids: HashMap<String, GoodId> = HashMap::new();
        for good_conf in &self.goods {
            let good_id = GoodId(Uuid::new_v4());
            good_ids.insert(good_conf.id.clone(), good_id);
            state.financial_system.goods.goods.insert(
                good_id,
                Good {
                    id: good_id,
                    name: good_conf.name.clone(),
                    unit: good_conf.unit.clone(),
                    category: match good_conf.category.as_str() {
                        "RawMaterial" => GoodCategory::RawMaterial,
                        "IntermediateGood" => GoodCategory::IntermediateGood,
                        "CapitalGood" => GoodCategory::CapitalGood,
                        _ => GoodCategory::ConsumerGood,
                    },
                    cpi_weight: good_conf.cpi_weight,
                },
            );
            state.financial_system.exchange.ensure_goods_market(good_id, &good_conf.name);
        }

        for recipe_conf in &self.recipes {
            let recipe_id = RecipeId(recipe_conf.id.clone());
            let inputs = recipe_conf
                .inputs
                .iter()
                .map(|io| RecipeIO { good_id: *good_ids.get(&io.good_id).unwrap(), quantity: io.quantity })
                .collect();
            let outputs = recipe_conf
                .outputs
                .iter()
                .map(|io| RecipeIO { good_id: *good_ids.get(&io.good_id).unwrap(), quantity: io.quantity })
                .collect();
            state.financial_system.goods.recipes.insert(
                recipe_id.clone(),
                ProductionRecipe {
                    id: recipe_id.clone(),
                    name: recipe_conf.name.clone(),
                    inputs,
                    outputs,
                    labour_hours: recipe_conf.labour_hours,
                },
            );
        }

        let mut decision_models_map: HashMap<AgentId, Box<dyn DecisionModel>> = HashMap::new();
        let mut agent_ids: HashMap<String, AgentId> = HashMap::new();

        {
            let mut temp_rng = StdRng::seed_from_u64(self.config.seed);
            let mut temp_factory = AgentFactory::new(&mut state, &mut temp_rng);
            let _ = temp_factory.initialize_treasury_general_account();

            for bank_conf in &self.banks {
                let bank = temp_factory.create_bank(bank_conf, cb_id);
                agent_ids.insert(bank_conf.id.clone(), bank.id);
                decision_models_map.insert(bank.id, Box::new(BasicBankDecisionModel::default()));
            }
        }

        let (mut expanded_consumers, mut expanded_firms) = self.expand_groups(&mut rng, &agent_ids);

        let mut all_consumers = self.consumers.clone();
        all_consumers.append(&mut expanded_consumers);

        let mut all_firms = self.firms.clone();
        all_firms.append(&mut expanded_firms);

        let mut factory = AgentFactory::new(&mut state, &mut rng);
        let mut count = 0;
        for consumer_conf in &all_consumers {
            let bank_id = *agent_ids.get(&consumer_conf.bank_id).expect("Bank not found for consumer");
            let consumer = factory.create_consumer(consumer_conf, bank_id, cb_id, &agent_ids, count);
            agent_ids.insert(consumer_conf.id.clone(), consumer.id);

            let basket: HashMap<GoodId, f64> = consumer_conf
                .consumption_basket
                .iter()
                .filter_map(|(slug, share)| good_ids.get(slug).map(|id| (*id, *share)))
                .collect();

            decision_models_map.insert(
                consumer.id,
                Box::new(SimpleConsumerDecisionModel { consumption_basket: basket, ..Default::default() }),
            );
            count += 1;
        }

        for firm_conf in &all_firms {
            let bank_id = *agent_ids.get(&firm_conf.bank_id).expect("Bank not found for firm");
            let firm = factory.create_firm(firm_conf, bank_id, cb_id, &agent_ids);
            agent_ids.insert(firm_conf.id.clone(), firm.id);

            decision_models_map.insert(firm.id, firm_conf.decision_model.clone().into_decision_model());

            for asset_conf in &firm_conf.initial_assets {
                if let AssetConfig::Inventory { good_slug, quantity, unit_cost } = asset_conf {
                    if let Some(good_id) = good_ids.get(good_slug) {
                        let unit_cost_money = Money::from_f64(*unit_cost).unwrap_or(Money::ZERO);
                        factory.state.financial_system.add_to_inventory(&firm.id, good_id, *quantity, unit_cost_money);
                    }
                }
            }
        }

        decision_models_map
            .insert(state.financial_system.government.id, Box::new(BasicGovernmentDecisionModel::default()));

        state.financial_system.exchange.ensure_labour_market(LabourMarketId(Uuid::new_v4()), "General");

        state.financial_system.attach_default_pricing_feeds(state.current_date);

        let all_instruments = state.financial_system.instruments.clone();
        for (inst_id, inst) in all_instruments.iter() {
            state.financial_system.exchange.ensure_listed(*inst_id, inst);
        }

        let mut engine = SimulationEngine::new_with_scheduler(state);
        engine.decision_models = decision_models_map;

        engine
    }
}
