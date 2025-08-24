use crate::*;
use super::*;
use chrono::NaiveDate;
use uuid::Uuid;

impl BalanceSheetQuery for FinancialSystem {
    fn get_bs_by_id(&self, agent_id: &AgentId) -> Option<&BalanceSheet> { self.balance_sheets.get(agent_id) }
    fn get_bs_mut_by_id(&mut self, agent_id: &AgentId) -> Option<&mut BalanceSheet> { self.balance_sheets.get_mut(agent_id) }
    fn get_total_assets(&self, agent_id: &AgentId) -> f64 { self.balance_sheets.get(agent_id).map_or(0.0, |bs| bs.total_assets()) }
    fn get_cash_assets(&self, agent_id: &AgentId) -> f64 {
        self.get_bs_by_id(agent_id).map_or(0.0, |bs| {
            bs.assets.values()
                .filter(|inst| inst.details.as_any().is::<CashDetails>())
                .map(|inst| inst.principal).sum()
        })
    }
    fn get_total_liabilities(&self, agent_id: &AgentId) -> f64 { self.balance_sheets.get(agent_id).map_or(0.0, |bs| bs.total_liabilities()) }
    fn get_liquid_assets(&self, agent_id: &AgentId) -> f64 { self.balance_sheets.get(agent_id).map_or(0.0, |bs| bs.liquid_assets()) }
    fn get_deposits_at_bank(&self, agent_id: &AgentId, bank_id: &AgentId) -> f64 { self.balance_sheets.get(agent_id).map_or(0.0, |bs| bs.deposits_at_bank(bank_id)) }
    fn liquidity(&self, agent_id: &AgentId) -> f64 { self.balance_sheets.get(agent_id).map_or(0.0, |bs| bs.liquid_assets()) }
    fn get_total_deposits(&self, agent_id: &AgentId) -> f64 { self.balance_sheets.get(agent_id).map_or(0.0, |bs| bs.total_deposits()) }
    fn get_bank_reserves(&self, agent_id: &AgentId) -> Option<f64> {
        self.balance_sheets.get(agent_id).map(|bs| {
            bs.assets.values()
                .filter(|inst| inst.details.as_any().is::<CentralBankReservesDetails>() || inst.details.as_any().is::<CashDetails>())
                .map(|inst| inst.principal).sum()
        })
    }
}

impl InstrumentManager for FinancialSystem {
    fn create_instrument(&mut self, instrument: FinancialInstrument) -> Result<(), String> {
        let id = instrument.id;

        self.balance_sheets
            .get_mut(&instrument.creditor)
            .ok_or("Creditor not found")?
            .assets
            .insert(id, instrument.clone());

        self.balance_sheets
            .get_mut(&instrument.debtor)
            .ok_or("Debtor not found")?
            .liabilities
            .insert(id, instrument.clone());

        self.instruments.insert(id, instrument);
        Ok(())
    }

    fn transfer_instrument(&mut self, instrument_id: &InstrumentId, new_creditor: AgentId) -> Result<(), String> {
        let instrument = self.instruments.get_mut(instrument_id).ok_or("Instrument not found")?;
        let old_creditor = instrument.creditor;

        self.balance_sheets.get_mut(&old_creditor).ok_or("Old creditor not found")?.assets.remove(instrument_id);

        instrument.creditor = new_creditor;
        self.balance_sheets
            .get_mut(&new_creditor)
            .ok_or("New creditor not found")?
            .assets
            .insert(*instrument_id, instrument.clone());

        Ok(())
    }
    fn find_consolidatable_instrument(&self, new_inst: &FinancialInstrument) -> Option<InstrumentId> {
        if let Some(key) = new_inst.consolidation_key() {
            if let Some(creditor_bs) = self.balance_sheets.get(&new_inst.creditor) {
                for (id, existing) in &creditor_bs.assets {
                    if existing.consolidation_key() == Some(key.clone()) {
                        return Some(*id);
                    }
                }
            }
        }
        None
    }

    fn create_or_consolidate_instrument(&mut self, instrument: FinancialInstrument) -> Result<InstrumentId, String> {
        if let Some(existing_id) = self.find_consolidatable_instrument(&instrument) {
            let principal_change = instrument.principal;
            let existing =
                self.instruments.get_mut(&existing_id).ok_or("Consolidatable instrument not found in main registry")?;
            existing.principal += principal_change;

            self.balance_sheets
                .get_mut(&existing.creditor)
                .and_then(|bs| bs.assets.get_mut(&existing_id))
                .map(|inst| inst.principal += principal_change);
            self.balance_sheets
                .get_mut(&existing.debtor)
                .and_then(|bs| bs.liabilities.get_mut(&existing_id))
                .map(|inst| inst.principal += principal_change);

            Ok(existing_id)
        } else {
            let id = instrument.id;
            self.create_instrument(instrument)?;
            Ok(id)
        }
    }

    fn update_instrument(&mut self, id: &InstrumentId, new_principal: f64) -> Result<(), String> {
        let instrument = self.instruments.get_mut(id).ok_or("Instrument not found")?;
        instrument.principal = new_principal;

        self.balance_sheets
            .get_mut(&instrument.creditor)
            .and_then(|bs| bs.assets.get_mut(id))
            .map(|inst| inst.principal = new_principal);
        self.balance_sheets
            .get_mut(&instrument.debtor)
            .and_then(|bs| bs.liabilities.get_mut(id))
            .map(|inst| inst.principal = new_principal);

        Ok(())
    }

