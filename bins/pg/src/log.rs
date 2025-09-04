use crate::*;
use std::collections::{HashMap, HashSet};


const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";

fn use_color() -> bool {
    std::env::var_os("NO_COLOR").is_none()
}

fn color_for(kind: &str) -> &'static str {
    match kind {
        "Bank" => "\x1b[38;5;39m",
        "Firm" => "\x1b[38;5;171m",
        "Consumer" => "\x1b[38;5;178m",
        "Government" => "\x1b[38;5;41m",
        "Central Bank" => "\x1b[38;5;33m",
        _ => "\x1b[38;5;250m",
    }
}

fn clr(kind: &str) -> &'static str {
    if use_color() { color_for(kind) } else { "" }
}

fn reset() -> &'static str {
    if use_color() { RESET } else { "" }
}

fn short_id(id: &AgentId) -> String {
    let s = id.to_string();
    s.chars().take(4).collect()
}



fn get_agent_display_info(state: &SimState, agent_id: &AgentId) -> (String, String) {
    if let Some(bank) = state.agents.banks.get(agent_id) {
        (bank.name.clone(), "Bank".to_string())
    } else if let Some(firm) = state.agents.firms.get(agent_id) {
        (firm.name.clone(), "Firm".to_string())
    } else if state.agents.consumers.contains_key(agent_id) {
        (
            format!("Consumer {}", &agent_id.to_string()[..4]),
            "Consumer".to_string(),
        )
    } else if agent_id == &state.financial_system.government.id {
        ("Government".to_string(), "Government".to_string())
    } else if agent_id == &state.financial_system.central_bank.id {
        ("Central Bank".to_string(), "Central Bank".to_string())
    } else {
        ("Unknown".to_string(), "Unknown".to_string())
    }
}
pub fn setup() -> SimState {
    let mut state = SimState::default();
    let cb_id = state.financial_system.central_bank.id;

    let bank_a = Bank::new("Bank A".to_string(), dec!(200), dec!(-50));
    let bank_b = Bank::new("Bank B".to_string(), dec!(220), dec!(-45));
    let consumer_1 = Consumer::new(30, bank_a.id, PersonalityArchetype::Balanced);
    let consumer_2 = Consumer::new(45, bank_b.id, PersonalityArchetype::Saver);

    for agent_id in [&bank_a.id, &bank_b.id, &consumer_1.id, &consumer_2.id] {
        state
            .financial_system
            .balance_sheets
            .insert(*agent_id, BalanceSheet::new(*agent_id));
    }
    state.agents.banks.insert(bank_a.id, bank_a.clone());
    state.agents.banks.insert(bank_b.id, bank_b.clone());
    state
        .agents
        .consumers
        .insert(consumer_1.id, consumer_1.clone());
    state
        .agents
        .consumers
        .insert(consumer_2.id, consumer_2.clone());

    let c1_deposit = Instrument::cash(
        InstrumentId(Uuid::new_v4()),
        bank_a.id,
        CashType::DemandDeposit,
        Currency::USD,
        dec!(50),
    )
    .build();
    create_and_register_instrument(
        &mut state,
        consumer_1.id,
        bank_a.id,
        c1_deposit,
        3000.0,
        1.0,
    )
    .unwrap();
    let c2_deposit = Instrument::cash(
        InstrumentId(Uuid::new_v4()),
        bank_b.id,
        CashType::DemandDeposit,
        Currency::USD,
        dec!(50),
    )
    .build();
    create_and_register_instrument(
        &mut state,
        consumer_2.id,
        bank_b.id,
        c2_deposit,
        2000.0,
        1.0,
    )
    .unwrap();
    let reserves_instrument = Instrument::cash(
        InstrumentId(Uuid::new_v4()),
        cb_id,
        CashType::CentralBankReserves,
        Currency::USD,
        dec!(425),
    )
    .build();
    create_and_register_instrument(
        &mut state,
        bank_a.id,
        cb_id,
        reserves_instrument.clone(),
        2000.0,
        1.0,
    )
    .unwrap();
    create_and_register_instrument(
        &mut state,
        bank_b.id,
        cb_id,
        reserves_instrument.clone(),
        2000.0,
        1.0,
    )
    .unwrap();

    state
}

