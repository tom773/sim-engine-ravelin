use crate::prelude::*;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConsolidationKey {
    pub issuer: AgentId,
    pub instrument_type: String,
    pub subtype: String,
}

impl Instrument {
    pub fn get_consolidation_key(&self) -> ConsolidationKey {
        match &self.instrument_type {
            InstrumentType::Cash(d) => ConsolidationKey {
                issuer: d.issuer,
                instrument_type: "Cash".to_string(),
                subtype: format!("{:?}", d.cash_type),
            },
            InstrumentType::Bond(d) => ConsolidationKey {
                issuer: d.issuer,
                instrument_type: "Bond".to_string(),
                subtype: format!(
                    "{:?}_{:?}_{:?}_{:?}",
                    d.bond_type,        // Bill/Note/Bond class
                    d.rating,           // Credit rating
                    d.maturity_date,    // Series maturity
                    d.coupon_rate_bps   // Coupon (bps); for zeros it’s 0
                ),
            },
            InstrumentType::RealAsset(d) => {
                let (owner, subtype) = match d {
                    RealAssetType::Inventory { owner, .. } => (*owner, "Inventory".to_string()),
                    RealAssetType::Property { owner, .. } => (*owner, "Property".to_string()),
                };
                ConsolidationKey {
                    issuer: owner,
                    instrument_type: "RealAsset".to_string(),
                    subtype,
                }
            }
            InstrumentType::Equity(d) => ConsolidationKey {
                issuer: d.issuer,
                instrument_type: "Equity".to_string(),
                subtype: "CommonStock".to_string(),
            },
            InstrumentType::Repo(d) => ConsolidationKey {
                issuer: d.borrower,
                instrument_type: "Repo".to_string(),
                subtype: "Repo".to_string(),
            },
            InstrumentType::Derivative(d) => ConsolidationKey {
                issuer: AgentId(Uuid::nil()),
                instrument_type: "Derivative".to_string(),
                subtype: format!("{:?}", d.underlying),
            },
            InstrumentType::StructuredTranche(d) => ConsolidationKey {
                issuer: d.issuer,
                instrument_type: "StructuredTranche".to_string(),
                subtype: format!("{:?}_{:?}", d.tranche_type, d.rating),
            },
        }
    }
}

impl FinancialSystem {
    pub fn create_or_consolidate_position(
        &mut self,
        creditor_id: &AgentId,
        debtor_id: &AgentId,
        instrument_id: &InstrumentId,
        quantity_change: f64,
        _book_value_change: f64,
    ) -> Result<(), String> {
        let creditor_bs = self
            .balance_sheets
            .get_mut(creditor_id)
            .ok_or("Creditor not found")?;
        let asset_pos = creditor_bs.assets.entry(*instrument_id).or_default();
        asset_pos.quantity += quantity_change;

        let debtor_bs = self
            .balance_sheets
            .get_mut(debtor_id)
            .ok_or("Debtor not found")?;
        let liability_pos = debtor_bs.liabilities.entry(*instrument_id).or_default();
        liability_pos.quantity += quantity_change;

        Ok(())
    }
}
