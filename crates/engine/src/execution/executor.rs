
use super::*;
use shared::*;

pub struct TransactionExecutor;

impl TransactionExecutor {
    pub fn execute_action(
        action: &SimAction,
        state: &SimState,
    ) -> ExecutionResult {
        match action {
            SimAction::IssueIncome { recipient, amount } => {
                Self::execute_issue_income(recipient, *amount, state)
            }
            
            SimAction::DepositCash { depositor, bank, amount } => {
                Self::execute_deposit_cash(depositor, bank, *amount, state)
            }
            
            SimAction::WithdrawCash { account_holder, bank, amount } => {
                Self::execute_withdraw_cash(account_holder, bank, *amount, state)
            }
            
            SimAction::Transfer { from, to, amount } => {
                Self::execute_transfer(from, to, *amount, state)
            }
            
            SimAction::Purchase { buyer, seller, good_id, amount } => {
                Self::execute_purchase(buyer, seller, good_id, *amount, state)
            }
            
            SimAction::UpdateReserves { bank, amount_change } => {
                Self::execute_update_reserves(bank, *amount_change, state)
            }
        }
    }
    
    pub fn apply_effects(effects: &[StateEffect], state: &mut SimState) -> Result<(), String> {
        for effect in effects {
            match effect {
                StateEffect::CreateInstrument(instrument) => {
                    state.financial_system.create_instrument(instrument.clone())?;
                }
                
                StateEffect::UpdateInstrument { id, new_principal } => {
                    if let Some(instrument) = state.financial_system.instruments.get_mut(id) {
                        let old_principal = instrument.principal;
                        instrument.principal = *new_principal;
                        
                        if let Some(creditor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.creditor) {
                            if let Some(asset) = creditor_bs.assets.get_mut(id) {
                                asset.principal = *new_principal;
                            }
                        }
                        
                        if let Some(debtor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.debtor) {
                            if let Some(liability) = debtor_bs.liabilities.get_mut(id) {
                                liability.principal = *new_principal;
                            }
                        }
                        
                        println!("Updated instrument {} from ${:.2} to ${:.2}", 
                            id.0, old_principal, new_principal);
                    }
                }
                
                StateEffect::TransferInstrument { id, new_creditor } => {
                    state.financial_system.transfer_instrument(id, new_creditor.clone())?;
                }
                
                StateEffect::RemoveInstrument(id) => {
                    if let Some(instrument) = state.financial_system.instruments.remove(id) {
                        if let Some(creditor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.creditor) {
                            creditor_bs.assets.remove(id);
                        }
                        if let Some(debtor_bs) = state.financial_system.balance_sheets.get_mut(&instrument.debtor) {
                            debtor_bs.liabilities.remove(id);
                        }
                    }
                }
                
                StateEffect::RecordTransaction(tx) => {
                    println!("Transaction recorded: {:?}", tx);
                }
                
                _ => {
                }
            }
        }
        Ok(())
    }
    
    
    fn execute_issue_income(
        recipient: &AgentId,
        amount: f64,
        state: &SimState,
    ) -> ExecutionResult {
        let mut effects = vec![];
        
        let cash = cash!(
            recipient.clone(),
            amount,
            state.financial_system.central_bank.id.clone(),
            state.ticknum
        );
        effects.push(StateEffect::CreateInstrument(cash));
        
        use chrono::Utc;
        let tx = Transaction::new(
            TransactionType::CashDeposit {
                holder: recipient.clone(),
                bank: state.financial_system.central_bank.id.clone(),
                amount,
            },
            amount,
            state.financial_system.central_bank.id.clone(),
            recipient.clone(),
            Utc::now(),
        );
        effects.push(StateEffect::RecordTransaction(tx));

        ExecutionResult {
            success: true,
            effects,
            errors: vec![],
        }
    }
    
