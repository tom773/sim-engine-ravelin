use crate::*;
pub trait Tradable {
    fn check_holdings(
        &self,
        agent_id: &AgentId,
        quantity: f64,
        fs: &FinancialSystem,
    ) -> Result<(), String>;
}
pub trait RatesMarket {
    fn price_to_daily_rate(&self, price: f64) -> f64;
    fn daily_rate_to_annual_bps(&self, daily_rate: f64) -> BasisPoints;
    fn annual_bps_to_daily_rate(&self, annual_bps: BasisPoints) -> f64;
}

pub trait MarketSnapshotProvider {
    fn snapshot(&self) -> super::market_types::MarketSnapshot;
}

pub trait MarketSummaryProvider {
    fn summary(&self) -> MarketSummary;
}