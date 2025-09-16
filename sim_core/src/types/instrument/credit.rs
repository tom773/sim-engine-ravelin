use crate::prelude::*;
use chrono::NaiveDate;
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoanPurpose {
    BusinessExpansion,
    WorkingCapital,
    Equipment,
    RealEstate,
    PersonalConsumption,
    Refinancing,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BorrowerType {
    Individual,
    Corporate,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoanTerms {
    pub principal: f64,
    pub annual_rate_bps: BasisPoints,
    pub term_months: u32,
    pub payment_frequency: PaymentFrequency,
    pub collateral_required: bool,
    pub loan_to_value_ratio: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PaymentFrequency {
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
    InterestOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoanApplication {
    pub application_id: Uuid,
    pub borrower_id: AgentId,
    pub requested_amount: f64,
    pub purpose: LoanPurpose,
    pub proposed_collateral: Option<Vec<InstrumentId>>,
    pub borrower_income: Option<f64>,
    pub debt_to_income_ratio: Option<f64>,
    pub application_date: chrono::NaiveDate,
    pub status: ApplicationStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ApplicationStatus {
    Pending,
    UnderReview,
    Approved,
    Rejected,
    Withdrawn,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoanDecision {
    Approve { terms: LoanTerms },
    Reject { reason: String },
    CounterOffer { alternative_terms: LoanTerms },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentRecord {
    pub payment_date: NaiveDate,
    pub due_date: NaiveDate,
    pub amount: f64,
    pub principal_paid: f64,
    pub interest_paid: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoanStatus {
    Current,
    Deliquent30,
    Deliquent60,
    Deliquent90,
    Defaulted,
    Resolved,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActiveLoan {
    pub instrument_id: InstrumentId,
    pub origination_date: NaiveDate,
    pub original_terms: LoanTerms,
    pub outstanding_principal: f64,
    pub payment_history: Vec<PaymentRecord>,
    pub status: LoanStatus,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CreditHistory {
    pub total_loans_originated: u32,
    pub total_loans_repaid: u32,
    pub total_defaults: u32,
    pub current_debt_service: f64,
    pub payment_performance: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DebtInstrument {
    Bond(BondDetails),
    Loan(LoanDetails),
    CreditLine(CreditLineDetails),
    TradeCredit(TradeCreditDetails),
}

impl DebtInstrument {
    pub fn listability(&self) -> Listability {
        match self {
            DebtInstrument::Bond(_) => Listability::Listed(VenueType::CentralLimitOrderBook),
            DebtInstrument::Loan(_) => Listability::Unlisted,
            DebtInstrument::CreditLine(_) => Listability::Unlisted,
            DebtInstrument::TradeCredit(_) => Listability::Unlisted,
        }
    }

    pub fn issuer(&self) -> AgentId {
        match self {
            DebtInstrument::Bond(b) => b.issuer,
            DebtInstrument::Loan(l) => l.lender,
            DebtInstrument::CreditLine(c) => c.lender,
            DebtInstrument::TradeCredit(t) => t.creditor,
        }
    }

    pub fn debtor(&self) -> AgentId {
        match self {
            DebtInstrument::Bond(b) => b.issuer,
            DebtInstrument::Loan(l) => l.borrower,
            DebtInstrument::CreditLine(c) => c.borrower,
            DebtInstrument::TradeCredit(t) => t.debtor,
        }
    }
    pub fn as_bond(&self) -> Option<&BondDetails> {
        if let DebtInstrument::Bond(b) = self {
            Some(b)
        } else {
            None
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoanDetails {
    pub loan_id: Uuid,
    pub lender: AgentId,
    pub borrower: AgentId,
    pub loan_type: LoanType,
    pub facility_id: Option<Uuid>,
    pub borrower_type: BorrowerType,
    pub principal: Money,
    pub outstanding_principal: Money,
    pub reference_rate: Option<RateIndex>,
    pub spread_bps: BasisPoints,
    pub rate_floor_bps: Option<BasisPoints>,
    pub rate_cap_bps: Option<BasisPoints>,

    pub day_count: DayCount,
    pub compounding: Compounding,
    pub payment_frequency: PaymentFrequency,

    pub origination_date: NaiveDate,
    pub maturity_date: NaiveDate,
    pub next_payment_date: NaiveDate,
    pub last_accrual_date: NaiveDate,

    pub amortization: Amortization,
    pub prepayment_terms: PrepaymentTerms,

    pub collateral: Vec<LienId>,
    pub covenants: Vec<Covenant>,
    pub credit_rating: Option<CreditRating>,

    pub impairment: ImpairmentState,

    pub accrued_interest: Money,
    pub unamortized_fees: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpCreditRating {
    AAA,
    AA,
    A,
    BBB,
    BB,
    B,
    CCC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsumerCreditRating {
    Prime,
    NearPrime,
    Subprime,
    DeepSubprime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CreditRating {
    Government(SpCreditRating),
    Corporate(SpCreditRating),
    Consumer(ConsumerCreditRating),
}

impl CreditRating {
    pub fn government_aaa() -> Self {
        CreditRating::Government(SpCreditRating::AAA)
    }

    pub fn corporate_bbb() -> Self {
        CreditRating::Corporate(SpCreditRating::BBB)
    }

    pub fn consumer_prime() -> Self {
        CreditRating::Consumer(ConsumerCreditRating::Prime)
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreditLineDetails {
    pub facility_id: Uuid,
    pub lender: AgentId,
    pub borrower: AgentId,
    pub facility_type: FacilityType,
    pub borrower_type: BorrowerType,
    pub commitment_amount: Money,
    pub available_amount: Money,
    pub drawn_amount: Money,

    pub undrawn_fee_bps: BasisPoints,
    pub reference_rate: Option<RateIndex>,
    pub spread_bps: BasisPoints,

    pub commitment_date: NaiveDate,
    pub expiry_date: NaiveDate,
    pub draw_period_end: Option<NaiveDate>,

    pub collateral: Vec<LienId>,
    pub covenants: Vec<Covenant>,
    pub credit_rating: Option<SpCreditRating>,

    pub drawn_loans: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeCreditDetails {
    pub creditor: AgentId,
    pub debtor: AgentId,
    pub amount: Money,
    pub invoice_date: NaiveDate,
    pub due_date: NaiveDate,
    pub payment_terms: PaymentTerms,
    pub status: TradeCreditStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoanType {
    TermLoan,
    WorkingCapital,
    BridgeLoan,
    MortgageLoan,
    ProjectFinance,
    AssetFinance,
    PersonalLoan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FacilityType {
    Revolver,
    TermLoanFacility,
    Overdraft,
    LetterOfCredit,
    MultiCurrency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RateIndex {
    RBACashRate,
    BBSW1M,
    BBSW3M,
    BBSW6M,
    SOFR,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Compounding {
    Simple,
    Compounded,
    EffectiveInterestRate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Amortization {
    Bullet,
    Linear,
    Annuity,
    InterestOnly,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepaymentTerms {
    pub allowed: bool,
    pub penalty_type: PrepaymentPenalty,
    pub lockout_period_months: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrepaymentPenalty {
    None,
    FixedAmount(i64),
    PercentOfPrincipal(u32),
    MakeWhole,
    YieldMaintenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImpairmentState {
    pub stage: ImpairmentStage,
    pub provision_amount: Money,
    pub days_past_due: u32,
    pub probability_of_default: f64,
    pub loss_given_default: f64,
    pub exposure_at_default: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImpairmentStage {
    Stage1Performing,
    Stage2Underperforming,
    Stage3NonPerforming,
    WrittenOff,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TradeCreditStatus {
    Outstanding,
    PartiallyPaid { amount_paid: Money },
    Paid,
    Overdue { days: u32 },
    Disputed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentTerms {
    pub net_days: u32,
    pub discount_rate: Option<f64>,
    pub discount_days: Option<u32>,
}

pub type LienId = Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lien {
    pub lien_id: LienId,
    pub secured_obligation: InstrumentId,
    pub collateral_type: CollateralType,
    pub status: LienStatus,
    pub priority: u32,
    pub created_date: chrono::NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CollateralType {
    Securities {
        instrument_id: InstrumentId,
        quantity: f64,
        haircut: f64,
    },
    RealEstate {
        property_id: Uuid,
        valuation: Money,
        ltv_limit: f64,
    },
    Receivables {
        debtor: AgentId,
        amount: Money,
    },
    Cash {
        amount: Money,
    },
    Other {
        description: String,
        value: Money,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LienStatus {
    Active,
    Released,
    Enforced,
    Subordinated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Covenant {
    pub covenant_type: CovenantType,
    pub status: CovenantStatus,
    pub test_frequency: TestFrequency,
    pub last_test_date: Option<chrono::NaiveDate>,
    pub cure_period_days: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CovenantType {
    DebtToEquity { max_ratio: f64 },
    InterestCoverage { min_ratio: f64 },
    CurrentRatio { min_ratio: f64 },
    MinimumNetWorth { amount: Money },
    MaximumCapex { amount: Money },
    RestrictedPayments,
    ChangeOfControl,
    CrossDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CovenantStatus {
    InCompliance,
    Breached { breach_date: chrono::NaiveDate },
    Waived { until_date: chrono::NaiveDate },
    Cured,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestFrequency {
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
    OnDemand,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CreditRegistry {
    pub applications: HashMap<Uuid, LoanApplication>,
    pub applications_by_bank: HashMap<AgentId, Vec<Uuid>>,
    pub applications_by_borrower: HashMap<AgentId, Vec<Uuid>>,

    pub facilities: HashMap<Uuid, CreditFacility>,
    pub facilities_by_lender: HashMap<AgentId, Vec<Uuid>>,
    pub facilities_by_borrower: HashMap<AgentId, Vec<Uuid>>,

    pub loans: HashMap<Uuid, Loan>,
    pub loans_by_lender: HashMap<AgentId, Vec<Uuid>>,
    pub loans_by_borrower: HashMap<AgentId, Vec<Uuid>>,
    pub loans_by_facility: HashMap<Uuid, Vec<Uuid>>,

    pub liens: HashMap<LienId, Lien>,
    pub liens_by_loan: HashMap<Uuid, Vec<LienId>>,
    pub liens_by_collateral: HashMap<InstrumentId, Vec<LienId>>,

    pub credit_histories: HashMap<AgentId, CreditHistory>,

    pub payment_schedules: HashMap<Uuid, PaymentSchedule>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreditFacility {
    pub instrument_id: InstrumentId,
    pub details: CreditLineDetails,
    pub status: FacilityStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Loan {
    pub instrument_id: InstrumentId,
    pub details: LoanDetails,
    pub status: LoanStatus,
    pub servicing_history: Vec<PaymentRecord>,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FacilityStatus {
    Active,
    Frozen,
    Expired,
    Terminated,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentSchedule {
    pub loan_id: Uuid,
    pub scheduled_payments: Vec<ScheduledPayment>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledPayment {
    pub payment_date: chrono::NaiveDate,
    pub principal_amount: Money,
    pub interest_amount: Money,
    pub fee_amount: Money,
    pub total_amount: Money,
    pub status: PaymentStatus,
    pub principal_payment_id: Option<Uuid>,
    pub interest_payment_id: Option<Uuid>,
    pub dpd_days: u32,
    pub paid_date: Option<chrono::NaiveDate>,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaymentStatus {
    Scheduled,
    Paid,
    PartiallyPaid,
    Missed,
    Deferred,
}

impl CreditRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_facility(&mut self, facility: CreditFacility) -> Result<(), String> {
        let facility_id = facility.details.facility_id;
        let lender = facility.details.lender;
        let borrower = facility.details.borrower;

        if self.facilities.contains_key(&facility_id) {
            return Err(format!("Facility {} already exists", facility_id));
        }

        self.facilities.insert(facility_id, facility);
        self.facilities_by_lender.entry(lender).or_default().push(facility_id);
        self.facilities_by_borrower.entry(borrower).or_default().push(facility_id);

        Ok(())
    }

    pub fn draw_from_facility(&mut self, facility_id: Uuid, loan: Loan, amount: Money) -> Result<(), String> {
        let facility =
            self.facilities.get_mut(&facility_id).ok_or_else(|| format!("Facility {} not found", facility_id))?;

        if facility.details.available_amount < amount {
            return Err(format!(
                "Insufficient availability: {} available, {} requested",
                facility.details.available_amount, amount
            ));
        }

        facility.details.available_amount -= amount;
        facility.details.drawn_amount += amount;
        facility.details.drawn_loans.push(loan.details.loan_id);

        let loan_id = loan.details.loan_id;
        let lender = loan.details.lender;
        let borrower = loan.details.borrower;

        self.loans.insert(loan_id, loan);
        self.loans_by_lender.entry(lender).or_default().push(loan_id);
        self.loans_by_borrower.entry(borrower).or_default().push(loan_id);
        self.loans_by_facility.entry(facility_id).or_default().push(loan_id);

        Ok(())
    }

    pub fn register_loan(&mut self, loan: Loan) -> Result<(), String> {
        let loan_id = loan.details.loan_id;
        let lender = loan.details.lender;
        let borrower = loan.details.borrower;

        if self.loans.contains_key(&loan_id) {
            return Err(format!("Loan {} already exists", loan_id));
        }

        self.loans.insert(loan_id, loan);
        self.loans_by_lender.entry(lender).or_default().push(loan_id);
        self.loans_by_borrower.entry(borrower).or_default().push(loan_id);

        Ok(())
    }

    pub fn create_lien(&mut self, lien: Lien, loan_id: Uuid) -> Result<(), String> {
        let lien_id = lien.lien_id;

        if self.liens.contains_key(&lien_id) {
            return Err(format!("Lien {} already exists", lien_id));
        }

        if let CollateralType::Securities { instrument_id, .. } = &lien.collateral_type {
            self.liens_by_collateral.entry(*instrument_id).or_default().push(lien_id);
        }

        self.liens.insert(lien_id, lien);
        self.liens_by_loan.entry(loan_id).or_default().push(lien_id);

        Ok(())
    }

    pub fn get_borrower_loans(&self, borrower: &AgentId) -> Vec<&Loan> {
        self.loans_by_borrower
            .get(borrower)
            .map(|ids| ids.iter().filter_map(|id| self.loans.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn get_lender_exposure(&self, lender: &AgentId) -> Money {
        self.loans_by_lender
            .get(lender)
            .map(|ids| {
                ids.iter().filter_map(|id| self.loans.get(id)).map(|loan| loan.details.outstanding_principal).sum()
            })
            .unwrap_or(Money::ZERO)
    }

    pub fn get_facility_utilization(&self, facility_id: &Uuid) -> Option<f64> {
        self.facilities.get(facility_id).map(|f| {
            let total = f.details.commitment_amount.to_f64();
            let drawn = f.details.drawn_amount.to_f64();
            if total > 0.0 { drawn / total } else { 0.0 }
        })
    }

    pub fn update_impairment(&mut self, loan_id: &Uuid, new_stage: ImpairmentStage) -> Result<(), String> {
        let loan = self.loans.get_mut(loan_id).ok_or_else(|| format!("Loan {} not found", loan_id))?;

        loan.details.impairment.stage = new_stage;

        if new_stage == ImpairmentStage::WrittenOff {
            let borrower = loan.details.borrower;
            let history = self.credit_histories.entry(borrower).or_default();
            history.total_defaults += 1;
        }

        Ok(())
    }
}

pub struct LoanServicer {
    pub current_date: NaiveDate,
    pub rate_environment: RateEnvironment,
}

#[derive(Clone, Debug, Default)]
pub struct RateEnvironment {
    pub rates: HashMap<RateIndex, Rate>,
}

impl LoanServicer {
    pub fn new(current_date: NaiveDate) -> Self {
        Self { current_date, rate_environment: RateEnvironment::default() }
    }

    pub fn accrue_interest(&self, loan: &mut LoanDetails) -> Money {
        if self.current_date <= loan.last_accrual_date {
            return Money::ZERO;
        }

        let _days = (self.current_date - loan.last_accrual_date).num_days() as f64;
        let year_frac = loan.day_count.year_fraction(loan.last_accrual_date, self.current_date);

        let rate = self.calculate_rate(loan);
        let interest = loan.outstanding_principal * rate * Rate::from_f64(year_frac).unwrap_or(Rate::ZERO);

        loan.accrued_interest += interest;
        loan.last_accrual_date = self.current_date;

        interest
    }

    pub fn calculate_rate(&self, loan: &LoanDetails) -> Rate {
        let base_rate = match &loan.reference_rate {
            Some(index) => self.rate_environment.rates.get(index).copied().unwrap_or(Rate::ZERO),
            None => Rate::ZERO,
        };

        let total_rate = base_rate + bps_to_decimal(loan.spread_bps);

        let mut final_rate = total_rate;
        if let Some(floor) = loan.rate_floor_bps {
            final_rate = final_rate.max(bps_to_decimal(floor));
        }
        if let Some(cap) = loan.rate_cap_bps {
            final_rate = final_rate.min(bps_to_decimal(cap));
        }

        final_rate
    }

    pub fn generate_schedule(&self, loan: &LoanDetails) -> PaymentSchedule {
        let mut scheduled_payments = Vec::new();
        let mut remaining_principal = loan.outstanding_principal;
        let mut current_date = loan.next_payment_date;

        while current_date <= loan.maturity_date && remaining_principal > Money::ZERO {
            let (principal_payment, interest_payment) = match loan.amortization {
                Amortization::Bullet => {
                    let interest = self.calculate_period_interest(loan, remaining_principal);
                    if current_date == loan.maturity_date {
                        (remaining_principal, interest)
                    } else {
                        (Money::ZERO, interest)
                    }
                }
                Amortization::Linear => {
                    let periods = self.count_payment_periods(loan);
                    let principal = loan.principal / periods as f64;
                    let interest = self.calculate_period_interest(loan, remaining_principal);
                    (principal, interest)
                }
                Amortization::Annuity => {
                    let payment = self.calculate_annuity_payment(loan);
                    let interest = self.calculate_period_interest(loan, remaining_principal);
                    let principal = (payment - interest).max(Money::ZERO);
                    (principal, interest)
                }
                Amortization::InterestOnly => {
                    let interest = self.calculate_period_interest(loan, remaining_principal);
                    if current_date == loan.maturity_date {
                        (remaining_principal, interest)
                    } else {
                        (Money::ZERO, interest)
                    }
                }
                Amortization::Custom => (Money::ZERO, Money::ZERO),
            };

            scheduled_payments.push(ScheduledPayment {
                payment_date: current_date,
                principal_amount: principal_payment,
                interest_amount: interest_payment,
                fee_amount: Money::ZERO,
                total_amount: principal_payment + interest_payment,
                status: PaymentStatus::Scheduled,
                principal_payment_id: None,
                interest_payment_id: None,
                dpd_days: 0,
                paid_date: None,
            });

            remaining_principal -= principal_payment;
            current_date = self.next_payment_date(current_date, &loan.payment_frequency);
        }

        PaymentSchedule { loan_id: loan.loan_id, scheduled_payments }
    }

    fn calculate_period_interest(&self, loan: &LoanDetails, principal: Money) -> Money {
        let rate = self.calculate_rate(loan);
        let period_frac = self.payment_frequency_to_year_frac(&loan.payment_frequency);
        principal * rate * Rate::from_f64(period_frac).unwrap_or(Rate::ZERO)
    }

    fn calculate_annuity_payment(&self, loan: &LoanDetails) -> Money {
        let rate = self.calculate_rate(loan);
        let period_rate =
            rate * Rate::from_f64(self.payment_frequency_to_year_frac(&loan.payment_frequency)).unwrap_or(Rate::ZERO);
        let n = self.count_payment_periods(loan) as i64;

        if n <= 0 {
            return loan.outstanding_principal;
        }

        if period_rate == Rate::ZERO {
            return loan.outstanding_principal / n as f64;
        }

        let v = Rate::ONE / (Rate::ONE + period_rate);
        let mut v_n = Rate::ONE;
        for _ in 0..n {
            v_n *= v;
        }

        let denom = Rate::ONE - v_n;
        if denom == Rate::ZERO {
            return loan.outstanding_principal;
        }

        let factor = period_rate / denom;
        loan.outstanding_principal * factor
    }

    fn count_payment_periods(&self, loan: &LoanDetails) -> u32 {
        let months = ((loan.maturity_date - self.current_date).num_days() as f64 / 30.0).ceil() as u32;
        match loan.payment_frequency {
            PaymentFrequency::Monthly => months,
            PaymentFrequency::Quarterly => months / 3,
            PaymentFrequency::SemiAnnual => months / 6,
            PaymentFrequency::Annual => months / 12,
            PaymentFrequency::InterestOnly => months,
        }
    }

    fn payment_frequency_to_year_frac(&self, freq: &PaymentFrequency) -> f64 {
        match freq {
            PaymentFrequency::Monthly => 1.0 / 12.0,
            PaymentFrequency::Quarterly => 0.25,
            PaymentFrequency::SemiAnnual => 0.5,
            PaymentFrequency::Annual => 1.0,
            PaymentFrequency::InterestOnly => 1.0 / 12.0,
        }
    }

    fn next_payment_date(&self, current: NaiveDate, freq: &PaymentFrequency) -> NaiveDate {
        match freq {
            PaymentFrequency::Monthly => add_months(current, 1),
            PaymentFrequency::Quarterly => add_months(current, 3),
            PaymentFrequency::SemiAnnual => add_months(current, 6),
            PaymentFrequency::Annual => add_months(current, 12),
            PaymentFrequency::InterestOnly => add_months(current, 1),
        }
    }
}