    fn execute_deposit_cash(
        depositor: &AgentId,
        bank: &AgentId,
        amount: f64,
        state: &SimState,
    ) -> ExecutionResult {
        let mut effects = vec![];
        
        let cash_holdings = state.financial_system.get_cash_assets(depositor);
            
        if cash_holdings < amount {
            return ExecutionResult {
                success: false,
                effects: vec![],
                errors: vec![format!("Insufficient cash: ${:.2} < ${:.2}", cash_holdings, amount)],
            };
        }
        
        if let Some(depositor_bs) = state.financial_system.balance_sheets.get(depositor) {
            if let Some((cash_id, cash_inst)) = depositor_bs.assets.iter()
                .find(|(_, inst)| matches!(inst.instrument_type, InstrumentType::Cash) && inst.principal >= amount) 
            {
                if cash_inst.principal > amount {
                    effects.push(StateEffect::UpdateInstrument {
                        id: cash_id.clone(),
                        new_principal: cash_inst.principal - amount,
                    });
                } else {
                    effects.push(StateEffect::RemoveInstrument(cash_id.clone()));
                }
                let deposit = deposit!(
                    depositor.clone(),
                    bank.clone(),
                    amount,
                    state.financial_system.central_bank.policy_rate - 200.0,
                    state.ticknum
                );
                effects.push(StateEffect::CreateInstrument(deposit));
                
                let bank_cash = cash!(
                    bank.clone(),
                    amount,
                    state.financial_system.central_bank.id.clone(),
                    state.ticknum
                );
                effects.push(StateEffect::CreateInstrument(bank_cash));
                
                let reserve_requirement = state.financial_system.central_bank.reserve_requirement;
                let required_reserves = amount * reserve_requirement;
                let excess_reserves = amount * (1.0 - reserve_requirement);

                let reserves = reserves!(
                    bank.clone(),
                    state.financial_system.central_bank.id.clone(),
                    required_reserves,
                    state.financial_system.central_bank.policy_rate - 50.0,
                    state.ticknum
                );

                effects.push(StateEffect::CreateInstrument(reserves));
                
                use chrono::Utc;
                let tx = Transaction::new(
                    TransactionType::CashDeposit {
                        holder: depositor.clone(),
                        bank: bank.clone(),
                        amount,
                    },
                    amount,
                    depositor.clone(),
                    bank.clone(),
                    Utc::now(),
                );
                effects.push(StateEffect::RecordTransaction(tx));
            }
        }
        
        let success = !effects.is_empty();
        ExecutionResult {
            success,
            effects,
            errors: if !success { vec!["Failed to process deposit".to_string()] } else { vec![] },
        }
    }
    
    fn execute_withdraw_cash(
        account_holder: &AgentId,
        bank: &AgentId,
        amount: f64,
        state: &SimState,
    ) -> ExecutionResult {
        let mut effects = vec![];
        
        let deposits = state.financial_system.get_deposits_at_bank(account_holder, bank);
        
        if deposits < amount {
            return ExecutionResult {
                success: false,
                effects: vec![],
                errors: vec![format!("Insufficient deposits: ${:.2} < ${:.2}", deposits, amount)],
            };
        }
        
        let bank_liquidity = state.financial_system.liquidity(bank);
        if bank_liquidity < amount {
            return ExecutionResult {
                success: false,
                effects: vec![],
                errors: vec![format!("Bank has insufficient liquidity for withdrawal")],
            };
        }
        
        if let Some(account_bs) = state.financial_system.balance_sheets.get(account_holder) {
            if let Some((deposit_id, deposit)) = account_bs.assets.iter()
                .find(|(_, inst)| inst.debtor == *bank && 
                    matches!(inst.instrument_type, InstrumentType::DemandDeposit) &&
                    inst.principal >= amount)
            {
                if deposit.principal > amount {
                    effects.push(StateEffect::UpdateInstrument {
                        id: deposit_id.clone(),
                        new_principal: deposit.principal - amount,
                    });
                } else {
                    effects.push(StateEffect::RemoveInstrument(deposit_id.clone()));
                }
                
                let cash = cash!(
                    account_holder.clone(),
                    amount,
                    state.financial_system.central_bank.id.clone(),
                    state.ticknum
                );
                effects.push(StateEffect::CreateInstrument(cash));
                
                
                use chrono::Utc;
                let tx = Transaction::new(
                    TransactionType::CashWithdrawal {
                        holder: account_holder.clone(),
                        bank: bank.clone(),
                        amount,
                    },
                    amount,
                    bank.clone(),
                    account_holder.clone(),
                    Utc::now(),
                );
                effects.push(StateEffect::RecordTransaction(tx));
            }
        }
        
        let success = !effects.is_empty();
        ExecutionResult {
            success,
            effects,
            errors: if !success { vec!["Failed to process withdrawal".to_string()] } else { vec![] },
        }
    }
    
