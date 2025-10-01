use crate::*;
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ClearingResult {
    pub matches: Vec<OvernightMatch>,
    pub unmatched_bids: Vec<ONQuote>,
    pub unmatched_offers: Vec<ONQuote>,
    pub clearing_stats: ClearingStats,
}

#[derive(Debug, Clone)]
pub struct OvernightMatch {
    pub lender: AgentId,
    pub borrower: AgentId,
    pub amount: f64,
    pub rate_bps: BasisPoints,
    pub venue: OvernightVenue,
    pub collateral: Option<CollateralDetails>,
}

#[derive(Debug, Clone)]
pub struct CollateralDetails {
    pub instrument_id: InstrumentId,
    pub quantity: f64,
    pub haircut_pct: f64,
}

#[derive(Debug, Clone, Default)]
pub struct ClearingStats {
    pub total_volume: f64,
    pub weighted_avg_rate: f64,
    pub num_matches: usize,
    pub unfilled_borrow_demand: f64,
    pub unfilled_lend_supply: f64,
}

pub fn clear_fedfunds(quotes: Vec<ONQuote>) -> ClearingResult {
    let (mut lenders, mut borrowers): (Vec<_>, Vec<_>) =
        quotes.into_iter().partition(|q| matches!(q.side, ONQuoteSide::Lend));

    lenders.sort_by(|a, b| a.limit_rate_bps.cmp(&b.limit_rate_bps));
    borrowers.sort_by(|a, b| b.limit_rate_bps.cmp(&a.limit_rate_bps));

    let mut matches = Vec::new();
    let mut total_volume = 0.0;
    let mut total_rate_volume = 0.0;

    let mut lender_idx = 0;
    let mut borrower_idx = 0;

    let mut lender_remaining: HashMap<usize, f64> = HashMap::new();
    let mut borrower_remaining: HashMap<usize, f64> = HashMap::new();

    while lender_idx < lenders.len() && borrower_idx < borrowers.len() {
        let lender = &lenders[lender_idx];
        let borrower = &borrowers[borrower_idx];

        if borrower.limit_rate_bps < lender.limit_rate_bps {
            break;
        }

        let lender_available = lender_remaining.get(&lender_idx).copied().unwrap_or(lender.notional);
        let borrower_needed = borrower_remaining.get(&borrower_idx).copied().unwrap_or(borrower.notional);

        let trade_amount = lender_available.min(borrower_needed);

        let lender_min_fill_ok = trade_amount >= lender.min_fill || lender_available - trade_amount < 1e-6;
        let borrower_min_fill_ok = trade_amount >= borrower.min_fill || borrower_needed - trade_amount < 1e-6;

        if !lender_min_fill_ok || !borrower_min_fill_ok {
            if lender.min_fill > borrower.min_fill {
                lender_idx += 1;
            } else {
                borrower_idx += 1;
            }
            continue;
        }

        let trade_rate = borrower.limit_rate_bps;

        matches.push(OvernightMatch {
            lender: lender.agent,
            borrower: borrower.agent,
            amount: trade_amount,
            rate_bps: trade_rate,
            venue: OvernightVenue::FedFundsON,
            collateral: None,
        });

        total_volume += trade_amount;
        total_rate_volume += trade_amount * trade_rate.to_f64().unwrap_or(0.0);

        let new_lender_remaining = lender_available - trade_amount;
        let new_borrower_remaining = borrower_needed - trade_amount;

        if new_lender_remaining < 1e-6 {
            lender_idx += 1;
            lender_remaining.remove(&lender_idx);
        } else {
            lender_remaining.insert(lender_idx, new_lender_remaining);
        }

        if new_borrower_remaining < 1e-6 {
            borrower_idx += 1;
            borrower_remaining.remove(&borrower_idx);
        } else {
            borrower_remaining.insert(borrower_idx, new_borrower_remaining);
        }
    }

    let mut unmatched_lenders = Vec::new();
    let mut unmatched_borrowers = Vec::new();
    let mut unfilled_supply = 0.0;
    let mut unfilled_demand = 0.0;

    for (idx, lender) in lenders.iter().enumerate() {
        let remaining = lender_remaining
            .get(&idx)
            .copied()
            .unwrap_or_else(|| if idx >= lender_idx { lender.notional } else { 0.0 });
        if remaining > 1e-6 {
            unmatched_lenders.push(lender.clone());
            unfilled_supply += remaining;
        }
    }

    for (idx, borrower) in borrowers.iter().enumerate() {
        let remaining = borrower_remaining
            .get(&idx)
            .copied()
            .unwrap_or_else(|| if idx >= borrower_idx { borrower.notional } else { 0.0 });
        if remaining > 1e-6 {
            unmatched_borrowers.push(borrower.clone());
            unfilled_demand += remaining;
        }
    }

    let weighted_avg_rate = if total_volume > 0.0 { total_rate_volume / total_volume } else { 0.0 };
    let num_matches = matches.len();

    ClearingResult {
        matches,
        unmatched_bids: unmatched_borrowers,
        unmatched_offers: unmatched_lenders,
        clearing_stats: ClearingStats {
            total_volume,
            weighted_avg_rate,
            num_matches,
            unfilled_borrow_demand: unfilled_demand,
            unfilled_lend_supply: unfilled_supply,
        },
    }
}

