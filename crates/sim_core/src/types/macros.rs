#[macro_export]
macro_rules! good_id {
    ($slug:literal) => {
        $crate::goods::CATALOGUE
            .get_good_id_by_slug($slug)
            .expect(concat!("unknown good slug: ", $slug))
    };
}

#[macro_export]
macro_rules! recipe_id {
    ($name:literal) => {
        sim_types::goods::CATALOGUE
            .get_recipe_id_by_name($name)
            .expect(concat!("unknown recipe name: ", $name))
    };
}

#[macro_export]
macro_rules! cash {
    ($creditor:expr, $amount:expr, $cb_id:expr, $originated:expr) => {
        $crate::FinancialInstrument {
            id: $crate::InstrumentId(uuid::Uuid::new_v4()),
            creditor: $creditor,
            debtor: $cb_id,
            principal: $amount,
            details: Box::new($crate::CashDetails),
            originated_date: $originated,
            accrued_interest: 0.0,
            last_accrual_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! deposit {
    ($depositor:expr, $bank:expr, $amount:expr, $rate:expr, $originated:expr) => {
        $crate::FinancialInstrument {
            id: $crate::InstrumentId(uuid::Uuid::new_v4()),
            creditor: $depositor,
            debtor: $bank,
            principal: $amount,
            details: Box::new($crate::DemandDepositDetails { interest_rate: $rate }),
            originated_date: $originated,
            accrued_interest: 0.0,
            last_accrual_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! reserves {
    ($bank:expr, $cb_id:expr, $amount:expr, $originated:expr) => {
        $crate::FinancialInstrument {
            id: $crate::InstrumentId(uuid::Uuid::new_v4()),
            creditor: $bank,
            debtor: $cb_id,
            principal: $amount,
            details: Box::new($crate::CentralBankReservesDetails),
            originated_date: $originated,
            accrued_interest: 0.0,
            last_accrual_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! bond {
    ($investor:expr, $issuer:expr, $principal:expr, $coupon_rate:expr, $maturity_date:expr, $face_value:expr, $bond_type:expr, $frequency:expr, $tenor:expr, $originated:expr) => {
        $crate::FinancialInstrument {
            id: $crate::InstrumentId(uuid::Uuid::new_v4()),
            creditor: $investor,
            debtor: $issuer,
            principal: $principal,
            details: Box::new($crate::BondDetails {
                bond_type: $bond_type,
                coupon_rate: $coupon_rate,
                face_value: $face_value,
                maturity_date: $maturity_date,
                frequency: $frequency,
                tenor: $tenor, // Pass tenor
                quantity: 1,
            }),
            originated_date: $originated,
            accrued_interest: 0.0,
            last_accrual_date: $originated,
        }
    };
}

#[macro_export]
macro_rules! pserde {
    ($outer:ty, $inner:ty) => {
        impl std::fmt::Display for $outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        
        impl std::str::FromStr for $outer {
            type Err = <$inner as std::str::FromStr>::Err;
            
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.parse::<$inner>()?))
            }
        }
    };
}