    fn execute_transfer(
        from: &AgentId,
        to: &AgentId,
        amount: f64,
        state: &SimState,
    ) -> ExecutionResult {
        let mut effects = vec![];
        
        let payer_liquidity = state.financial_system.liquidity(from);
        if payer_liquidity < amount {
            return ExecutionResult {
                success: false,
                effects: vec![],
                errors: vec![format!("Insufficient funds: ${:.2} < ${:.2}", payer_liquidity, amount)],
            };
        }
        
        if let Some(from_bs) = state.financial_system.balance_sheets.get(from) {
            if let Some((instrument_id, deposit)) = from_bs.assets.iter()
                .find(|(_, inst)| matches!(inst.instrument_type, InstrumentType::DemandDeposit) && 
                    inst.principal >= amount) 
            {
                let from_bank = deposit.debtor.clone();
                
                if deposit.principal > amount {
                    effects.push(StateEffect::UpdateInstrument {
                        id: instrument_id.clone(),
                        new_principal: deposit.principal - amount,
                    });
                } else {
                    effects.push(StateEffect::RemoveInstrument(instrument_id.clone()));
                }
                let new_deposit = deposit!(
                    to.clone(),
                    from_bank,
                    amount,
                    state.financial_system.central_bank.policy_rate - 200.0,
                    state.ticknum
                );
                effects.push(StateEffect::CreateInstrument(new_deposit));
                
            }
        }
        
        use chrono::Utc;
        let tx = Transaction::new(
            TransactionType::CashDeposit {
                holder: to.clone(),
                bank: AgentId(Uuid::new_v4()), // TODO: Get actual bank
                amount,
            },
            amount,
            from.clone(),
            to.clone(),
            Utc::now(),
        );
        effects.push(StateEffect::RecordTransaction(tx));
        
        let success = !effects.is_empty();
        ExecutionResult {
            success,
            effects,
            errors: if !success { vec!["Failed to execute transfer".to_string()] } else { vec![] },
        }
    }
    
    fn execute_purchase(
        buyer: &AgentId,
        seller: &AgentId,
        good_id: &str,
        amount: f64,
        state: &SimState,
    ) -> ExecutionResult {
        Self::execute_transfer(buyer, seller, amount, state)
    }
    
    fn execute_update_reserves(
        bank: &AgentId,
        amount_change: f64,
        state: &SimState,
    ) -> ExecutionResult {
        let mut effects = vec![];
        
        if let Some(bank_bs) = state.financial_system.balance_sheets.get(bank) {
            if let Some((reserve_id, reserve)) = bank_bs.assets.iter()
                .find(|(_, inst)| matches!(inst.instrument_type, InstrumentType::CentralBankReserves))
            {
                let new_amount = (reserve.principal + amount_change).max(0.0);
                effects.push(StateEffect::UpdateInstrument {
                    id: reserve_id.clone(),
                    new_principal: new_amount,
                });
            } else if amount_change > 0.0 {
                let reserves = reserves!(
                    bank.clone(),
                    state.financial_system.central_bank.id.clone(),
                    amount_change,
                    state.financial_system.central_bank.policy_rate - 50.0,
                    state.ticknum
                );
                effects.push(StateEffect::CreateInstrument(reserves));
            }
        }
        
        let success = !effects.is_empty();
        ExecutionResult {
            success,
            effects,
            errors: if !success { vec!["Failed to update reserves".to_string()] } else { vec![] },
        }
    }
}