    fn remove_instrument(&mut self, id: &InstrumentId) -> Result<(), String> {
        if let Some(instrument) = self.instruments.remove(id) {
            self.balance_sheets.get_mut(&instrument.creditor).and_then(|bs| bs.assets.remove(id));
            self.balance_sheets.get_mut(&instrument.debtor).and_then(|bs| bs.liabilities.remove(id));
            Ok(())
        } else {
            Err("Instrument not found".to_string())
        }
    }

    fn swap_instrument(
        &mut self, id: &InstrumentId, new_debtor: &AgentId, new_creditor: &AgentId,
    ) -> Result<(), String> {
        let instrument = self.instruments.get_mut(id).ok_or("Instrument not found")?;
        let old_debtor = instrument.debtor;
        let old_creditor = instrument.creditor;

        instrument.debtor = *new_debtor;
        instrument.creditor = *new_creditor;

        if let Some(liability) = self.balance_sheets.get_mut(&old_debtor).and_then(|bs| bs.liabilities.remove(id)) {
            self.balance_sheets.get_mut(new_debtor).and_then(|bs| bs.liabilities.insert(*id, liability));
        }

        if let Some(asset) = self.balance_sheets.get_mut(&old_creditor).and_then(|bs| bs.assets.remove(id)) {
            self.balance_sheets.get_mut(new_creditor).and_then(|bs| bs.assets.insert(*id, asset));
        }

        Ok(())
    }
    fn split_and_transfer_instrument(
        &mut self, instrument_id: &InstrumentId, buyer: AgentId, quantity_to_transfer: u64,
    ) -> Result<InstrumentId, String> {
        let seller_instrument = self.instruments.get(instrument_id).ok_or("Instrument not found")?.clone();

        let bond_details =
            seller_instrument.details.as_any().downcast_ref::<BondDetails>().ok_or("Instrument is not a bond")?;

        if bond_details.quantity < quantity_to_transfer {
            return Err(format!(
                "Insufficient bond quantity: have {}, need {}",
                bond_details.quantity, quantity_to_transfer
            ));
        }

        let seller = seller_instrument.creditor;
        let remaining_quantity = bond_details.quantity - quantity_to_transfer;
        let principal_per_bond = seller_instrument.principal / bond_details.quantity as f64;
        let transfer_principal = principal_per_bond * quantity_to_transfer as f64;
        let remaining_principal = seller_instrument.principal - transfer_principal;

        if remaining_quantity == 0 {
            self.remove_instrument(instrument_id)?;
        } else {
            let updated_instrument =
                self.instruments.get_mut(instrument_id).ok_or("Instrument not found for update")?;
            updated_instrument.principal = remaining_principal;

            if let Some(updated_details) = updated_instrument.details.as_any_mut().downcast_mut::<BondDetails>() {
                updated_details.quantity = remaining_quantity;
            }

            if let Some(seller_bs) = self.balance_sheets.get_mut(&seller) {
                if let Some(asset) = seller_bs.assets.get_mut(instrument_id) {
                    asset.principal = remaining_principal;
                    if let Some(asset_details) = asset.details.as_any_mut().downcast_mut::<BondDetails>() {
                        asset_details.quantity = remaining_quantity;
                    }
                }
            }

            let debtor = updated_instrument.debtor;
            if let Some(debtor_bs) = self.balance_sheets.get_mut(&debtor) {
                if let Some(liability) = debtor_bs.liabilities.get_mut(instrument_id) {
                    liability.principal = remaining_principal;
                    if let Some(liability_details) = liability.details.as_any_mut().downcast_mut::<BondDetails>() {
                        liability_details.quantity = remaining_quantity;
                    }
                }
            }
        }

        let mut buyer_bond_details = bond_details.clone();
        buyer_bond_details.quantity = quantity_to_transfer;

        let buyer_instrument = FinancialInstrument {
            id: InstrumentId(Uuid::new_v4()),
            creditor: buyer,
            debtor: seller_instrument.debtor,
            principal: transfer_principal,
            details: Box::new(buyer_bond_details),
            originated_date: seller_instrument.originated_date,
            accrued_interest: (seller_instrument.accrued_interest / bond_details.quantity as f64)
                * quantity_to_transfer as f64,
            last_accrual_date: seller_instrument.last_accrual_date,
        };

        self.create_or_consolidate_instrument(buyer_instrument)
    }
    fn pay_interest(&mut self, instrument_id: InstrumentId, payment_date: NaiveDate) -> Result<(), String> {
        let instrument = self.instruments.get_mut(&instrument_id).ok_or("Instrument not found")?;
        let bond_details =
            instrument.details.as_any_mut().downcast_mut::<BondDetails>().ok_or("Instrument is not a bond")?;

        let interest_payment = bond_details.coupon_rate_bps * instrument.principal / 10000.0;

        instrument.accrued_interest += interest_payment;

        instrument.last_accrual_date = payment_date;

        if let Some(bs) = self.balance_sheets.get_mut(&instrument.creditor) {
            bs.assets.entry(instrument_id).and_modify(|inst| {
                inst.accrued_interest += interest_payment;
            });
        }

        Ok(())
    }
}