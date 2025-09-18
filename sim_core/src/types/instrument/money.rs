use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

pub type Rate = Decimal;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct Money(pub Decimal);

impl Money {
    pub const ZERO: Money = Money(Decimal::ZERO);
    pub const ONE: Money = Money(Decimal::ONE);
    pub fn from_f64(val: f64) -> Option<Self> {
        Decimal::from_f64(val).map(Money)
    }

    pub fn from_str(val: &str) -> Option<Self> {
        Decimal::from_str_exact(val).ok().map(Money)
    }

    pub fn to_f64(&self) -> f64 {
        self.0.to_f64().unwrap_or(f64::NAN)
    }
}

impl From<i32> for Money {
    fn from(val: i32) -> Self {
        Money(Decimal::from(val))
    }
}
impl From<i64> for Money {
    fn from(val: i64) -> Self {
        Money(Decimal::from(val))
    }
}
impl From<u32> for Money {
    fn from(val: u32) -> Self {
        Money(Decimal::from(val))
    }
}
impl From<u64> for Money {
    fn from(val: u64) -> Self {
        Money(Decimal::from(val))
    }
}

impl Add for Money {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Money(self.0 + rhs.0)
    }
}
impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for Money {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Money(self.0 - rhs.0)
    }
}
impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Mul<Rate> for Money {
    type Output = Self;
    fn mul(self, rhs: Rate) -> Self::Output {
        Money(self.0 * rhs)
    }
}

impl Mul<Money> for f64 {
    type Output = Money;
    fn mul(self, rhs: Money) -> Self::Output {
        if let Some(decimal_self) = Decimal::from_f64(self) { Money(decimal_self * rhs.0) } else { Money::ZERO }
    }
}

impl Div<Rate> for Money {
    type Output = Self;
    fn div(self, rhs: Rate) -> Self::Output {
        Money(self.0 / rhs)
    }
}

impl Div<Money> for Money {
    type Output = Rate;
    fn div(self, rhs: Money) -> Self::Output {
        self.0 / rhs.0
    }
}

impl Neg for Money {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Money(-self.0)
    }
}

impl Mul<f64> for Money {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        let rhs_dec = Decimal::from_f64(rhs).unwrap_or_default();
        Money(self.0 * rhs_dec)
    }
}

impl Div<f64> for Money {
    type Output = Self;
    fn div(self, rhs: f64) -> Self::Output {
        let rhs_dec = Decimal::from_f64(rhs).unwrap_or_default();
        if rhs_dec.is_zero() {
            return Money::ZERO;
        }
        Money(self.0 / rhs_dec)
    }
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Self {
        iter.fold(Money::ZERO, |acc, x| acc + x)
    }
}

impl<'a> Sum<&'a Money> for Money {
    fn sum<I: Iterator<Item = &'a Money>>(iter: I) -> Self {
        iter.fold(Money::ZERO, |acc, x| acc + *x)
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Add<Decimal> for Money {
    type Output = Self;
    fn add(self, rhs: Decimal) -> Self::Output {
        Money(self.0 + rhs)
    }
}
impl Add<i64> for Money {
    type Output = Self;
    fn add(self, rhs: i64) -> Self::Output {
        Money(self.0 + Decimal::from(rhs))
    }
}
impl Add<u64> for Money {
    type Output = Self;
    fn add(self, rhs: u64) -> Self::Output {
        Money(self.0 + Decimal::from(rhs))
    }
}
impl Add<f64> for Money {
    type Output = Self;
    fn add(self, rhs: f64) -> Self::Output {
        if let Some(rhs_dec) = Decimal::from_f64(rhs) { Money(self.0 + rhs_dec) } else { self }
    }
}
impl Mul<i32> for Money {
    type Output = Self;
    fn mul(self, rhs: i32) -> Self::Output {
        Money(self.0 * Decimal::from(rhs))
    }
}
