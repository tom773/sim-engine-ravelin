use crate::*;
use chrono::Datelike;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;
#[derive(Debug, Clone, Copy)]
pub struct ShortId(pub u64);

impl fmt::Display for ShortId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut v = self.0;
        if v == 0 {
            return f.write_str("0");
        }

        const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let mut buf = [0u8; 20];
        let mut i = buf.len();

        while v > 0 {
            i -= 1;
            buf[i] = CHARS[(v % 36) as usize];
            v /= 36;
        }

        let full_id = std::str::from_utf8(&buf[i..]).unwrap();
        let truncated_id = &full_id[..full_id.len().min(4)];
        f.write_str(truncated_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub struct Symbol(pub String);

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::str::FromStr for Symbol {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Symbol(s.to_string()))
    }
}
#[derive(Debug, Default, Clone)] // This derive will now work
pub struct SymbolRegistry {
    inst: Arc<RwLock<DashMap<InstrumentId, (Symbol, ShortId)>>>,
    goods: Arc<RwLock<DashMap<GoodId, (Symbol, ShortId)>>>,
    labour: Arc<RwLock<DashMap<LabourMarketId, (Symbol, ShortId)>>>,
}

impl SymbolRegistry {
    pub fn new() -> Self {
        Self {
            inst: Arc::new(RwLock::new(DashMap::new())),
            goods: Arc::new(RwLock::new(DashMap::new())),
            labour: Arc::new(RwLock::new(DashMap::new())),
        }
    }

    pub fn ensure_instrument(&self, id: InstrumentId, inst: &Instrument) -> Symbol {
        let w = self.inst.write();
        let entry = w.entry(id).or_insert_with(|| {
            let sym = derive_instrument_symbol(inst);
            (sym.clone(), short_from_uuid(id.0))
        });
        entry.0.clone()
    }

    pub fn register_good(&self, id: GoodId, name: &str) -> Symbol {
        let short = short_from_uuid(id.0);
        let w = self.goods.write();
        let entry = w.entry(id).or_insert_with(|| (Symbol(format!("{}:{}", slugify(name), short)), short));
        entry.0.clone()
    }

    pub fn register_labour(&self, id: LabourMarketId, label: &str) -> Symbol {
        let w = self.labour.write();
        let entry = w.entry(id).or_insert_with(|| (Symbol(slugify(label)), short_from_uuid(id.0)));
        entry.0.clone()
    }

    pub fn symbol_of_inst(&self, id: &InstrumentId) -> Option<(Symbol, ShortId)> {
        self.inst.read().get(id).map(|entry| entry.clone())
    }

    pub fn symbol_of_good(&self, id: &GoodId) -> Option<(Symbol, ShortId)> {
        self.goods.read().get(id).map(|entry| entry.clone())
    }

    pub fn symbol_of_labour(&self, id: &LabourMarketId) -> Option<(Symbol, ShortId)> {
        self.labour.read().get(id).map(|entry| entry.clone())
    }
}

fn short_from_uuid(u: Uuid) -> ShortId {
    ShortId((u.as_u128() >> 64) as u64 ^ (u.as_u128() as u64))
}

fn short_agent(a: AgentId) -> ShortId {
    short_from_uuid(a.0)
}

fn short_inst(i: InstrumentId) -> ShortId {
    short_from_uuid(i.0)
}

fn fmt_tenor(t: Tenor) -> &'static str {
    match t {
        Tenor::T1M => "1M",
        Tenor::T2M => "2M",
        Tenor::T3M => "3M",
        Tenor::T6M => "6M",
        Tenor::T1Y => "1Y",
        Tenor::T2Y => "2Y",
        Tenor::T5Y => "5Y",
        Tenor::T10Y => "10Y",
        Tenor::T30Y => "30Y",
    }
}

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let ch = match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' => c.to_ascii_uppercase(),
            _ => '_',
        };
        out.push(ch);
    }
    out.trim_matches('_').to_string();
    out.to_lowercase().replace(' ', "_")
}

