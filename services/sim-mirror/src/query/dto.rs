use crate::digest::{AgentBalanceDigest, DigestEvent, MarketsDigest, RiskDigest, StateDigest, StatusDigest};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct DashboardDto {
    pub tick: u32,
    pub status: StatusDigest,
    pub risk: RiskDigest,
    pub top_agents: Vec<AgentBalanceDigest>,
    pub top_liquidity: Vec<AgentBalanceDigest>,
    pub markets: MarketsDigest,
    pub highlights: Vec<DigestEvent>,
}

impl From<&StateDigest> for DashboardDto {
    fn from(digest: &StateDigest) -> Self {
        Self {
            tick: digest.tick,
            status: digest.status.clone(),
            risk: digest.risk.clone(),
            top_agents: digest.agents.leaderboard.clone(),
            top_liquidity: digest.agents.liquidity_leaderboard.clone(),
            markets: digest.markets.clone(),
            highlights: digest.highlights.clone(),
        }
    }
}
