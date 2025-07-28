use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimAction {
    // Banks
    Deposit { agent_id: AgentId, bank: AgentId, amount: f64 },
    Withdraw { agent_id: AgentId, bank: AgentId, amount: f64 },
    Transfer { agent_id: AgentId, from: AgentId, to: AgentId, amount: f64 },
    // Firms
    Wages { agent_id: AgentId, amount: f64 },
    Hire { agent_id: AgentId, count: u32 },
    Produce { agent_id: AgentId, recipe_id: RecipeId, batches: u32 },
    // Households 
    Purchase { agent_id: AgentId, seller: AgentId, good_id: GoodId, amount: f64 },
    Consume { agent_id: AgentId, good_id: GoodId, amount: f64 },
    // Central Bank (Sort of) 
    UpdateReserves { bank: AgentId, amount_change: f64 },
    InjectLiquidity,
    // Trading
    PostBid { agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64 },
    PostAsk { agent_id: AgentId, market_id: MarketId, quantity: f64, price: f64 },
}

impl SimAction {
    pub fn name(&self) -> String {
        match self {
            SimAction::Wages { .. } => "Issue Income".to_string(),
            SimAction::Deposit { .. } => "Deposit Cash".to_string(),
            SimAction::Withdraw { .. } => "Withdraw Cash".to_string(),
            SimAction::Transfer { .. } => "Transfer Funds".to_string(),
            SimAction::Purchase { .. } => "Purchase Good".to_string(),
            SimAction::UpdateReserves { .. } => "Update Reserves".to_string(),
            SimAction::Hire { .. } => "Hire Employees".to_string(),
            SimAction::Produce { .. } => "Produce Goods".to_string(),
            SimAction::Consume { .. } => "Consume Goods".to_string(),
            SimAction::InjectLiquidity => "Inject Liquidity".to_string(),
            SimAction::PostBid { .. } => "Post Bid".to_string(),
            SimAction::PostAsk { .. } => "Post Ask".to_string(),
        }
    }
}
