use super::*;
use shared::*;

pub struct TransactionExecutor;

impl TransactionExecutor {
    pub fn execute_action(action: &SimAction, state: &SimState) -> ExecutionResult {
        match action {
            SimAction::IssueIncome { agent_id, amount } => {
                Self::execute_issue_income(agent_id, *amount, state)
            }

            SimAction::DepositCash {
                agent_id,
                bank,
                amount,
            } => Self::execute_deposit_cash(agent_id, bank, *amount, state),

            SimAction::WithdrawCash {
                agent_id,
                bank,
                amount,
            } => Self::execute_withdraw_cash(agent_id, bank, *amount, state),

            SimAction::Transfer {
                agent_id: _,
                from,
                to,
                amount,
            } => Self::execute_transfer(from, to, *amount, state),

            SimAction::Purchase {
                agent_id,
                seller,
                good_id,
                amount,
            } => Self::execute_purchase(agent_id, seller, good_id, *amount, state),

            SimAction::UpdateReserves {
                bank,
                amount_change,
            } => Self::execute_update_reserves(bank, *amount_change, state),
            SimAction::Hire { agent_id, count } => Self::execute_hire(agent_id, *count),
            SimAction::Produce {
                agent_id,
                good_id,
                amount,
            } => Self::execute_produce(agent_id, *amount, good_id),
        }
    }

