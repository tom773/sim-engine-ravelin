use crate::*;
use super::*;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::{BTreeMap, HashMap, VecDeque};

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Exchange {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub goods_markets: HashMap<GoodId, GoodsMarket>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub financial_markets: HashMap<FinancialMarketId, FinancialMarket>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub labour_markets: HashMap<LabourMarketId, LabourMarket>,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub trade_tape: HashMap<MarketId, VecDeque<TimedTrade>>,
}

impl Exchange {
    pub fn register_goods_market(&mut self, good_id: GoodId, goods_registry: &GoodsRegistry) {
        let name = goods_registry.get_good_name(&good_id).unwrap_or("Unknown Good").to_string();
        self.goods_markets.entry(good_id).or_insert_with(|| GoodsMarket::new(good_id, name));
    }

    pub fn register_financial_market(&mut self, market_id: FinancialMarketId) {
        let name = match &market_id {
            FinancialMarketId::FederalFundsOvernight => "Federal Funds Overnight".to_string(),
            FinancialMarketId::TreasuryRepoOvernight => "Treasury Repo Overnight".to_string(),
            FinancialMarketId::Treasury { tenor } => format!("Treasury {}", tenor),
            FinancialMarketId::CorporateBond { rating } => format!("Corporate Bond {:?}", rating),
            FinancialMarketId::DiscountWindow => "Discount Window".to_string(),
            FinancialMarketId::StandingRepoFacility => "Standing Repo Facility".to_string(),
            FinancialMarketId::OvernightReverseRepo => "Reverse Repo Overnight".to_string(),
        };
        self.financial_markets.entry(market_id.clone()).or_insert_with(|| FinancialMarket::new(market_id, name));
    }

    pub fn register_labour_market(&mut self, market_id: LabourMarketId) {
        let name = market_id.clone().to_string();
        self.labour_markets.entry(market_id.clone()).or_insert_with(|| LabourMarket {
            market_id,
            name,
            job_offers: Vec::new(),
            job_applications: Vec::new(),
        });
    }

    pub fn goods_market(&self, good_id: &GoodId) -> Option<&GoodsMarket> { self.goods_markets.get(good_id) }
    pub fn goods_market_mut(&mut self, good_id: &GoodId) -> Option<&mut GoodsMarket> { self.goods_markets.get_mut(good_id) }
    pub fn financial_market(&self, market_id: &FinancialMarketId) -> Option<&FinancialMarket> { self.financial_markets.get(market_id) }
    pub fn financial_market_mut(&mut self, market_id: &FinancialMarketId) -> Option<&mut FinancialMarket> { self.financial_markets.get_mut(market_id) }
    pub fn labour_market_mut(&mut self, market_id: &LabourMarketId) -> Option<&mut LabourMarket> { self.labour_markets.get_mut(market_id) }

    pub fn clear_markets(&mut self, now_ts: i64) -> (Vec<Trade>, HashMap<MarketId, MarketSnapshot>) {
        let mut all_trades = Vec::new();
        let mut snapshots = HashMap::new();

        for (id, market) in self.goods_markets.iter_mut() {
            let market_id = MarketId::Goods(*id);
            snapshots.insert(market_id.clone(), market.snapshot());
            all_trades.extend(market.order_book.clear_and_match(&market_id));
        }
        for (id, market) in self.financial_markets.iter_mut() {
            let market_id = MarketId::Financial(id.clone());
            snapshots.insert(market_id.clone(), market.snapshot());
            all_trades.extend(market.order_book.clear_and_match(&market_id));
        }
        for tr in &all_trades {
            let e = self.trade_tape.entry(tr.market_id.clone()).or_default();
            e.push_back(TimedTrade { at: now_ts, trade: tr.clone() });
            if e.len() > 10_000 { e.pop_front(); }
        }
        (all_trades, snapshots)
    }
    
    pub fn clear_labour_markets(&mut self, state: &SimState) -> Vec<StateEffect> {
        self.labour_markets.values_mut().flat_map(|market| market.clear_and_match(state)).collect()
    }

    pub fn get_treasury_bond_details(&self, fs: &FinancialSystem, tenor: &Tenor) -> Option<BondDetails> {
        for instrument in fs.instruments.values() {
            if let Some(bond_details) = instrument.details.as_any().downcast_ref::<BondDetails>() {
                if bond_details.bond_type == BondType::Government && &bond_details.tenor == tenor {
                    return Some(bond_details.clone());
                }
            }
        }
        None
    }

    pub fn get_treasury_market_summaries(&self, fs: &FinancialSystem) -> Vec<TreasuryMarketSummary> {
        let mut summaries = Vec::new();
        for (market_key, market) in &self.financial_markets {
            if let FinancialMarketId::Treasury { tenor } = market_key {
                summaries.push(TreasuryMarketSummary {
                    market_id: market_key.clone(),
                    tenor: *tenor,
                    price: market.current_price(),
                    yield_to_maturity: market.calculate_ytm(fs).unwrap_or_else(|| market.default_yield()),
                    spread_bps: market.spread_bps(),
                });
            }
        }
        summaries.sort_by_key(|s| s.tenor.to_days());
        summaries
    }

    pub fn calculate_candles(&self, market_id: &MarketId, bucket_secs: i64, limit: usize) -> Vec<Candle> {
        if bucket_secs <= 0 { return Vec::new(); }
        let tape = match self.trade_tape.get(market_id) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut buckets: BTreeMap<i64, Vec<&Trade>> = BTreeMap::new();
        for t in tape {
            let k = (t.at / bucket_secs) * bucket_secs;
            buckets.entry(k).or_default().push(&t.trade);
        }

        buckets.into_iter().rev().take(limit).rev().map(|(ts, group)| {
            let open = group.first().map(|x| x.price).unwrap_or(0.0);
            let close = group.last().map(|x| x.price).unwrap_or(open);
            let high = group.iter().map(|x| x.price).fold(f64::MIN, f64::max);
            let low = group.iter().map(|x| x.price).fold(f64::MAX, f64::min);
            let volume = group.iter().map(|x| x.quantity).sum::<f64>();
            let vwap = if volume > 0.0 { Some(group.iter().map(|x| x.price * x.quantity).sum::<f64>() / volume) } else { None };
            Candle { ts, open, high, low, close, volume, vwap, trades_count: group.len() as u32 }
        }).collect()
    }
}