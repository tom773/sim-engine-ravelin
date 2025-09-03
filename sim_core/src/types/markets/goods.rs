// In crates/sim_core/src/types/goods.rs

use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoodCategory {
    RawMaterial,
    IntermediateGood,
    CapitalGood,
    ConsumerGood,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Good {
    pub id: GoodId,
    pub name: String,
    pub unit: String,
    pub category: GoodCategory,
    pub cpi_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeIO {
    pub good_id: GoodId,
    pub quantity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionRecipe {
    pub id: RecipeId,
    pub name: String,
    pub inputs: Vec<RecipeIO>,
    pub outputs: Vec<RecipeIO>,
    pub labour_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoodsRegistry {
    pub goods: HashMap<GoodId, Good>,
    pub recipes: HashMap<RecipeId, ProductionRecipe>,
}