pub fn clear_repo_gc1d(quotes: Vec<ONQuote>, state: &crate::state::SimState, fixed_haircut_pct: f64) -> ClearingResult {
    let (mut lenders, mut borrowers): (Vec<_>, Vec<_>) =
        quotes.into_iter().partition(|q| matches!(q.side, ONQuoteSide::Lend));

    lenders.sort_by(|a, b| a.limit_rate_bps.cmp(&b.limit_rate_bps));
    borrowers.sort_by(|a, b| b.limit_rate_bps.cmp(&a.limit_rate_bps));

    let mut matches = Vec::new();
    let mut total_volume = 0.0;
    let mut total_rate_volume = 0.0;

    let mut lender_idx = 0;
    let mut borrower_idx = 0;

    let mut lender_remaining: HashMap<usize, f64> = HashMap::new();
    let mut borrower_remaining: HashMap<usize, f64> = HashMap::new();
    let mut used_collateral: HashMap<AgentId, Vec<(InstrumentId, f64)>> = HashMap::new();

    while lender_idx < lenders.len() && borrower_idx < borrowers.len() {
        let lender = &lenders[lender_idx];
        let borrower = &borrowers[borrower_idx];

        if borrower.limit_rate_bps < lender.limit_rate_bps {
            break;
        }

        let lender_available = lender_remaining.get(&lender_idx).copied().unwrap_or(lender.notional);
        let borrower_needed = borrower_remaining.get(&borrower_idx).copied().unwrap_or(borrower.notional);

        let collateral =
            find_eligible_collateral(&borrower.agent, borrower_needed, fixed_haircut_pct, state, &used_collateral);

        if collateral.is_none() {
            borrower_idx += 1;
            continue;
        }

        let (collateral_id, collateral_qty, collateral_value) = collateral.unwrap();
        let max_borrowable = collateral_value * (1.0 - fixed_haircut_pct / 100.0);
        let trade_amount = lender_available.min(borrower_needed).min(max_borrowable);

        let lender_min_fill_ok = trade_amount >= lender.min_fill || lender_available - trade_amount < 1e-6;
        let borrower_min_fill_ok = trade_amount >= borrower.min_fill || borrower_needed - trade_amount < 1e-6;

        if !lender_min_fill_ok || !borrower_min_fill_ok {
            if lender.min_fill > borrower.min_fill {
                lender_idx += 1;
            } else {
                borrower_idx += 1;
            }
            continue;
        }

        let trade_rate = borrower.limit_rate_bps;

        matches.push(OvernightMatch {
            lender: lender.agent,
            borrower: borrower.agent,
            amount: trade_amount,
            rate_bps: trade_rate,
            venue: OvernightVenue::RepoGC1D,
            collateral: Some(CollateralDetails {
                instrument_id: collateral_id,
                quantity: collateral_qty,
                haircut_pct: fixed_haircut_pct,
            }),
        });

        used_collateral.entry(borrower.agent).or_default().push((collateral_id, collateral_qty));

        total_volume += trade_amount;
        total_rate_volume += trade_amount * trade_rate.to_f64().unwrap_or(0.0);

        let new_lender_remaining = lender_available - trade_amount;
        let new_borrower_remaining = borrower_needed - trade_amount;

        if new_lender_remaining < 1e-6 {
            lender_idx += 1;
            lender_remaining.remove(&lender_idx);
        } else {
            lender_remaining.insert(lender_idx, new_lender_remaining);
        }

        if new_borrower_remaining < 1e-6 {
            borrower_idx += 1;
            borrower_remaining.remove(&borrower_idx);
        } else {
            borrower_remaining.insert(borrower_idx, new_borrower_remaining);
        }
    }

    let mut unmatched_lenders = Vec::new();
    let mut unmatched_borrowers = Vec::new();
    let mut unfilled_supply = 0.0;
    let mut unfilled_demand = 0.0;

    for (idx, lender) in lenders.iter().enumerate() {
        let remaining = lender_remaining
            .get(&idx)
            .copied()
            .unwrap_or_else(|| if idx >= lender_idx { lender.notional } else { 0.0 });
        if remaining > 1e-6 {
            unmatched_lenders.push(lender.clone());
            unfilled_supply += remaining;
        }
    }

    for (idx, borrower) in borrowers.iter().enumerate() {
        let remaining = borrower_remaining
            .get(&idx)
            .copied()
            .unwrap_or_else(|| if idx >= borrower_idx { borrower.notional } else { 0.0 });
        if remaining > 1e-6 {
            unmatched_borrowers.push(borrower.clone());
            unfilled_demand += remaining;
        }
    }

    let weighted_avg_rate = if total_volume > 0.0 { total_rate_volume / total_volume } else { 0.0 };
    let num_matches = matches.len();

    ClearingResult {
        matches,
        unmatched_bids: unmatched_borrowers,
        unmatched_offers: unmatched_lenders,
        clearing_stats: ClearingStats {
            total_volume,
            weighted_avg_rate,
            num_matches,
            unfilled_borrow_demand: unfilled_demand,
            unfilled_lend_supply: unfilled_supply,
        },
    }
}