fn create_and_register_instrument(
    state: &mut SimState,
    owner_id: AgentId,
    issuer_id: AgentId,
    instrument: Instrument,
    quantity: f64,
    book_value_per_unit: f64,
) -> Result<(), String> {
    state.financial_system.create_or_consolidate_instrument(
        owner_id,
        issuer_id,
        instrument,
        quantity,
        book_value_per_unit,
    )?;
    Ok(())
}


fn _log_positions(
    positions: &HashMap<InstrumentId, Position>,
    state: &SimState
) -> String {
    if positions.is_empty() {
        return "  (None)".to_string();
    }

    let mut lines = Vec::with_capacity(positions.len());

    for (instr_id, position) in positions {
        if let Some(info) =
            state
                .financial_system
                .get_instrument_info(instr_id, &state.agents, state.current_date)
        {
            let book_value = position.quantity * position.book_value_per_unit.to_f64();

            let details = match info.instrument_type {
                "Corporate Bond" | "Government Bond" => format!(
                    "Mat: {}, Cpn: {:.0}bps",
                    info.maturity_date.unwrap_or_default(),
                    info.coupon_rate_bps.unwrap_or_default()
                ),
                "Demand Deposit" | "Savings Deposit" | "Central Bank Reserves" => format!(
                    "Rate: {:.0}bps, Issuer: {}",
                    info.coupon_rate_bps.unwrap_or_default(),
                    info.issuer_name.as_deref().unwrap_or("N/A")
                ),
                _ => String::new(),
            };

            lines.push(format!(
                "  | {:<24} | ${:>10.2} | {}",
                info.instrument_type, book_value, details
            ));
        } else {
            lines.push(format!(
                "  | {:<24} | Error fetching info",
                instr_id.to_string()
            ));
        }
    }

    lines.join("\n")
}


pub fn _log_balance_sheets(state: &SimState) {
    println!("\n{:=<80}", "");
    println!("{:.^80}", " BALANCE SHEET SNAPSHOT ");
    println!("{:=<80}\n", "");

    let mut agent_ids: Vec<_> = state.financial_system.balance_sheets.keys().collect();
    agent_ids.sort_by_key(|id| get_agent_display_info(state, id).1);


    for agent_id in agent_ids {
        let bs = &state.financial_system.balance_sheets[agent_id];
        let (name, agent_type) = get_agent_display_info(state, agent_id);

        let color = clr(&agent_type);
        let r = reset();
        let id4 = short_id(agent_id);
        println!();
        println!("{color}{BOLD}■ {agent_type}: {name} [{id4}]{r}");
        println!("{color}{rule}{r}", rule = "─".repeat(80));

        let pos = _log_positions(&bs.assets, state);
        println!("{color}  Assets:\n{r}{pos}");

        println!(); // spacer
        let pos_l = _log_positions(&bs.liabilities, state);
        println!("{color}  Liabilities:\n{r}{pos_l}");

        println!("\n{:-<80}\n", "");
    }
}

pub fn log_balance_sheet_changes(before: &SimState, after: &SimState) {
    println!("\n{:=<80}", "");
    println!("{:.^80}", " BALANCE SHEET CHANGES ");
    println!("{:=<80}\n", "");

    let mut all_agent_ids: HashSet<_> = before.financial_system.balance_sheets.keys().collect();
    all_agent_ids.extend(after.financial_system.balance_sheets.keys());
    
    let mut sorted_ids: Vec<_> = all_agent_ids.into_iter().collect();
    sorted_ids.sort_by_key(|id| get_agent_display_info(after, id).1);

    for agent_id in sorted_ids {
        let before_bs = before.financial_system.balance_sheets.get(agent_id);
        let after_bs = after.financial_system.balance_sheets.get(agent_id);

        if before_bs == after_bs { continue; }

        let (name, agent_type) = get_agent_display_info(after, agent_id);
        let color = clr(&agent_type);
        let r = reset();
        let id4 = short_id(agent_id);
        println!("{color}{BOLD}■ {agent_type}: {name} [{id4}]{r}");

        log_position_changes("Assets", before_bs, after_bs, after, PositionSide::Asset);
        log_position_changes("Liabilities", before_bs, after_bs, after, PositionSide::Liability);
        println!();
    }
}