    pub fn apply_effects(effects: &[StateEffect], state: &mut SimState) -> Result<(), String> {
        for effect in effects {
            match effect {
                StateEffect::CreateInstrument(instrument) => {
                    let should_consolidate = matches!(
                        instrument.instrument_type,
                        InstrumentType::Cash | InstrumentType::CentralBankReserves
                    );

                    if should_consolidate {
                        let existing_instrument_id = state
                            .financial_system
                            .balance_sheets
                            .get(&instrument.creditor)
                            .and_then(|bs| {
                                bs.assets
                                    .iter()
                                    .find(|(_, inst)| {
                                        inst.instrument_type == instrument.instrument_type
                                            && inst.debtor == instrument.debtor
                                            && inst.creditor == instrument.creditor
                                    })
                                    .map(|(id, _)| id.clone())
                            });

                        if let Some(inst_id) = existing_instrument_id {
                            if let Some(main_inst) =
                                state.financial_system.instruments.get_mut(&inst_id)
                            {
                                main_inst.principal += instrument.principal;
                            }

                            if let Some(creditor_bs) = state
                                .financial_system
                                .balance_sheets
                                .get_mut(&instrument.creditor)
                            {
                                if let Some(asset) = creditor_bs.assets.get_mut(&inst_id) {
                                    asset.principal += instrument.principal;
                                }
                            }

                            if let Some(debtor_bs) = state
                                .financial_system
                                .balance_sheets
                                .get_mut(&instrument.debtor)
                            {
                                if let Some(liability) = debtor_bs.liabilities.get_mut(&inst_id) {
                                    liability.principal += instrument.principal;
                                }
                            }
                        } else {
                            state
                                .financial_system
                                .create_instrument(instrument.clone())?;
                        }
                    } else {
                        state
                            .financial_system
                            .create_instrument(instrument.clone())?;
                    }
                }

                StateEffect::UpdateInstrument { id, new_principal } => {
                    if let Some(instrument) = state.financial_system.instruments.get_mut(id) {
                        let old_principal = instrument.principal;
                        instrument.principal = *new_principal;

                        if let Some(creditor_bs) = state
                            .financial_system
                            .balance_sheets
                            .get_mut(&instrument.creditor)
                        {
                            if let Some(asset) = creditor_bs.assets.get_mut(id) {
                                asset.principal = *new_principal;
                            }
                        }

                        if let Some(debtor_bs) = state
                            .financial_system
                            .balance_sheets
                            .get_mut(&instrument.debtor)
                        {
                            if let Some(liability) = debtor_bs.liabilities.get_mut(id) {
                                liability.principal = *new_principal;
                            }
                        }

                        println!(
                            "Updated instrument {} from ${:.2} to ${:.2}",
                            id.0, old_principal, new_principal
                        );
                    }
                }

                StateEffect::TransferInstrument { id, new_creditor } => {
                    state
                        .financial_system
                        .transfer_instrument(id, new_creditor.clone())?;
                }

                StateEffect::RemoveInstrument(id) => {
                    if let Some(instrument) = state.financial_system.instruments.remove(id) {
                        if let Some(creditor_bs) = state
                            .financial_system
                            .balance_sheets
                            .get_mut(&instrument.creditor)
                        {
                            creditor_bs.assets.remove(id);
                        }
                        if let Some(debtor_bs) = state
                            .financial_system
                            .balance_sheets
                            .get_mut(&instrument.debtor)
                        {
                            debtor_bs.liabilities.remove(id);
                        }
                    }
                }
                StateEffect::SwapInstrument {
                    id,
                    new_debtor,
                    new_creditor,
                } => {
                    if let Some(instrument) = state.financial_system.instruments.get_mut(id) {
                        let old_debtor = instrument.debtor.clone();
                        instrument.debtor = new_debtor.clone();
                        instrument.creditor = new_creditor.clone();

                        if let Some(old_bs) =
                            state.financial_system.balance_sheets.get_mut(&old_debtor)
                        {
                            old_bs.liabilities.remove(id);
                        }
                        if let Some(new_bs) =
                            state.financial_system.balance_sheets.get_mut(&new_creditor)
                        {
                            new_bs.assets.insert(id.clone(), instrument.clone());
                        }
                    }
                }
                StateEffect::RecordTransaction(tx) => {
                    println!("Transaction recorded: {:?}", tx);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn execute_hire(firm: &AgentId, count: u32) -> ExecutionResult {
        let mut effects = vec![];

        effects.push(StateEffect::Hire {
            firm: firm.clone(),
            count: count,
        });

        ExecutionResult {
            success: true,
            effects,
            errors: vec![],
        }
    }
    fn execute_produce(firm: &AgentId, amount: f64, good_id: &GoodId) -> ExecutionResult {
        let mut effects = vec![];

        effects.push(StateEffect::Produce {
            firm: firm.clone(),
            good_id: good_id.clone(),
            amount,
        });

        ExecutionResult {
            success: true,
            effects,
            errors: vec![],
        }
    }
    fn execute_issue_income(recipient: &AgentId, amount: f64, state: &SimState) -> ExecutionResult {
        let mut effects = vec![];

        let cash = cash!(
            recipient.clone(),
            amount,
            state.financial_system.central_bank.id.clone(),
            state.ticknum
        );
        effects.push(StateEffect::CreateInstrument(cash));

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
                errors: vec![format!(
                    "Insufficient cash: ${:.2} < ${:.2}",
                    cash_holdings, amount
                )],
            };
        }

        if let Some(depositor_bs) = state.financial_system.balance_sheets.get(depositor) {
            if let Some((cash_id, cash_inst)) = depositor_bs.assets.iter().find(|(_, inst)| {
                matches!(inst.instrument_type, InstrumentType::Cash) && inst.principal >= amount
            }) {
                // TODO Partial transactions are going to be constant - own function?
                if cash_inst.principal == amount {
                    // Transfer the entire cash instrument to the bank
                    effects.push(StateEffect::TransferInstrument {
                        id: cash_id.clone(),
                        new_creditor: bank.clone(),
                    });
                } else {
                    // Partial transfer: reduce depositor's cash and transfer the difference to bank
                    effects.push(StateEffect::UpdateInstrument {
                        id: cash_id.clone(),
                        new_principal: cash_inst.principal - amount,
                    });

                    // Create new cash for bank (this is the transferred portion)
                    let bank_cash = cash!(
                        bank.clone(),
                        amount,
                        state.financial_system.central_bank.id.clone(),
                        state.ticknum
                    );
                    effects.push(StateEffect::CreateInstrument(bank_cash));
                }

                // Can stay
                let deposit = deposit!(
                    depositor.clone(),
                    bank.clone(),
                    amount,
                    state.financial_system.central_bank.policy_rate - 200.0,
                    state.ticknum
                );
                effects.push(StateEffect::CreateInstrument(deposit));

                // TODO: Seperate into own function to check reserve requirements
                let reserve_requirement = state.financial_system.central_bank.reserve_requirement;
                let required_reserves_for_deposit = amount * reserve_requirement;
                let current_reserves = state.financial_system
                    .get_bank_reserves(bank)
                    .unwrap_or(0.0);

                let total_deposits_after = state.financial_system.get_total_liabilities(bank)+
                    amount;
                let total_required_reserves = total_deposits_after * reserve_requirement;

                if current_reserves < total_required_reserves {
                    let reserve_shortfall = total_required_reserves - current_reserves;
                    // TODO: First financial market tx should be to post a bid for overnight reserves
                    // IDEA: If bank has excess reserves it can post an ask for overnight reserves
                    let reserves = reserves!(
                        bank.clone(),
                        state.financial_system.central_bank.id.clone(),
                        reserve_shortfall,
                        state.financial_system.central_bank.policy_rate - 50.0,
                        state.ticknum
                    );
                    effects.push(StateEffect::CreateInstrument(reserves));
                }
            }
        }

        let success = !effects.is_empty();
        ExecutionResult {
            success,
            effects,
            errors: if !success {
                vec!["Failed to process deposit".to_string()]
            } else {
                vec![]
            },
        }
    }

    fn execute_withdraw_cash(
        account_holder: &AgentId,
        bank: &AgentId,
        amount: f64,
        state: &SimState,
    ) -> ExecutionResult {
        let mut effects = vec![];

        let deposits = state
            .financial_system
            .get_deposits_at_bank(account_holder, bank);

        if deposits < amount {
            return ExecutionResult {
                success: false,
                effects: vec![],
                errors: vec![format!(
                    "Insufficient deposits: ${:.2} < ${:.2}",
                    deposits, amount
                )],
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
            if let Some((deposit_id, deposit)) = account_bs.assets.iter().find(|(_, inst)| {
                inst.debtor == *bank
                    && matches!(inst.instrument_type, InstrumentType::DemandDeposit)
                    && inst.principal >= amount
            }) {
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
            }
        }

        let success = !effects.is_empty();
        ExecutionResult {
            success,
            effects,
            errors: if !success {
                vec!["Failed to process withdrawal".to_string()]
            } else {
                vec![]
            },
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
                errors: vec![format!(
                    "Insufficient funds: ${:.2} < ${:.2}",
                    payer_liquidity, amount
                )],
            };
        }

        if let Some(from_bs) = state.financial_system.balance_sheets.get(from) {
            if let Some((instrument_id, deposit)) = from_bs.assets.iter().find(|(_, inst)| {
                matches!(inst.instrument_type, InstrumentType::DemandDeposit)
                    && inst.principal >= amount
            }) {
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

        let success = !effects.is_empty();
        ExecutionResult {
            success,
            effects,
            errors: if !success {
                vec!["Failed to execute transfer".to_string()]
            } else {
                vec![]
            },
        }
    }
    pub fn execute_inject_liquidity(state: &SimState) -> ExecutionResult {
        let mut effects = vec![];
        for consumer in &state.consumers {
            let cash = cash!(
                consumer.id.clone(),
                100.0,
                state.financial_system.central_bank.id.clone(),
                state.ticknum
            );
            effects.push(StateEffect::CreateInstrument(cash));
        }
        let success = !effects.is_empty();
        ExecutionResult {
            success,
            effects,
            errors: if !success {
                vec!["Failed to execute transfer".to_string()]
            } else {
                vec![]
            },
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
            if let Some((reserve_id, reserve)) = bank_bs.assets.iter().find(|(_, inst)| {
                matches!(inst.instrument_type, InstrumentType::CentralBankReserves)
            }) {
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
            errors: if !success {
                vec!["Failed to update reserves".to_string()]
            } else {
                vec![]
            },
        }
    }
}
