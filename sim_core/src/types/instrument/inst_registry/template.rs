use crate::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentTemplate {
    pub id: TemplateId,
    pub template_type: TemplateType,
    pub product_family: ProductFamily,
    pub market_classification: MarketClassification,
    pub lifecycle_rules: LifecycleRules,
    pub created_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateType {
    DepositAccount {
        deposit_type: CashType,
        interest_type: InterestType,
    },
    
    BondTemplate {
        bond_class: BondClass,
        cash_flow_type: CashFlow,
        day_count: DayCount,
    },
    
    LoanFacility {
        facility_type: FacilityType,
        amortization: Amortization,
        prepayment_allowed: bool,
    },
    
    StructuredProduct {
        product_type: StructuredProductType,
        underlying_asset_class: AssetClass,
    },
    
    DerivativeContract {
        derivative_class: DerivativeClass,
        settlement_type: SettlementType,
    },
}