fn derive_instrument_symbol(inst: &Instrument) -> Symbol {
    match &inst.instrument_type {
        InstrumentType::Debt(debt) => match debt {
            DebtInstrument::Bond(b) => {
                let base = match b.bond_type {
                    BondType::Government => "GOV",
                    BondType::Corporate => "CORP",
                    BondType::InterbankLoan => "IBL",
                };

                let tenor = Tenor::from_years(b.original_tenor_years());
                let y = b.maturity_date.year();
                let iss = short_agent(b.issuer);

                let body = if matches!(b.bond_type, BondType::Corporate) {
                    format!("{base}_{iss}_{}_{}", fmt_tenor(tenor), y)
                } else {
                    format!("{base}_{}_{}", fmt_tenor(tenor), y)
                };

                Symbol(body)
            }
            DebtInstrument::Loan(l) => {
                let d = l.maturity_date.format("%Y%m%d");
                Symbol(format!("LOAN_{}_{}", short_agent(l.borrower), d))
            }
            DebtInstrument::CreditLine(c) => {
                let d = c.expiry_date.format("%Y%m%d");
                Symbol(format!("CRED_{}_{}", short_agent(c.borrower), d))
            }
            DebtInstrument::TradeCredit(t) => {
                let d = t.due_date.format("%Y%m%d");
                Symbol(format!("TCR_{}_{}", short_agent(t.debtor), d))
            }
            DebtInstrument::Consumer(c) => match c {
                ConsumerDebt::ResidentialMortgage(l) => {
                    let d = l.maturity_date.format("%Y%m%d");
                    Symbol(format!("MTG_{}_{}", short_agent(l.borrower), d))
                }
                ConsumerDebt::AutoLoan(l) => {
                    let d = l.maturity_date.format("%Y%m%d");
                    Symbol(format!("AUTO_{}_{}", short_agent(l.borrower), d))
                }
                ConsumerDebt::PersonalLoan(l) => {
                    let d = l.maturity_date.format("%Y%m%d");
                    Symbol(format!("PLN_{}_{}", short_agent(l.borrower), d))
                }
                ConsumerDebt::CreditCard(c) => {
                    let d = c.expiry_date.format("%Y%m%d");
                    Symbol(format!("CARD_{}_{}", short_agent(c.borrower), d))
                }
                ConsumerDebt::StudentLoan(l) => {
                    let d = l.maturity_date.format("%Y%m%d");
                    Symbol(format!("STUD_{}_{}", short_agent(l.borrower), d))
                }
            },
        },
        InstrumentType::Equity(e) => Symbol(format!("EQ_{}", short_agent(e.issuer))),
        InstrumentType::Cash(c) => {
            let tag = match c.cash_type {
                CashType::DemandDeposit => "DEM",
                CashType::SavingsDeposit => "SAV",
                CashType::TimeDeposit => "TERM",
                CashType::CentralBankReserves => "MM",
                CashType::Currency => "CUR",
                CashType::VaultCash => "VAULT",
                CashType::TreasuryGeneralAccount => "TGA",
            };
            let ccy = format!("{:?}", c.currency);
            Symbol(format!("{tag}_{ccy}"))
        }
        InstrumentType::Derivative(d) => {
            let und = match &d.underlying {
                UnderlyingAsset::Instrument(i) => format!("I{}", short_inst(*i)),
                UnderlyingAsset::Good(g) => format!("G{}", short_from_uuid(g.0)),
                UnderlyingAsset::Index(s) => slugify(s),
            };
            let dte = d.expiry_date.format("%Y%m%d");
            Symbol(format!("DRV_{}_{}", und, dte))
        }
        InstrumentType::StructuredTranche(st) => {
            let y = st.maturity_date.year();
            let r = match st.rating {
                CreditRating::Government(r) | CreditRating::Corporate(r) => format!("{:?}", r),
                CreditRating::Consumer(_) => "CN".into(),
            };
            Symbol(format!("TR_{}_{}", r, y))
        }
        InstrumentType::Repo(rp) => {
            let d = rp.end_date.format("%Y%m%d");
            Symbol(format!("REPO_{}_{}", short_inst(rp.collateral_id), d))
        }
        InstrumentType::RealAsset(_) => Symbol("REAL".into()),
    }
}

impl From<&Instrument> for Symbol {
    fn from(i: &Instrument) -> Self {
        derive_instrument_symbol(i)
    }
}

impl From<Instrument> for Symbol {
    fn from(i: Instrument) -> Self {
        derive_instrument_symbol(&i)
    }
}