fn log_position_changes(
    label: &str,
    before_bs: Option<&BalanceSheet>,
    after_bs: Option<&BalanceSheet>,
    after_state: &SimState,
    side: PositionSide,
) {
    let empty_map = HashMap::new();
    let before_pos = before_bs.map(|bs| match side {
        PositionSide::Asset => &bs.assets,
        PositionSide::Liability => &bs.liabilities,
    }).unwrap_or(&empty_map);
    
    let after_pos = after_bs.map(|bs| match side {
        PositionSide::Asset => &bs.assets,
        PositionSide::Liability => &bs.liabilities,
    }).unwrap_or(&empty_map);

    let mut all_keys: HashSet<_> = before_pos.keys().collect();
    all_keys.extend(after_pos.keys());

    let mut changes_found = false;
    let mut output_lines = Vec::new();

    for instr_id in all_keys {
        let before_p = before_pos.get(instr_id);
        let after_p = after_pos.get(instr_id);
        let info = after_state.financial_system.get_instrument_info(instr_id, &after_state.agents, after_state.current_date);
        let inst_type = info.as_ref().map_or("Unknown", |i| i.instrument_type);

        let line = match (before_p, after_p) {
            (Some(b), Some(a)) if (b.quantity - a.quantity).abs() > 1e-9 => { // Use tolerance for f64 comparison
                changes_found = true;
                Some(format!(
                    "{YELLOW}~ | {:<24} | ${:>10.2} -> ${:>10.2}{RESET}",
                    inst_type, b.quantity, a.quantity
                ))
            }
            (None, Some(a)) => {
                changes_found = true;
                Some(format!(
                    "{GREEN}+ | {:<24} | ${:>10.2}{RESET}",
                    inst_type, a.quantity
                ))
            }
            (Some(b), None) => {
                changes_found = true;
                Some(format!(
                    "{RED}- | {:<24} | ${:>10.2}{RESET}",
                    inst_type, b.quantity
                ))
            }
            _ => None,
        };
        if let Some(l) = line {
            output_lines.push(l);
        }
    }
    
    if changes_found {
        println!("  {}:", label);
        for line in output_lines {
            println!("    {}", line);
        }
    }
}


pub fn _log_effects(effects: &[StateEffect], state: &SimState) {
    println!("Generated {} Effects:", effects.len());

    const GREEN: &str = "\x1b[32m";
    const RED: &str = "\x1b[31m";
    const YELLOW: &str = "\x1b[33m";
    const RESET: &str = "\x1b[0m";

    for effect in effects {
        match effect {
            StateEffect::Financial(FinancialEffect::AdjustPosition {
                owner,
                instrument_id,
                delta_quantity,
                side,
                ..
            }) => {
                let (name, _agent_type) = get_agent_display_info(state, owner);
                let inst_info = state.financial_system.get_instrument_info(instrument_id, &state.agents, state.current_date);
                let inst_type = inst_info.as_ref().map_or("Unknown Instrument", |i| i.instrument_type);

                let (verb, color) = match (side.clone(), *delta_quantity > 0.0) {
                    (PositionSide::Asset, true) | (PositionSide::Liability, false) => ("DEBIT", RED),
                    (PositionSide::Asset, false) | (PositionSide::Liability, true) => ("CREDIT", GREEN),
                };

                println!(
                    "  - {color}{verb: <7}{RESET} | '{name:<16}' | {inst_type:<24} | Amount: {:>10.2}",
                    delta_quantity.abs(),
                );
            }
            StateEffect::Financial(FinancialEffect::RecordTransaction(tx)) => {
                let (from_name, _) = get_agent_display_info(state, &tx.from_agent);
                let (to_name, _) = get_agent_display_info(state, &tx.to_agent);
                println!(
                    " \n - {YELLOW}RECORD {RESET} | Transaction: '{}' from '{}' to '{}' for ${:.2}",
                    tx.transaction_type, from_name, to_name, tx.amount
                );
            }
            _ => {
                println!("  - {} [{:#?}]", effect.name(), effect);
            }
        }
    }
}