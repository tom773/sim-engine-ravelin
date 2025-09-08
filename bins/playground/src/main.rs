use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use engine_v3::{Scenario, SimulationEngine};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Wrap},
};
use sim_core::prelude::*;
use std::{io, time::Duration};

struct App<'a> {
    engine: SimulationEngine,
    menu_items: Vec<&'a str>,
    menu_state: ListState,
    last_action_result: String,
}

impl<'a> App<'a> {
    fn new(engine: SimulationEngine) -> App<'a> {
        let mut menu_state = ListState::default();
        menu_state.select(Some(0));

        App {
            engine,
            menu_items: vec![
                "View Agent Balance Sheet",
                "Apply Sample Effect",
                "Execute Sample Action (Payment)",
                "Process RTGS Queue",
            ],
            menu_state,
            last_action_result: String::new(),
        }
    }

    fn next(&mut self) {
        let i = match self.menu_state.selected() {
            Some(i) => {
                if i >= self.menu_items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.menu_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.menu_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.menu_items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.menu_state.select(Some(i));
    }

    fn on_enter(&mut self) {
        if let Some(selected) = self.menu_state.selected() {
            match selected {
                1 => self.apply_effect(),
                2 => self.execute_action(),
                3 => self.process_rtgs(),
                _ => {}
            }
        }
    }

    fn apply_effect(&mut self) {
        let consumer_id = self.engine.state.agents.consumers.keys().next().cloned().unwrap();
        let bank_id = self.engine.state.agents.banks.keys().next().cloned().unwrap();

        let new_deposit_instrument = Instrument::cash(
            InstrumentId(uuid::Uuid::new_v4()),
            bank_id,
            CashType::DemandDeposit,
            Currency::USD,
            rust_decimal_macros::dec!(0),
        )
        .build();

        let effect = StateEffect::Financial(FinancialEffect::CreateInstrument {
            instrument: new_deposit_instrument,
            creditor: consumer_id,
            debtor: bank_id,
            quantity: 5000.0,
        });

        match self.engine.state.apply_effect(&effect) {
            Ok(_) => self.last_action_result = "Success: Applied CreateInstrument effect.".to_string(),
            Err(e) => self.last_action_result = format!("Error: {:?}", e),
        };
    }

    fn execute_action(&mut self) {
        let firm_id = self.engine.state.agents.firms.keys().next().cloned().unwrap();
        let consumer_id = self.engine.state.agents.consumers.keys().next().cloned().unwrap();
        let action = SimAction::Transaction(TransactionAction::InitiatePayment {
            from: firm_id,
            to: consumer_id,
            amount: 20000.0,
            context: TransactionContext::GenericTransfer {
                from: firm_id,
                to: consumer_id,
                amount: 20000.0,
            },
        });
        match self.engine.domain_registry.execute_action(&action, &self.engine.state) {
            Ok(effects) => {
                let effect_count = effects.len();
                match self.engine.state.apply_effects(&effects) {
                    Ok(_) => {
                        self.last_action_result =
                            format!("Success: Action generated and applied {} effects.", effect_count)
                    }
                    Err(e) => self.last_action_result = format!("Error applying effects: {:?}", e),
                }
            }
            Err(e) => self.last_action_result = format!("Error executing action: {}", e),
        };
    }

    fn process_rtgs(&mut self) {
        let initial_pending = self.engine.state.financial_system.rtgs.pending.len();
        if initial_pending == 0 {
            self.last_action_result = "RTGS: No pending payments to process.".to_string();
            return;
        }

        match run_rtgs(&mut self.engine.state) {
            Ok(finalization_effects) => {
                if let Err(e) = self.engine.state.apply_effects(&finalization_effects) {
                    self.last_action_result = format!("RTGS Error: Failed to apply finalization effects: {:?}", e);
                    return;
                }

                let settled_count = initial_pending - self.engine.state.financial_system.rtgs.pending.len();
                self.last_action_result = format!("RTGS: Processed {} payments.", settled_count);
            }
            Err(e) => {
                self.last_action_result = format!("RTGS Error: {:?}", e);
            }
        }
    }
}

fn main() -> Result<(), io::Error> {
    let scenario =
        Scenario::from_toml_str(include_str!("../../../config/config.toml")).expect("Failed to load scenario");
    let engine = scenario.initialize_engine();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(engine);
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => app.next(),
                    KeyCode::Up => app.previous(),
                    KeyCode::Enter => app.on_enter(),
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(chunks[0]);

    let menu_items: Vec<ListItem> = app.menu_items.iter().map(|&i| ListItem::new(i)).collect();
    let menu = List::new(menu_items)
        .block(Block::default().borders(Borders::ALL).title("Menu"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::Gray).fg(Color::Black))
        .highlight_symbol("> ");
    f.render_stateful_widget(menu, main_chunks[0], &mut app.menu_state);

    let view_block = Block::default().borders(Borders::ALL).title("View");
    let is_bs_selected = app.menu_state.selected() == Some(0);

    if is_bs_selected {
        let agent_id = app.engine.state.agents.firms.keys().next().unwrap();
        let bs = app.engine.state.financial_system.balance_sheets.get(agent_id).unwrap();

        let header_cells = ["Instrument", "Quantity", "Value (Mkt/Book)"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
        let header = Row::new(header_cells).style(Style::default().bg(Color::DarkGray)).height(1);

        let mut total_assets = Money::ZERO;
        let asset_rows: Vec<Row> = bs
            .assets
            .iter()
            .map(|(id, pos)| {
                let inst = app.engine.state.financial_system.instruments.get(id).unwrap();
                let market_price =
                    app.engine.state.financial_system.get_market_price(id).unwrap_or(pos.book_value_per_unit);
                let value = market_price * pos.quantity;
                total_assets += value;
                Row::new(vec![
                    Cell::from(inst.type_as_string()),
                    Cell::from(format!("{:.2}", pos.quantity)),
                    Cell::from(format!("{:.2}", value.to_f64())),
                ])
            })
            .collect();

        let mut total_liabilities = Money::ZERO;
        let liability_rows: Vec<Row> = bs
            .liabilities
            .iter()
            .map(|(id, pos)| {
                let inst = app.engine.state.financial_system.instruments.get(id).unwrap();
                let market_price =
                    app.engine.state.financial_system.get_market_price(id).unwrap_or(pos.book_value_per_unit);
                let value = market_price * pos.quantity;
                total_liabilities += value;
                Row::new(vec![
                    Cell::from(inst.type_as_string()),
                    Cell::from(format!("{:.2}", pos.quantity)),
                    Cell::from(format!("{:.2}", value.to_f64())),
                ])
            })
            .collect();

        let net_worth = total_assets - total_liabilities;

        let all_rows = [
            vec![Row::new(vec![Cell::from("--- Assets ---").style(Style::default().add_modifier(Modifier::BOLD))])],
            asset_rows,
            vec![Row::new(vec![
                Cell::from("Total Assets").style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from(""),
                Cell::from(format!("{:.2}", total_assets.to_f64()))
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            ])],
            vec![Row::new(vec![Cell::from("")])],
            vec![Row::new(vec![
                Cell::from("--- Liabilities ---").style(Style::default().add_modifier(Modifier::BOLD)),
            ])],
            liability_rows,
            vec![Row::new(vec![
                Cell::from("Total Liabilities").style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from(""),
                Cell::from(format!("{:.2}", total_liabilities.to_f64()))
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            ])],
            vec![Row::new(vec![Cell::from("")])],
            vec![Row::new(vec![
                Cell::from("Net Worth").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Cell::from(""),
                Cell::from(format!("{:.2}", net_worth.to_f64()))
                    .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ])],
        ]
        .concat();

        let widths = [Constraint::Percentage(50), Constraint::Percentage(25), Constraint::Percentage(25)];
        let table = Table::new(all_rows, widths).header(header).block(view_block);

        f.render_widget(table, main_chunks[1]);
    } else {
        let info_text = if let Some(selected) = app.menu_state.selected() {
            match selected {
                1 => {
                    "INFO: Press Enter to apply a sample 'CreateInstrument' effect.\n\nThis will create a new 5000 USD deposit for the first consumer."
                }
                2 => {
                    "INFO: Press Enter to execute a sample 'Produce' action.\n\nThis will make the first firm attempt to produce one batch using its recipe."
                }
                _ => "Unknown selection.",
            }
        } else {
            "Welcome!"
        };

        let content = Paragraph::new(info_text).block(view_block).wrap(Wrap { trim: true });
        f.render_widget(content, main_chunks[1]);
    }

    let footer_block = Block::default().borders(Borders::ALL).title("Status");
    let footer_text =
        format!("Last Action: {} | Use ↑/↓ to navigate, Enter to select, 'q' to quit.", app.last_action_result);
    let footer = Paragraph::new(footer_text).block(footer_block);
    f.render_widget(footer, chunks[1]);
}
