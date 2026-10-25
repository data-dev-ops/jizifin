//! Integer cent amounts. Decimal formatting happens only at the presentation edge.

use serde::{Deserialize, Serialize};
use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

/// A signed amount of money in whole cents.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cents(i64);

impl Cents {
    /// Zero cents.
    pub const ZERO: Self = Self(0);

    /// Wraps a raw cent value.
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// Raw cent value.
    pub const fn value(self) -> i64 {
        self.0
    }

    /// Absolute amount.
    pub const fn abs(self) -> Self {
        Self(self.0.abs())
    }

    /// True when the amount is exactly zero.
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// True when the amount is strictly positive.
    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }

    /// True when the amount is strictly negative.
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }
}

impl Add for Cents {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Cents {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Neg for Cents {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl AddAssign for Cents {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Cents {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Sum for Cents {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, Add::add)
    }
}
