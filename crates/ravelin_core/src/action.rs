use crate::*;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Clone, Debug, Serialize, Deserialize, Display, EnumString)]
pub enum SimAction {
    Deposit { agent_id: AgentId, bank: AgentId, amount: f64 },
    Withdraw { agent_id: AgentId, bank: AgentId, amount: f64 },
    Transfer { agent_id: AgentId, from: AgentId, to: AgentId, amount: f64 },
    PayWages { agent_id: AgentId, employee: AgentId, amount: f64 },
    Hire { agent_id: AgentId, count: u32 },
    Produce { agent_id: AgentId, recipe_id: RecipeId, batches: u32 },
    Purchase { agent_id: AgentId, seller: AgentId, good_id: GoodId, amount: f64 },
    Consume { agent_id: AgentId, good_id: GoodId, amount: f64 },
    UpdateReserves { bank: AgentId, amount_change: f64 },
    InjectLiquidity,
    PostBid { agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64 },
    PostAsk { agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64 },
}

impl SimAction {
    pub fn name(&self) -> String {
        self.to_string()
    }
}