use domains::*;
use serde::{Deserialize, Serialize};
use sim_core::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainRegistry {
    pub banking: BankingDomain,
    pub production: ProductionDomain,
    pub trading: TradingDomain,
    pub consumption: ConsumptionDomain,
    pub fiscal: FiscalDomain,
    pub settlement: SettlementDomain,
}

impl DomainRegistry {
    pub fn new() -> Self {
        Self {
            banking: BankingDomain::new(),
            production: ProductionDomain::new(),
            trading: TradingDomain::new(),
            consumption: ConsumptionDomain::new(),
            fiscal: FiscalDomain::new(),
            settlement: SettlementDomain::new(),
        }
    }

    pub fn execute(&self, action: &SimAction, state: &SimState) -> Vec<StateEffect> {
        match action {
            SimAction::Banking(action) => {
                if self.banking.can_handle(action) {
                    let result = self.banking.execute(action, state);
                    if !result.success {
                        println!("Banking action failed: {:?}", result.errors);
                    }
                    result.effects
                } else {
                    println!("Banking domain cannot handle action: {:?}", action);
                    vec![]
                }
            }
            SimAction::Production(action) => {
                if self.production.can_handle(action) {
                    let result = self.production.execute(action, state);
                    if !result.success {
                        println!("Production action failed: {:?}", result.errors);
                    }
                    result.effects
                } else {
                    println!("Production domain cannot handle action: {:?}", action);
                    vec![]
                }
            }
            SimAction::Trading(action) => {
                if self.trading.can_handle(action) {
                    let result = self.trading.execute(action, state);
                    if !result.success {
                        println!("Trading action failed: {:?}", result.errors);
                    }
                    result.effects
                } else {
                    println!("Trading domain cannot handle action: {:?}", action);
                    vec![]
                }
            }
            SimAction::Consumption(action) => {
                if self.consumption.can_handle(action) {
                    let result = self.consumption.execute(action, state);
                    if !result.success {
                        println!("Consumption action failed: {:?}", result.errors);
                    }
                    result.effects
                } else {
                    println!("Consumption domain cannot handle action: {:?}", action);
                    vec![]
                }
            }
            SimAction::Fiscal(action) => {
                if self.fiscal.can_handle(action) {
                    let result = self.fiscal.execute(action, state);
                    if !result.success {
                        println!("Fiscal action failed: {:?}", result.errors);
                    }
                    result.effects
                } else {
                    println!("Fiscal domain cannot handle action: {:?}", action);
                    vec![]
                }
            }
            SimAction::Settlement(action) => {
                if self.settlement.can_handle(action) {
                    let result = self.settlement.execute(action, state);
                    if !result.success {
                        println!("Settlement action failed: {:?}", result.errors);
                    }
                    result.effects
                } else {
                    println!("Settlement domain cannot handle action: {:?}", action);
                    vec![]
                }
            }
        }
    }
}

impl Default for DomainRegistry {
    fn default() -> Self {
        Self::new()
    }
}