fn find_eligible_collateral(
    borrower: &AgentId, cash_needed: f64, haircut_pct: f64, state: &crate::state::SimState,
    used_collateral: &HashMap<AgentId, Vec<(InstrumentId, f64)>>,
) -> Option<(InstrumentId, f64, f64)> {
    let account = state.financial_system.clearing_house.csd.custody_accounts.get(borrower)?;

    let already_used: HashMap<InstrumentId, f64> =
        used_collateral.get(borrower).map(|v| v.iter().cloned().collect()).unwrap_or_default();

    for (inst_id, holding) in &account.holdings {
        let available = holding.available - already_used.get(inst_id).copied().unwrap_or(0.0);

        if available < 1e-6 {
            continue;
        }

        if let Some(instrument) = state.financial_system.instruments.get(inst_id) {
            if let crate::types::instrument::InstrumentRuntime::Bond(bond_state) = instrument.state() {
                if bond_state.bond_type() == crate::types::instrument::BondType::Government {
                    let collateral_value = bond_state.archetype.face_value.to_f64();

                    let required_collateral_value = cash_needed / (1.0 - haircut_pct / 100.0);
                    let required_qty = (required_collateral_value / collateral_value).min(available);

                    if required_qty > 1e-6 {
                        return Some((*inst_id, required_qty, required_qty * collateral_value));
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    #[test]
    fn test_simple_fedfunds_match() {
        let lender = ONQuote {
            venue: OvernightVenue::FedFundsON,
            agent: AgentId(Uuid::from_u128(1)),
            side: ONQuoteSide::Lend,
            notional: 1_000_000.0,
            limit_rate_bps: dec!(500),
            haircut: None,
            preferred_collateral: None,
            min_fill: 100_000.0,
            ts: 0,
        };

        let borrower = ONQuote {
            venue: OvernightVenue::FedFundsON,
            agent: AgentId(Uuid::from_u128(2)),
            side: ONQuoteSide::Borrow,
            notional: 1_000_000.0,
            limit_rate_bps: dec!(525),
            haircut: None,
            preferred_collateral: None,
            min_fill: 100_000.0,
            ts: 0,
        };

        let result = clear_fedfunds(vec![lender, borrower]);

        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].amount, 1_000_000.0);
        assert_eq!(result.matches[0].rate_bps, dec!(525));
        assert_eq!(result.clearing_stats.total_volume, 1_000_000.0);
    }

    #[test]
    fn test_no_match_rates_dont_cross() {
        let lender = ONQuote {
            venue: OvernightVenue::FedFundsON,
            agent: AgentId(Uuid::from_u128(1)),
            side: ONQuoteSide::Lend,
            notional: 1_000_000.0,
            limit_rate_bps: dec!(550),
            haircut: None,
            preferred_collateral: None,
            min_fill: 100_000.0,
            ts: 0,
        };

        let borrower = ONQuote {
            venue: OvernightVenue::FedFundsON,
            agent: AgentId(Uuid::from_u128(2)),
            side: ONQuoteSide::Borrow,
            notional: 1_000_000.0,
            limit_rate_bps: dec!(500),
            haircut: None,
            preferred_collateral: None,
            min_fill: 100_000.0,
            ts: 0,
        };

        let result = clear_fedfunds(vec![lender, borrower]);

        assert_eq!(result.matches.len(), 0);
        assert_eq!(result.unmatched_offers.len(), 1);
        assert_eq!(result.unmatched_bids.len(), 1);
    }

    #[test]
    fn test_partial_fills() {
        let lender = ONQuote {
            venue: OvernightVenue::FedFundsON,
            agent: AgentId(Uuid::from_u128(1)),
            side: ONQuoteSide::Lend,
            notional: 2_000_000.0,
            limit_rate_bps: dec!(500),
            haircut: None,
            preferred_collateral: None,
            min_fill: 100_000.0,
            ts: 0,
        };

        let borrower1 = ONQuote {
            venue: OvernightVenue::FedFundsON,
            agent: AgentId(Uuid::from_u128(2)),
            side: ONQuoteSide::Borrow,
            notional: 800_000.0,
            limit_rate_bps: dec!(525),
            haircut: None,
            preferred_collateral: None,
            min_fill: 100_000.0,
            ts: 0,
        };

        let borrower2 = ONQuote {
            venue: OvernightVenue::FedFundsON,
            agent: AgentId(Uuid::from_u128(3)),
            side: ONQuoteSide::Borrow,
            notional: 800_000.0,
            limit_rate_bps: dec!(510),
            haircut: None,
            preferred_collateral: None,
            min_fill: 100_000.0,
            ts: 0,
        };

        let result = clear_fedfunds(vec![lender, borrower1, borrower2]);

        assert_eq!(result.matches.len(), 2);
        assert_eq!(result.clearing_stats.total_volume, 1_600_000.0);

        assert_eq!(result.matches[0].rate_bps, dec!(525));
        assert_eq!(result.matches[0].amount, 800_000.0);

        assert_eq!(result.matches[1].rate_bps, dec!(510));
        assert_eq!(result.matches[1].amount, 800_000.0);
    }
}
