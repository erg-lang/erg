use std::fmt::Display;
use std::ops::{Add, Div, Mul, Neg, Sub};

use crate::{imag::Imag, ratio::Ratio};

#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub struct Complex {
    re: Ratio,
    im: Imag,
}

impl Complex {
    pub fn new(re: Ratio, im: Imag) -> Self {
        Self { re, im }
    }

    pub fn int_new(re: i64, im: i64) -> Self {
        Self {
            re: Ratio::new(re, 1),
            im: Imag::int_new(im),
        }
    }

    pub fn float_new(re: f64, im: f64, limit: Option<u64>) -> Self {
        Self {
            re: Ratio::float_new(re, limit),
            im: Imag::float_new(im, limit),
        }
    }

    pub fn im(&self) -> Ratio {
        self.im.coefficient()
    }

    pub fn re(&self) -> Ratio {
        self.re
    }

    fn conj(&self) -> Self {
        Self {
            re: self.re(),
            im: Imag::new(-self.im()),
        }
    }
}

impl Add for Complex {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let re = self.re() + rhs.re();
        let im = Imag::new(self.im() + rhs.im());
        Self::new(re, im)
    }
}

impl Sub for Complex {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let re = self.re() - rhs.re();
        let im = Imag::new(self.im() - rhs.im());
        Self::new(re, im)
    }
}

impl Mul for Complex {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let re = self.re() * rhs.re() - self.im() * rhs.im();
        let im = Imag::new(self.re() * rhs.im() + self.im() * rhs.re());
        Self::new(re, im)
    }
}

impl Div for Complex {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let denom = (rhs * rhs.conj()).re();
        let numer = self * rhs.conj();
        let re = numer.re() / denom;
        let im = Imag::new(numer.im() / denom);
        Self::new(re, im)
    }
}

impl Neg for Complex {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            re: -self.re,
            im: -self.im,
        }
    }
}

impl Display for Complex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let re = self.re.to_float();
        let im = self.im.to_float();
        let im_zero = Imag::zero();
        if self.im == im_zero {
            write!(f, "{}", re)
        } else if self.re == Ratio::int_new(0) {
            write!(f, "{}im", im)
        } else if self.im > im_zero {
            write!(f, "{}+{}im", re, im)
        } else {
            write!(f, "{}{}im", re, im)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{imag::Imag, ratio::Ratio};

    use super::Complex;

    #[test]
    fn test_display() {
        let a = Complex::int_new(1, 3);
        println!("{a}");
        let a = Complex::int_new(1, -3);
        println!("{a}");
        let a = Complex::int_new(1, 0);
        println!("{a}");
        let a = Complex::int_new(0, 3);
        println!("{a}");
        let a = Complex::int_new(0, 0);
        println!("{a}");
    }

    #[test]
    fn test_add() {
        let a = Complex::int_new(1, 1);
        let b = Complex::int_new(1, 1);
        assert_eq!(Complex::int_new(2, 2), a + b);
        let a = Complex::int_new(1, 2);
        let b = Complex::int_new(5, 7);
        assert_eq!(Complex::int_new(6, 9), a + b);
    }

    #[test]
    fn test_mul() {
        let a = Complex::int_new(1, 1);
        let b = Complex::int_new(1, 1);
        assert_eq!(Complex::int_new(0, 2), a * b);
        let a = Complex::int_new(1, 2);
        let b = Complex::int_new(5, 7);
        assert_eq!(Complex::int_new(-9, 17), a * b);
        let a = Complex::int_new(2, 2);
        let b = a.conj();
        assert_eq!(Complex::int_new(8, 0), a * b);
    }

    #[test]
    fn test_div() {
        let a = Complex::int_new(1, 1);
        let b = Complex::int_new(1, 1);
        assert_eq!(Complex::int_new(1, 0), a / b);
        let a = Complex::int_new(1, 2);
        let b = Complex::int_new(5, 7);
        assert_eq!(
            Complex::new(Ratio::new(19, 74), Imag::new(Ratio::new(3, 74))),
            a / b
        );
    }
}
