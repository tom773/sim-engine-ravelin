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
    // Updated symbol derivation to match on InstrumentRuntime now that legacy InstrumentType was removed.
    match inst.state() {
        InstrumentRuntime::Bond(bond) => {
            let base = match bond.bond_type() {
                BondType::Government => "GOV",
                BondType::Corporate => "CORP",
                BondType::InterbankLoan => "IBL",
                BondType::Municipal => "MUNI",
                BondType::Agency => "AGY",
                BondType::Supranational => "SUPRA",
            };

            let tenor_years = bond.archetype.day_count.year_fraction(bond.issue_date, bond.maturity_date).abs();
            let tenor = Tenor::from_years(tenor_years);
            let year = bond.maturity_date.year();
            let issuer_short = short_agent(bond.issuer);

            let body = if matches!(bond.bond_type(), BondType::Corporate) {
                format!("{base}_{issuer_short}_{}_{}", fmt_tenor(tenor), year)
            } else {
                format!("{base}_{}_{}", fmt_tenor(tenor), year)
            };

            Symbol(body)
        }
        InstrumentRuntime::Credit(credit) => match credit {
            CreditState::Loan(loan) => {
                let d = loan.maturity_date.format("%Y%m%d");
                Symbol(format!("LOAN_{}_{}", short_agent(loan.borrower), d))
            }
            CreditState::ConsumerLoan { category, loan } => {
                let tag = match category {
                    ConsumerLoanCategory::ResidentialMortgage => "MTG",
                    ConsumerLoanCategory::AutoLoan => "AUTO",
                    ConsumerLoanCategory::PersonalLoan => "PLN",
                    ConsumerLoanCategory::StudentLoan => "STUD",
                };
                let d = loan.maturity_date.format("%Y%m%d");
                Symbol(format!("{tag}_{}_{}", short_agent(loan.borrower), d))
            }
            CreditState::ConsumerCreditCard(facility) => {
                let d = facility.expiry_date.format("%Y%m%d");
                Symbol(format!("CARD_{}_{}", short_agent(facility.borrower), d))
            }
            CreditState::Facility(facility) => {
                let d = facility.expiry_date.format("%Y%m%d");
                Symbol(format!("CRED_{}_{}", short_agent(facility.borrower), d))
            }
            CreditState::TradeCredit(details) => {
                let d = details.due_date.format("%Y%m%d");
                Symbol(format!("TCR_{}_{}", short_agent(details.debtor), d))
            }
        },
        InstrumentRuntime::Equity(equity) => Symbol(format!("EQ_{}", short_agent(equity.profile.issuer))),
        InstrumentRuntime::Cash(cash) => {
            let tag = match cash.cash_type {
                CashType::DemandDeposit => "DEM",
                CashType::SavingsDeposit => "SAV",
                CashType::TimeDeposit => "TERM",
                CashType::CentralBankReserves => "MM",
                CashType::Currency => "CUR",
                CashType::VaultCash => "VAULT",
                CashType::TreasuryGeneralAccount => "TGA",
            };
            let ccy = format!("{:?}", cash.currency);
            Symbol(format!("{tag}_{ccy}"))
        }
        InstrumentRuntime::Derivative(derivative) => {
            let und = match &derivative.underlying {
                UnderlyingAsset::Instrument(i) => format!("I{}", short_inst(*i)),
                UnderlyingAsset::Good(g) => format!("G{}", short_from_uuid(g.0)),
                UnderlyingAsset::Index(s) => slugify(s),
            };
            let dte = derivative
                .expiry_date
                .map(|d| d.format("%Y%m%d").to_string())
                .unwrap_or_else(|| "00000000".to_string());
            Symbol(format!("DRV_{}_{}", und, dte))
        }
        InstrumentRuntime::Structured(tranche) => {
            let year = tranche.maturity_date.year();
            let rating_label = match tranche.rating {
                CreditRating::Government(r) | CreditRating::Corporate(r) => format!("{:?}", r),
                CreditRating::Consumer(_) => "CN".into(),
            };
            Symbol(format!("TR_{}_{}", rating_label, year))
        }
        InstrumentRuntime::Repo(repo) => {
            let d = repo.end_date.format("%Y%m%d");
            Symbol(format!("REPO_{}_{}", short_inst(repo.collateral_id), d))
        }
        InstrumentRuntime::RealAsset(asset) => match asset {
            RealAssetState::Inventory { .. } => Symbol("REAL_INV".into()),
            RealAssetState::Property { .. } => Symbol("REAL_PROP".into()),
            RealAssetState::Custom { description, .. } => {
                let slug = slugify(description);
                Symbol(if slug.is_empty() { "REAL_CUSTOM".into() } else { slug })
            }
        },
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
