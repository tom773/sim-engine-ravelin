use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::{*, types::*};
use uuid::Uuid;
use chrono::Utc;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialSystem {
    pub instruments: HashMap<InstrumentId, FinancialInstrument>,
    pub balance_sheets: HashMap<AgentId, BalanceSheet>,
    pub central_bank: CentralBank,
    pub commercial_banks: HashMap<AgentId, Bank>,
}

impl FinancialSystem {
    pub fn create_instrument(&mut self, instrument: FinancialInstrument) -> Result<(), String> {
        let id = instrument.id.clone();

        if let Some(creditor_bs) = self.balance_sheets.get_mut(&instrument.creditor) {
            creditor_bs.assets.insert(id.clone(), instrument.clone());
        } else {
            return Err("Creditor not found".to_string());
        }
        if let Some(debtor_bs) = self.balance_sheets.get_mut(&instrument.debtor) {
            debtor_bs.liabilities.insert(id.clone(), instrument.clone());
        } else {
            if let Some(creditor_bs) = self.balance_sheets.get_mut(&instrument.creditor) {
                creditor_bs.assets.remove(&id);
            }
            return Err("Debtor not found".to_string());
        }
        self.instruments.insert(id, instrument);
        Ok(())
    }

    pub fn transfer_instrument(
        &mut self, 
        instrument_id: &InstrumentId, 
        new_creditor: AgentId
    ) -> Result<(), String> {
        let instrument = self.instruments.get_mut(instrument_id)
            .ok_or("Instrument not found")?;
        
        let old_creditor = instrument.creditor.clone();
        
        if let Some(old_bs) = self.balance_sheets.get_mut(&old_creditor) {
            old_bs.assets.remove(instrument_id);
        }
        
        instrument.creditor = new_creditor.clone();
        if let Some(new_bs) = self.balance_sheets.get_mut(&new_creditor) {
            new_bs.assets.insert(instrument_id.clone(), instrument.clone());
        }
        
        Ok(())
    }

    pub fn process_payment(
        &mut self,
        payer: &AgentId,
        payee: &AgentId,
        amount: f64,
    ) -> Transaction {
        let tx = Transaction::new(
            TransactionType::CashDeposit {
                holder: payer.clone(),
                bank: AgentId(Uuid::new_v4()),
                amount,
            },
            amount,
            payer.clone(),
            payee.clone(),
            Utc::now(),
        );
        tx 
        
    }
}

impl Default for FinancialSystem {
    fn default() -> Self {
        let id = AgentId(Uuid::new_v4());
        Self {
            instruments: HashMap::new(),
            balance_sheets: HashMap::new(),
            central_bank: CentralBank {
                id: id.clone(),
                policy_rate: 500.0,
                reserve_requirement: 0.1,
            },
            commercial_banks: HashMap::new(),
        }
    }
}