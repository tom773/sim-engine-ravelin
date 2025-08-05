use crate::*;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml;
use uuid::Uuid;
use serde_with::{serde_as, DisplayFromStr};

const GOODS_NAMESPACE: Uuid = Uuid::from_u128(0x4A8B382D22C14A4C8F1A2E3D4B5C6F7A);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Good {
    pub id: GoodId,
    pub name: String,
    pub unit: String,
    pub category: GoodCategory,
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy)]
pub enum GoodCategory {
    RawMaterial,
    IntermediateGood,
    FinalGood,
    Energy,
    Service,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionRecipe {
    pub id: RecipeId,
    pub name: String,
    pub inputs: Vec<(GoodId, f64)>,
    pub output: (GoodId, f64),
    pub labour_hours: f64,
    pub capital_required: f64,
    pub efficiency: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InventoryItem {
    pub quantity: f64,
    pub unit_cost: f64,
}

impl GoodId {
    pub fn from_slug(slug: &str) -> Self {
        Self(Uuid::new_v5(&GOODS_NAMESPACE, slug.as_bytes()))
    }
}

impl RecipeId {
    pub fn from_name(name: &str) -> Self {
        Self(Uuid::new_v5(&GOODS_NAMESPACE, name.as_bytes()))
    }
}

#[derive(Debug, Deserialize)]
struct TomlConfig {
    goods: Vec<TomlGood>,
    recipes: Vec<TomlRecipe>,
}

#[derive(Debug, Deserialize)]
struct TomlGood {
    slug: String,
    name: String,
    unit: String,
    category: GoodCategory,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TomlRecipe {
    name: String,
    output: TomlRecipeItem,
    inputs: Vec<TomlRecipeItem>,
    labour_hours: f64,
    capital_required: f64,
    efficiency: f64,
}

#[derive(Debug, Deserialize)]
struct TomlRecipeItem {
    slug: String,
    qty: f64,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoodsRegistry {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub goods: HashMap<GoodId, Good>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub recipes: HashMap<RecipeId, ProductionRecipe>,
    #[serde(skip)]
    slug_to_id: HashMap<String, GoodId>,
    #[serde(skip)]
    name_to_recipe_id: HashMap<String, RecipeId>,
}

impl Default for GoodsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl GoodsRegistry {
    pub fn new() -> Self {
        Self {
            goods: HashMap::new(),
            recipes: HashMap::new(),
            slug_to_id: HashMap::new(),
            name_to_recipe_id: HashMap::new(),
        }
    }

    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        let config: TomlConfig = toml::from_str(toml_str)?;
        let mut registry = Self::new();

        for good_def in config.goods {
            let id = GoodId::from_slug(&good_def.slug);
            let good = Good { id, name: good_def.name, unit: good_def.unit, category: good_def.category };
            registry.goods.insert(id, good);
            registry.slug_to_id.insert(good_def.slug, id);
        }

        for recipe_def in config.recipes {
            let recipe_id = RecipeId::from_name(&recipe_def.name);

            let output_good_id = registry.get_good_id_by_slug(&recipe_def.output.slug).unwrap_or_else(|| {
                panic!("Output good '{}' for recipe '{}' not found", recipe_def.output.slug, recipe_def.name)
            });

            let inputs = recipe_def
                .inputs
                .iter()
                .map(|input_def| {
                    let input_good_id = registry.get_good_id_by_slug(&input_def.slug).unwrap_or_else(|| {
                        panic!("Input good '{}' for recipe '{}' not found", input_def.slug, recipe_def.name)
                    });
                    (input_good_id, input_def.qty)
                })
                .collect();

            let recipe = ProductionRecipe {
                id: recipe_id,
                name: recipe_def.name.clone(),
                inputs,
                output: (output_good_id, recipe_def.output.qty),
                labour_hours: recipe_def.labour_hours,
                capital_required: recipe_def.capital_required,
                efficiency: recipe_def.efficiency,
            };
            registry.recipes.insert(recipe_id, recipe.clone());
            registry.name_to_recipe_id.insert(recipe_def.name, recipe_id);
        }

        Ok(registry)
    }

    pub fn get_good_id_by_slug(&self, slug: &str) -> Option<GoodId> {
        self.slug_to_id.get(slug).copied()
    }

    pub fn get_recipe_id_by_name(&self, name: &str) -> Option<RecipeId> {
        self.name_to_recipe_id.get(name).copied()
    }

    pub fn get_good_name(&self, id: &GoodId) -> Option<&str> {
        self.goods.get(id).map(|good| good.name.as_str())
    }

    pub fn get_recipe(&self, id: &RecipeId) -> Option<&ProductionRecipe> {
        self.recipes.get(id)
    }
}

pub static CATALOGUE: Lazy<GoodsRegistry> = Lazy::new(|| {
    GoodsRegistry::from_toml(include_str!("../../../../config/goods.toml")).expect("failed to parse goods catalogue")
});