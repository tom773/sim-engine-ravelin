use serde::{Serialize, Deserialize};
use crate::*;
use rand::{rngs::StdRng, RngCore};
use dyn_clone::{clone_trait_object, DynClone};
use std::fmt::Debug;
use ndarray::Array1;

pub trait Agent {
    type DecisionType;
    
    fn decide(&self, fs: &FinancialSystem, rng: &mut StdRng) -> Vec<Self::DecisionType>;
    fn act(&self, decisions: &[Self::DecisionType]) -> Vec<SimAction>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsumerDecision {
    Spend { 
        agent_id: AgentId, 
        seller_id: AgentId, 
        amount: f64, 
        good_id: GoodId 
    },
    Save { 
        agent_id: AgentId, 
        amount: f64 
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FirmDecision {
    Produce { 
        good_id: GoodId, 
        quantity: u32 
    },
    Hire { 
        quantity: u32 
    },
    SetPrice {
        good_id: GoodId,
        price: f64,
    },
}

#[typetag::serde(tag = "type")]
pub trait DecisionModel: DynClone + Send + Sync {
    fn decide(&self, consumer: &Consumer, fs: &FinancialSystem, rng: &mut dyn RngCore) -> Vec<ConsumerDecision>;
}

clone_trait_object!(DecisionModel);

impl Debug for dyn DecisionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecisionModel")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasicDecisionModel {
    pub propensity_to_consume: f64,
}

#[typetag::serde]
impl DecisionModel for BasicDecisionModel {
    fn decide(&self, consumer: &Consumer, fs: &FinancialSystem, _rng: &mut dyn RngCore) -> Vec<ConsumerDecision> {
        let cash_holdings = fs.balance_sheets
            .get(&consumer.id)
            .map(|bs| bs.assets.values()
                .filter(|inst| matches!(inst.instrument_type, crate::InstrumentType::Cash))
                .map(|inst| inst.principal)
                .sum::<f64>()
            )
            .unwrap_or(0.0);
            
        let mut decisions = Vec::new();
        let total_available = consumer.income + cash_holdings;

        let seller_id = fs.exchange.goods_market(&GoodId::generic())
            .and_then(|market| market.best_ask(&GoodId::generic()))
            .map(|ask| ask.agent_id.clone());

        let spend_amount = total_available * self.propensity_to_consume;
        if spend_amount > 0.0 && seller_id.is_some() {
            decisions.push(ConsumerDecision::Spend {
                agent_id: consumer.id.clone(),
                seller_id: seller_id.unwrap(),
                amount: spend_amount.min(1000.0), // Cap at 1000 for now
                good_id: GoodId::generic(),
            }); 
        }

        let save_amount = total_available * (1.0 - self.propensity_to_consume);
        if save_amount > 0.0 {
            decisions.push(ConsumerDecision::Save {
                agent_id: consumer.id.clone(),
                amount: save_amount.min(1000.0), // Cap at 1000 for now
            });
        }
        
        decisions 
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MLDecisionModel {
    #[serde(skip)]
    pub predictor: Option<Box<dyn SpendingPredictor>>,
    pub model_path: String,
}

#[typetag::serde]
impl DecisionModel for MLDecisionModel {
    fn decide(&self, consumer: &Consumer, fs: &FinancialSystem, _rng: &mut dyn RngCore) -> Vec<ConsumerDecision> {
        if let Some(predictor) = &self.predictor {
            let features = extract_consumer_features(consumer, fs);
            let predicted_annual_spending = predictor.predict_spending(&features);
            
            let cash_holdings = fs.balance_sheets
                .get(&consumer.id)
                .map(|bs| bs.assets.values()
                    .filter(|inst| matches!(inst.instrument_type, crate::InstrumentType::Cash))
                    .map(|inst| inst.principal)
                    .sum::<f64>()
                )
                .unwrap_or(0.0);
            
            let total_available = consumer.income + cash_holdings;
            let spending_per_period = predicted_annual_spending / 52.0; // Weekly
            let spend_amount = spending_per_period.min(total_available);
            let save_amount = total_available - spend_amount;
            
            let mut decisions = Vec::new();
            
            let seller_id = fs.exchange.goods_market(&GoodId::generic())
                .and_then(|market| market.best_ask(&GoodId::generic()))
                .map(|ask| ask.agent_id.clone());
            
            if spend_amount > 0.0 && seller_id.is_some() {
                decisions.push(ConsumerDecision::Spend {
                    agent_id: consumer.id.clone(),
                    seller_id: seller_id.unwrap(),
                    amount: spend_amount,
                    good_id: GoodId::generic(),
                });
            }
            
            if save_amount > 0.0 {
                decisions.push(ConsumerDecision::Save {
                    agent_id: consumer.id.clone(),
                    amount: save_amount,
                });
            }
            
            decisions
        } else {
            let basic = BasicDecisionModel { propensity_to_consume: 0.7 };
            basic.decide(consumer, fs, _rng)
        }
    }
}

fn extract_consumer_features(consumer: &Consumer, _fs: &FinancialSystem) -> Array1<f64> {
    let income = consumer.get_income();
    let log_income = income.max(1000.0).ln();
    
    let income_bracket = if income < 30000.0 { 1.0 }
                        else if income < 50000.0 { 2.0 }
                        else if income < 75000.0 { 3.0 }
                        else if income < 100000.0 { 4.0 }
                        else if income < 150000.0 { 5.0 }
                        else { 6.0 };

    let food_share = 0.15;
    let housing_share = 0.30;
    let transport_share = 0.20;
    let health_share = 0.10;

    let age = consumer.get_age();
    let age_group = if age < 35 { 1.0 } 
                    else if age < 55 { 2.0 } 
                    else if age < 65 { 3.0 } 
                    else { 4.0 };

    let education = consumer.get_education_level_numeric() as f64;
    
    Array1::from(vec![
        income,
        log_income,
        age_group,
        consumer.get_family_size() as f64,
        if consumer.get_has_children() { 1.0 } else { 0.0 },
        education,
        consumer.get_housing_status_numeric() as f64,
        if consumer.get_is_urban() { 1.0 } else { 0.0 },
        1.0,
        consumer.get_region_numeric() as f64,
        food_share,
        housing_share,
        transport_share,
        health_share,
        income_bracket * age_group,
        income_bracket * education,
    ])
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParametricMPC {
    pub mpc_min: f64,
    pub mpc_max: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

#[typetag::serde]
impl DecisionModel for ParametricMPC {
    fn decide(&self, consumer: &Consumer, fs: &FinancialSystem, _rng: &mut dyn RngCore) -> Vec<ConsumerDecision> {
        let cash = fs.get_liquid_assets(&consumer.id);
        let total = consumer.income + cash;
        let wealth_ratio = fs.get_total_assets(&consumer.id) / consumer.income.max(1.0);

        let mpc = self.mpc_min + (self.mpc_max - self.mpc_min)
                 / (1.0 + (self.a + self.b * consumer.income.ln()
                                + self.c * wealth_ratio).exp());

        let spend_amount = mpc * total;
        let save_amount = total - spend_amount;
        
        let mut decisions = Vec::new();
        
        let seller_id = fs.exchange.goods_market(&GoodId::generic())
            .and_then(|market| market.best_ask(&GoodId::generic()))
            .map(|ask| ask.agent_id.clone());
        
        if spend_amount > 0.0 && seller_id.is_some() {
            decisions.push(ConsumerDecision::Spend {
                agent_id: consumer.id.clone(),
                seller_id: seller_id.unwrap(),
                amount: spend_amount,
                good_id: GoodId::generic(),
            });
        }
        
        if save_amount > 0.0 {
            decisions.push(ConsumerDecision::Save {
                agent_id: consumer.id.clone(),
                amount: save_amount,
            });
        }
        
        decisions
    }
}