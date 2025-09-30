use crate::digest::{AgentBalanceDigest, MarketsDigest, RiskDigest, StateDigest, StatusDigest};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct DashboardDto {
    pub tick: u32,
    pub status: StatusDigest,
    pub risk: RiskDigest,
    pub top_agents: Vec<AgentBalanceDigest>,
    pub top_liquidity: Vec<AgentBalanceDigest>,
    pub markets: MarketsDigest,
}

impl From<&StateDigest> for DashboardDto {
    fn from(digest: &StateDigest) -> Self {
        let (top_agents, top_liquidity) = if let Some(catalogue) = &digest.agents.catalogue {
            let mut by_net_worth = catalogue.roster.clone();
            by_net_worth.sort_by(|a, b| b.net_worth.partial_cmp(&a.net_worth).unwrap_or(std::cmp::Ordering::Equal));
            by_net_worth.truncate(10);

            let mut by_liquidity = catalogue.roster.clone();
            by_liquidity.sort_by(|a, b| b.liquidity.partial_cmp(&a.liquidity).unwrap_or(std::cmp::Ordering::Equal));
            by_liquidity.truncate(10);

            (by_net_worth, by_liquidity)
        } else {
            (Vec::new(), Vec::new())
        };

        Self {
            tick: digest.tick,
            status: digest.status.clone(),
            risk: digest.risk.clone(),
            top_agents,
            top_liquidity,
            markets: digest.markets.clone(),
        }
    }
}
