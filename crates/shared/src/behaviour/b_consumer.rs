use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum PersonalityArchetype {
    #[default]
    Defensive,
    Balanced,
    Aggresive,
}

#[derive(Clone, Debug)]
pub struct PersonalityParams {
    pub prop_to_consume: f64,
}

impl PersonalityArchetype {
    pub fn get_params(&self) -> PersonalityParams {
        match self {
            PersonalityArchetype::Balanced => PersonalityParams { prop_to_consume: 0.7 },
            PersonalityArchetype::Aggresive => PersonalityParams { prop_to_consume: 0.8 },
            PersonalityArchetype::Defensive => PersonalityParams { prop_to_consume: 0.6 },
        }
    }
}
