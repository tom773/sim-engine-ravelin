use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaxRates {
    pub income_tax: f64,
    pub corporate_tax: f64,
    pub capital_gains: f64,
    pub consumption_tax: f64,
}

impl Default for TaxRates {
    fn default() -> Self {
        Self {
            income_tax: 0.2,
            corporate_tax: 0.2,
            capital_gains: 0.15,
            consumption_tax: 0.1,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpendingTargets {
    pub transfers: f64,
    pub purchases: f64,
    pub investment: f64,
    pub debt_service: f64,
}

impl Default for SpendingTargets {
    fn default() -> Self {
        Self {
            transfers: 0.0,
            purchases: 0.0,
            investment: 0.0,
            debt_service: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FiscalPolicy {
    Balanced,
    Expansionary { deficit_target: f64 },
    Contractionary { surplus_target: f64 },
    Automatic,
}
impl Default for FiscalPolicy {
    fn default() -> Self {
        FiscalPolicy::Balanced
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TaxType {
    Income,
    Corporate,
    CapitalGains,
    Consumption,
}
