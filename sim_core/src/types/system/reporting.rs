use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactSnapshot {
    pub t: u32,
    pub bs: HashMap<String, CompactBalanceSheet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactBalanceSheet {
    pub a: Vec<CompactPosition>,
    pub l: Vec<CompactPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub e: Option<CompactPosition>,
    pub i: CompactIncome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UltraCompactPosition {
    pub k: String,
    pub v: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactPosition {
    pub k: String,
    pub q: f64,
    pub b: f64,
    pub c: f64,
    pub m: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactIncome {
    pub r: f64,
    pub c: f64,
    pub o: f64,
    pub ii: f64,
    pub ie: f64,
    pub n: f64,
}

impl FinancialSystem {
    pub fn export_compact_snapshot_to_file(&self, tick: u32, filepath: &str) -> std::io::Result<()> {
        use std::fs::{File, OpenOptions};
        use std::io::Write;

        let snapshot = self.create_compact_snapshot(tick);
        let json = serde_json::to_string(&snapshot)?;

        let mut file = if tick == 0 {
            File::create(filepath)?
        } else {
            OpenOptions::new().append(true).create(true).open(filepath)?
        };

        if tick > 0 {
            file.write_all(b"\n")?;
        }

        file.write_all(json.as_bytes())?;

        println!("Snapshot saved: {} bytes (tick {})", json.len(), tick);
        Ok(())
    }

    pub fn export_ultra_compact_to_file(&self, tick: u32, filepath: &str) -> std::io::Result<()> {
        use std::fs::{File, OpenOptions};
        use std::io::Write;

        let snapshot = self.create_ultra_compact_snapshot(tick);
        let json = serde_json::to_string(&snapshot)?;

        if tick == 0 {
            let mut file = File::create(filepath)?;
            let legend = CompactSnapshot::generate_legend();
            file.write_all(legend.as_bytes())?;
            file.write_all(b"\n\n```json\n")?;
            file.write_all(json.as_bytes())?;
            file.write_all(b"\n")?;
        } else {
            let mut file = OpenOptions::new().append(true).open(filepath)?;
            file.write_all(json.as_bytes())?;
            file.write_all(b"\n")?;
        }

        Ok(())
    }

    pub fn export_as_ndjson(&self, tick: u32, filepath: &str) -> std::io::Result<()> {
        use std::fs::{File, OpenOptions};
        use std::io::Write;

        let mut snapshot = serde_json::json!({});

        for (agent_id, balance_sheet) in &self.balance_sheets {
            let agent_key = format!("{:.6}", agent_id.0.to_string());

            let mut agent_data = serde_json::json!({});

            let net_worth = self.get_total_assets(agent_id) - self.get_total_liabilities(agent_id);

            let assets: Vec<_> = balance_sheet
                .assets
                .iter()
                .filter(|(_, p)| p.quantity.abs() > 0.01)
                .map(|(id, pos)| {
                    let inst = &self.instruments.instruments[id];
                    let code = self.get_instrument_type_code(inst);
                    vec![
                        format!("{}:{}", code, &id.0.to_string()[0..4]),
                        format!("{:.2}", pos.quantity),
                        format!("{:.0}", pos.book_value_per_unit.to_f64()),
                    ]
                })
                .collect();

            if !assets.is_empty() {
                agent_data["a"] = serde_json::json!(assets);
            }

            let liabilities: Vec<_> = balance_sheet
                .liabilities
                .iter()
                .filter(|(_, p)| p.quantity.abs() > 0.01)
                .map(|(id, pos)| {
                    let inst = &self.instruments.instruments[id];
                    let code = self.get_instrument_type_code(inst);
                    vec![
                        format!("{}:{}", code, &id.0.to_string()[0..4]),
                        format!("{:.2}", pos.quantity),
                        format!("{:.0}", pos.book_value_per_unit.to_f64()),
                    ]
                })
                .collect();

            if !liabilities.is_empty() {
                agent_data["l"] = serde_json::json!(liabilities);
            }

            if let Some(csd_account) = self.clearing_house.csd.custody_accounts.get(agent_id) {
                let securities: Vec<_> = csd_account
                    .holdings
                    .iter()
                    .filter(|(_, h)| h.total_position() > 0.01)
                    .map(|(inst_id, holding)| {
                        let inst = &self.instruments.instruments[inst_id];
                        let code = self.get_instrument_type_code(inst);
                        vec![
                            format!("{}:{}", code, &inst_id.0.to_string()[0..4]),
                            format!("{:.2}", holding.total_position()),
                            format!("{:.0}", inst.unit_par_value().map(|v| v.to_f64()).unwrap_or(1.0)),
                        ]
                    })
                    .collect();

                if !securities.is_empty() {
                    agent_data["s"] = serde_json::json!(securities);
                }
            }

            if net_worth.abs() > 0.01 {
                agent_data["e"] = serde_json::json!([
                    format!("EQ:{}", &agent_id.0.to_string()[0..4]),
                    format!("{:.2}", net_worth),
                    "1",
                ]);
            }

            let is = &balance_sheet.income_statement;
            if is.net_income.to_f64().abs() > 0.01 {
                agent_data["i"] = serde_json::json!({
                    "n": format!("{:.2}", is.net_income.to_f64())
                });
            }

            if !agent_data.as_object().unwrap().is_empty() {
                snapshot[agent_key] = agent_data;
            }
        }

        let line = serde_json::json!({
            "t": tick,
            "d": snapshot
        });

        if tick == 0 {
            let mut file = File::create(filepath)?;

            let legend_json = serde_json::json!({
                "_legend": true,
                "format": "Balance Sheet NDJSON",
                "fields": {
                    "t": "tick number",
                    "d": "data by agent (6-char id prefix)",
                "a": "assets [[type:id, qty, book_val]]",
                "l": "liabilities [[type:id, qty, book_val]]",
                "s": "securities [[type:id, qty, face_val]]",
                "e": "equity [type:id, qty, book_val]",
                "i": "income {n: net_income}"
            },
                "types": {
                    "DD": "Demand Deposit",
                    "SD": "Savings Deposit",
                    "TD": "Time Deposit",
                    "CUR": "Currency",
                    "RES": "Central Bank Reserves",
                    "TGA": "Treasury General Account",
                    "VC": "Vault Cash",
                    "CB": "Corporate Bond",
                    "GB": "Government Bond",
                    "IB": "Interbank Loan",
                    "LN": "Loan",
                    "CL": "Credit Line",
                    "TC": "Trade Credit",
                    "INV": "Inventory",
                    "PRO": "Property",
                    "EQ": "Equity",
                    "DER": "Derivative",
                    "ST": "Structured Tranche",
                    "RP": "Repo"
                }
            });

            writeln!(file, "{}", serde_json::to_string(&legend_json)?)?;
            writeln!(file, "{}", serde_json::to_string(&line)?)?;
        } else {
            let mut file = OpenOptions::new().append(true).open(filepath)?;
            writeln!(file, "{}", serde_json::to_string(&line)?)?;
        }

        Ok(())
    }

    pub fn create_compact_snapshot(&self, tick: u32) -> CompactSnapshot {
        let mut bs = HashMap::new();

        for (agent_id, balance_sheet) in &self.balance_sheets {
            let agent_key = format!("{:.8}", agent_id.0.to_string());

            let assets = self.compact_positions(&balance_sheet.assets, true);
            let mut total_assets_book = assets.iter().map(|p| p.q * p.b).sum::<f64>();

            let liabilities = self.compact_positions(&balance_sheet.liabilities, false);
            let total_liabilities_book = liabilities.iter().map(|p| p.q * p.b).sum::<f64>();

            let mut all_assets = assets;
            if let Some(csd_holdings) = self.clearing_house.csd.custody_accounts.get(agent_id) {
                for (inst_id, holding) in &csd_holdings.holdings {
                    if holding.total_position() > 1e-9 {
                        if let Some(compact) = self.create_compact_position_from_csd(inst_id, holding) {
                            total_assets_book += compact.q * compact.b;
                            all_assets.push(compact);
                        }
                    }
                }
            }

            let equity_amount = total_assets_book - total_liabilities_book;

            bs.insert(
                agent_key,
                CompactBalanceSheet {
                    a: all_assets,
                    l: liabilities,
                    e: Self::equity_position_from_amount(agent_id, equity_amount),
                    i: CompactIncome {
                        r: balance_sheet.income_statement.revenue.to_f64(),
                        c: balance_sheet.income_statement.cost_of_goods_sold.to_f64(),
                        o: balance_sheet.income_statement.operating_expenses.to_f64(),
                        ii: balance_sheet.income_statement.interest_income.to_f64(),
                        ie: balance_sheet.income_statement.interest_expense.to_f64(),
                        n: balance_sheet.income_statement.net_income.to_f64(),
                    },
                },
            );
        }

        CompactSnapshot { t: tick, bs }
    }

    fn compact_positions(&self, positions: &HashMap<InstrumentId, Position>, _is_asset: bool) -> Vec<CompactPosition> {
        positions
            .iter()
            .filter_map(|(id, pos)| {
                if pos.quantity.abs() < 1e-9 {
                    return None;
                }

                let inst = self.instruments.instruments.get(id)?;
                let type_code = self.get_instrument_type_code(inst);
                let id_short = format!("{:.6}", id.0.to_string());
                let key = format!("{}:{}", type_code, id_short);

                Some(CompactPosition {
                    k: key,
                    q: pos.quantity,
                    b: pos.book_value_per_unit.to_f64(),
                    c: pos.cost_basis_per_unit.to_f64(),
                    m: self.get_market_price(id).map(|p| p.to_f64()),
                })
            })
            .collect()
    }

    fn create_compact_position_from_csd(
        &self, inst_id: &InstrumentId, holding: &SecurityHolding,
    ) -> Option<CompactPosition> {
        let inst = self.instruments.instruments.get(inst_id)?;
        let type_code = self.get_instrument_type_code(inst);
        let id_short = format!("{:.6}", inst_id.0.to_string());
        let key = format!("{}:{}", type_code, id_short);

        let book_value = inst.unit_par_value().unwrap_or(Money::ONE).to_f64();

        Some(CompactPosition {
            k: key,
            q: holding.total_position(),
            b: book_value,
            c: book_value,
            m: self.get_market_price(inst_id).map(|p| p.to_f64()),
        })
    }

    fn equity_position_from_amount(agent_id: &AgentId, net_worth: f64) -> Option<CompactPosition> {
        if net_worth.abs() < 1e-6 {
            return None;
        }

        let id_short = format!("{:.6}", agent_id.0.to_string());
        let key = format!("EQ:{}", id_short);

        Some(CompactPosition { k: key, q: net_worth, b: 1.0, c: 1.0, m: None })
    }

    fn get_instrument_type_code(&self, inst: &Instrument) -> &str {
        // Runtime-based codes now that InstrumentType wrappers are gone.
        match inst.state() {
            InstrumentRuntime::Cash(d) => match d.cash_type {
                CashType::DemandDeposit => "DD",
                CashType::SavingsDeposit => "SD",
                CashType::TimeDeposit => "TD",
                CashType::Currency => "CUR",
                CashType::CentralBankReserves => "RES",
                CashType::VaultCash => "VC",
                CashType::TreasuryGeneralAccount => "TGA",
            },
            InstrumentRuntime::Bond(b) => match b.bond_type() {
                BondType::Corporate => "CB",
                BondType::Government => "GB",
                BondType::InterbankLoan => "IB",
                BondType::Municipal => "MB",
                BondType::Agency => "AG",
                BondType::Supranational => "SB",
            },
            InstrumentRuntime::Credit(credit) => match credit {
                CreditState::Loan(_) => "LN",
                CreditState::ConsumerLoan { .. } => "LN",
                CreditState::ConsumerCreditCard(_) => "CL",
                CreditState::Facility(_) => "CL",
                CreditState::TradeCredit(_) => "TC",
            },
            InstrumentRuntime::RealAsset(asset) => match asset {
                RealAssetState::Inventory { .. } => "INV",
                RealAssetState::Property { .. } => "PRO",
                RealAssetState::Custom { .. } => "REAL",
            },
            InstrumentRuntime::Equity(_) => "EQ",
            InstrumentRuntime::Derivative(_) => "DER",
            InstrumentRuntime::Structured(_) => "ST",
            InstrumentRuntime::Repo(_) => "RP",
        }
    }

    pub fn create_ultra_compact_snapshot(&self, tick: u32) -> serde_json::Value {
        use serde_json::json;

        let mut snapshot = json!({});

        for (agent_id, balance_sheet) in &self.balance_sheets {
            let agent_key = &agent_id.0.to_string()[0..6];

            let mut agent_data = json!({});

            let mut asset_total = 0.0;
            let assets: Vec<_> = balance_sheet
                .assets
                .iter()
                .filter(|(_, p)| p.quantity.abs() > 0.01)
                .map(|(id, pos)| {
                    let inst = &self.instruments.instruments[id];
                    let code = self.get_instrument_type_code(inst);
                    let book = pos.book_value_per_unit.to_f64();
                    asset_total += pos.quantity * book;
                    json!([
                        format!("{}:{}", code, &id.0.to_string()[0..4]),
                        (pos.quantity * 100.0).round() / 100.0,
                        book.round(),
                    ])
                })
                .collect();

            if !assets.is_empty() {
                agent_data["a"] = json!(assets);
            }

            let mut liability_total = 0.0;
            let liabilities: Vec<_> = balance_sheet
                .liabilities
                .iter()
                .filter(|(_, p)| p.quantity.abs() > 0.01)
                .map(|(id, pos)| {
                    let inst = &self.instruments.instruments[id];
                    let code = self.get_instrument_type_code(inst);
                    let book = pos.book_value_per_unit.to_f64();
                    liability_total += pos.quantity * book;
                    json!([
                        format!("{}:{}", code, &id.0.to_string()[0..4]),
                        (pos.quantity * 100.0).round() / 100.0,
                        book.round(),
                    ])
                })
                .collect();

            if !liabilities.is_empty() {
                agent_data["l"] = json!(liabilities);
            }

            let mut securities_total = 0.0;
            if let Some(csd_account) = self.clearing_house.csd.custody_accounts.get(agent_id) {
                let securities: Vec<_> = csd_account
                    .holdings
                    .iter()
                    .filter(|(_, h)| h.total_position() > 0.01)
                    .map(|(inst_id, holding)| {
                        let inst = &self.instruments.instruments[inst_id];
                        let code = self.get_instrument_type_code(inst);
                        let book = inst.unit_par_value().map(|v| v.to_f64()).unwrap_or(1.0);
                        securities_total += holding.total_position() * book;
                        json!([
                            format!("{}:{}", code, &inst_id.0.to_string()[0..4]),
                            (holding.total_position() * 100.0).round() / 100.0,
                            book.round(),
                        ])
                    })
                    .collect();

                if !securities.is_empty() {
                    agent_data["s"] = json!(securities);
                }
            }

            let net_worth = (asset_total + securities_total) - liability_total;

            if net_worth.abs() > 0.01 {
                agent_data["e"] =
                    json!([format!("EQ:{}", &agent_id.0.to_string()[0..4]), (net_worth * 100.0).round() / 100.0, 1.0,]);
            }

            let is = &balance_sheet.income_statement;
            if is.net_income.to_f64().abs() > 0.01 {
                let mut income = json!({});
                if is.revenue.to_f64().abs() > 0.01 {
                    income["r"] = json!((is.revenue.to_f64() * 100.0).round() / 100.0);
                }
                if is.cost_of_goods_sold.to_f64().abs() > 0.01 {
                    income["c"] = json!((is.cost_of_goods_sold.to_f64() * 100.0).round() / 100.0);
                }
                income["n"] = json!((is.net_income.to_f64() * 100.0).round() / 100.0);
                agent_data["i"] = income;
            }

            if !agent_data.as_object().unwrap().is_empty() {
                snapshot[agent_key] = agent_data;
            }
        }

        snapshot["t"] = json!(tick);
        snapshot
    }
}

impl CompactSnapshot {
    pub fn generate_legend() -> String {
        r#"COMPACT BALANCE SHEET FORMAT LEGEND:
=====================================
Field Abbreviations:
  t: tick number
  d: data (agent_id -> balance sheet)
  a: assets array
  l: liabilities array
  s: securities (CSD holdings)
  e: equity array
  i: income statement
  
Array format: [type:id, quantity, book_value]
  
Income Statement:
  r: revenue
  c: cost of goods sold
  n: net income

Instrument Type Codes:
  DD: Demand Deposit       RES: Central Bank Reserves
  SD: Savings Deposit      TGA: Treasury General Account
  TD: Time Deposit         CB: Corporate Bond
  CUR: Currency            GB: Government Bond
  VC: Vault Cash          IB: Interbank Loan
  LN: Loan                CL: Credit Line
  TC: Trade Credit        INV: Inventory
  PRO: Property           EQ: Equity
  DER: Derivative         ST: Structured Tranche
  RP: Repo"#
            .to_string()
    }
}
