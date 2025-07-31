use crate::*;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml;
use uuid::Uuid;
use std::{str::FromStr, fmt};
use serde_with::serde_as;

const GOODS_NAMESPACE: Uuid = Uuid::from_u128(0x4A8B382D22C14A4C8F1A2E3D4B5C6F7A);
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy)]
pub struct GoodId(pub Uuid);
prep_serde_as!(GoodId, Uuid); // Use the macro

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy)]
pub struct RecipeId(pub Uuid);
prep_serde_as!(RecipeId, Uuid); // Use the macro

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
    #[serde_as(as = "HashMap<_, _>")] // <--- ADD THIS
    pub goods: HashMap<GoodId, Good>,
    #[serde_as(as = "HashMap<_, _>")] // <--- ADD THIS
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
            let id = GoodId(Uuid::new_v5(&GOODS_NAMESPACE, good_def.slug.as_bytes()));
            let good = Good { id, name: good_def.name, unit: good_def.unit, category: good_def.category };
            registry.goods.insert(id, good);
            registry.slug_to_id.insert(good_def.slug, id);
        }

        for recipe_def in config.recipes {
            let recipe_id = RecipeId(Uuid::new_v5(&GOODS_NAMESPACE, recipe_def.name.as_bytes()));

            let output_good_id = registry
                .slug_to_id
                .get(&recipe_def.output.slug)
                .unwrap_or_else(|| {
                    panic!("Output good '{}' for recipe '{}' not found", recipe_def.output.slug, recipe_def.name)
                });

            let inputs = recipe_def
                .inputs
                .iter()
                .map(|input_def| {
                    let input_good_id = registry.slug_to_id.get(&input_def.slug).unwrap_or_else(|| {
                        panic!("Input good '{}' for recipe '{}' not found", input_def.slug, recipe_def.name)
                    });
                    (*input_good_id, input_def.qty)
                })
                .collect();

            let recipe = ProductionRecipe {
                id: recipe_id,
                name: recipe_def.name.clone(),
                inputs,
                output: (*output_good_id, recipe_def.output.qty),
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

    pub fn get_good_slug(&self, id: &GoodId) -> Option<&str> {
        self.slug_to_id.iter().find(|(_, good_id)| *good_id == id).map(|(slug, _)| slug.as_str())
    }

    pub fn get_recipe_name(&self, id: &RecipeId) -> Option<&str> {
        self.recipes.get(id).map(|recipe| recipe.name.as_str())
    }
}

pub static CATALOGUE: Lazy<GoodsRegistry> = Lazy::new(|| {
    GoodsRegistry::from_toml(include_str!("../../../../engine/src/state/config/goods.toml"))
        .expect("failed to parse goods catalogue")
});

#[macro_export]
macro_rules! good_id {
    ($slug:literal) => {
        $crate::model::goods::CATALOGUE
            .get_good_id_by_slug($slug)
            .expect(concat!("unknown good slug: ", $slug))
    };
}

#[macro_export]
macro_rules! recipe_id {
    ($name:literal) => {
        $crate::model::goods::CATALOGUE
            .get_recipe_id_by_name($name)
            .expect(concat!("unknown recipe name: ", $name))
    };
}

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
pub struct ProductionRecipe {
    pub id: RecipeId,
    pub name: String,
    pub inputs: Vec<(GoodId, f64)>,
    pub output: (GoodId, f64),
    pub labour_hours: f64,
    pub capital_required: f64,
    pub efficiency: f64,
}