use serde::{Deserialize, Serialize};
use ravelin_core::*;
use std::{error::Error, fs};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub description: String,
    pub config: ScenarioConfig,
    pub banks: Vec<BankConfig>,
    pub firms: Vec<FirmConfig>,
    pub consumers: Vec<ConsumerConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioConfig {
    pub iterations: u32,
    pub treasury_tenors_to_register: Vec<Tenor>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankConfig {
    pub id: String,
    pub name: String,
    pub initial_reserves: f64,
    pub initial_bonds: Vec<BondConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FirmConfig {
    pub id: String, // Added for testability
    pub name: String,
    pub bank_id: String,
    pub recipe_name: String,
    pub initial_cash: f64,
    pub initial_inventory: Vec<InventoryConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumerConfig {
    pub id: String, // Added for testability
    pub bank_id: String,
    pub initial_cash: f64,
    pub income: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BondConfig {
    pub tenor: Tenor,
    pub face_value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InventoryConfig {
    pub good_slug: String,
    pub quantity: f64,
    pub unit_cost: f64,
}

impl Scenario {
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    pub fn from_file(path: &str) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        Self::from_toml(&content).map_err(|e| e.into())
    }
}



#[cfg(test)]
mod scenario_tests {
    use super::*;

    #[test]
    fn load_from_yaml() {
        let yaml_str = include_str!("./config/config.toml");
        
        let scenario = Scenario::from_toml(yaml_str).unwrap();
        println!("{:#?}", scenario);
    }
}
