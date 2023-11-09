use std::fmt::Display;
use std::ops::{Add, Div, Mul, Neg, Sub};

use crate::ratio::Ratio;

#[derive(Debug, Eq, PartialEq, PartialOrd, Clone, Copy, Hash)]
pub struct Imag {
    imag: Ratio,
}

impl Imag {
    pub fn new(imag: Ratio) -> Self {
        Self { imag }
    }

    pub fn int_new(i: i64) -> Self {
        Self {
            imag: Ratio::new(i, 1),
        }
    }

    pub fn float_new(f: f64, limit: Option<u64>) -> Self {
        Self {
            imag: Ratio::float_new(f, limit),
        }
    }

    pub fn zero() -> Self {
        Self {
            imag: Ratio::int_new(0),
        }
    }

    pub fn to_float(self) -> f64 {
        self.imag.to_float()
    }

    pub fn coefficient(&self) -> Ratio {
        self.imag
    }

    pub fn int_mul(&self, i: i64) -> Self {
        let n = self.coefficient() * Ratio::int_new(i);
        Self::new(n)
    }

    pub fn ratio_mul(&self, r: Ratio) -> Self {
        let n = self.coefficient() * r;
        Self::new(n)
    }

    pub fn int_div(&self, i: i64) -> Self {
        let n = self.coefficient() / Ratio::int_new(i);
        Self::new(n)
    }

    pub fn ratio_div(&self, r: Ratio) -> Self {
        let n = self.coefficient() / r;
        Self::new(n)
    }
}

impl Add for Imag {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            imag: self.imag + rhs.imag,
        }
    }
}

impl Sub for Imag {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            imag: self.imag - rhs.imag,
        }
    }
}

impl Mul for Imag {
    type Output = Ratio;

    fn mul(self, rhs: Self) -> Self::Output {
        self.imag * rhs.imag * Ratio::new(-1, 1)
    }
}

impl Div for Imag {
    type Output = Ratio;

    fn div(self, rhs: Self) -> Self::Output {
        self.imag / rhs.imag
    }
}

impl Neg for Imag {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Imag::int_new(-self.imag.numer())
    }
}

impl Display for Imag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.imag == Ratio::int_new(0) {
            write!(f, "0")
        } else {
            write!(f, "{}im", self.imag.to_float())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ratio::Ratio;

    use super::Imag;

    #[test]
    fn test_display() {
        let puls = Imag::int_new(10);
        println!("{puls}");
        let minus = Imag::int_new(-10);
        println!("{minus}");
        let zero = Imag::int_new(0);
        println!("{zero}");
    }

    #[test]
    fn test_add() {
        let a = Imag::int_new(1);
        let b = Imag::int_new(-1);
        assert_eq!(Imag::new(Ratio::int_new(0)), a + b);
    }

    #[test]
    fn test_sub() {
        let a = Imag::int_new(1);
        let b = Imag::int_new(1);
        assert_eq!(Imag::new(Ratio::int_new(0)), a - b);
    }
    #[test]
    fn test_mul() {
        let a = Imag::int_new(1);
        let b = Imag::int_new(1);
        assert_eq!(Ratio::new(-1, 1), a * b);
        let a = Imag::int_new(1);
        let b = Imag::int_new(-1);
        assert_eq!(Ratio::new(1, 1), a * b);
    }

    #[test]
    fn test_div() {
        let a = Imag::int_new(2);
        let b = Imag::int_new(3);
        assert_eq!(Ratio::new(2, 3), a / b);
        let a = Imag::int_new(3);
        let b = Imag::int_new(-7);
        assert_eq!(Ratio::new(-3, 7), a / b);
    }
}
