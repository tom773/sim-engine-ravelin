use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::pserde;

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, Copy, Default)]
pub struct AgentId(pub Uuid);
pserde!(AgentId, Uuid);

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, Copy, Default)]
pub struct InstrumentId(pub Uuid);
pserde!(InstrumentId, Uuid);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy, Default)]
pub struct AssetId(pub Uuid);
pserde!(AssetId, Uuid);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy, Default)]
pub struct GoodId(pub Uuid);
pserde!(GoodId, Uuid);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct RecipeId(pub String);
pserde!(RecipeId, String);

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, Copy, Default)]
pub struct LabourMarketId(pub Uuid);
pserde!(LabourMarketId, Uuid);