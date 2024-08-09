use std::cmp::Ordering::{self, Equal, Greater, Less};
use std::fmt::Display;
use std::ops::Neg;
use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub struct Ratio {
    numer: i64,
    denom: i64,
}

impl Ratio {
    pub fn new(numer: i64, denom: i64) -> Self {
        if numer == 0 {
            return Self::zero();
        }
        if denom == 0 {
            panic!("Zero is an invalid denominator!");
        }

        let gcd = gcd(numer, denom);
        Self {
            numer: numer / gcd,
            denom: denom / gcd,
        }
    }

    pub fn zero() -> Self {
        Self { numer: 0, denom: 1 }
    }

    const EPSILON: f64 = 1e-10;
    pub fn float_new(f: f64) -> Self {
        let mut f = f;
        let mut minus = false;
        match f.partial_cmp(&0f64) {
            Some(Equal) => return Ratio::zero(),
            Some(Less) => {
                minus = true;
                f *= -1.0;
            }
            Some(_) => {}
            None => panic!("Something went wrong: {f} cannot compare"),
        }
        let mut n: i64 = 1;
        let mut d: i64 = 1;
        let mut error: f64 = (f - n as f64 / d as f64).abs();
        while error > Self::EPSILON {
            if f > n as f64 / d as f64 {
                n += 1;
            } else {
                d += 1;
            }
            let new_error = (f - n as f64 / d as f64).abs();
            error = new_error;
        }
        if minus {
            n *= -1;
        }
        Ratio::new(n, d)
    }

    pub fn to_float(self) -> f64 {
        self.numer as f64 / self.denom as f64
    }

    pub fn to_int(self) -> i64 {
        self.numer / self.denom
    }

    pub fn denom(&self) -> i64 {
        self.denom
    }

    pub fn numer(&self) -> i64 {
        self.numer
    }

    pub fn to_le_bytes(&self) -> Vec<u8> {
        [self.numer.to_le_bytes(), self.denom.to_le_bytes()].concat()
    }
}

impl Neg for Ratio {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.numer, self.denom)
    }
}

impl Add for Ratio {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let lcm = lcm(self.denom, rhs.denom);
        let denom = lcm;
        let numer = self.numer * lcm / self.denom + rhs.numer * lcm / rhs.denom;
        Self::new(numer, denom)
    }
}

impl Sub for Ratio {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let lcm = lcm(self.denom, rhs.denom);
        let denom = lcm;
        let numer = self.numer * lcm / self.denom - rhs.numer * lcm / rhs.denom;
        Self::new(numer, denom)
    }
}

impl Mul for Ratio {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let numer = self.numer * rhs.numer;
        let denom = self.denom * rhs.denom;
        Self::new(numer, denom)
    }
}

impl Div for Ratio {
    type Output = Self;

    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        let ac = gcd(self.numer, rhs.numer);
        let bd = gcd(self.denom, rhs.denom);
        Self::new(self.numer / ac, self.denom / bd * (rhs.numer / ac))
    }
}

impl PartialOrd for Ratio {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.numer.partial_cmp(&other.numer) {
            Some(Equal) => {}
            ord => return ord,
        }
        self.denom.partial_cmp(&other.denom)
    }

    fn lt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Less))
    }

    fn le(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Less | Equal))
    }

    fn gt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Greater))
    }

    fn ge(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Greater | Equal))
    }
}

impl Display for Ratio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} / {}", self.numer, self.denom)
    }
}

fn gcd(x: i64, y: i64) -> i64 {
    if y > x {
        return gcd(y, x);
    }
    let mut x = x;
    let mut y = y;
    while y != 0 {
        let t = y;
        y = x % y;
        x = t;
    }
    x
}

fn lcm(x: i64, y: i64) -> i64 {
    if x == 0 || y == 0 {
        return 0;
    }
    x * y / gcd(x, y)
}

#[cfg(test)]
mod test {
    use super::Ratio;
    use crate::ratio::{gcd, lcm};

    #[test]
    fn test_gcd() {
        assert_eq!(1, gcd(1, 1));
        assert_eq!(-1, gcd(-1, -1));

        assert_eq!(0, gcd(0, 0));
        assert_eq!(5, gcd(0, 5));
        assert_eq!(5, gcd(5, 0));
        assert_eq!(1, gcd(1, 1));
        assert_eq!(gcd(-1, 1), gcd(1, -1));
        assert_eq!(-1, gcd(-1, -1));

        assert_eq!(i64::MAX, gcd(i64::MAX, i64::MAX));
        assert_eq!(i64::MIN, gcd(i64::MIN, i64::MIN));
        assert_eq!(gcd(i64::MIN, i64::MAX), gcd(i64::MAX, i64::MIN));
        assert_eq!(gcd(i64::MIN + 1, i64::MAX), gcd(i64::MAX, i64::MIN + 1));
        assert_eq!(gcd(0, i64::MAX), gcd(i64::MAX, 0));
        assert_eq!(gcd(i64::MIN, 1), gcd(1, i64::MIN));

        assert_eq!(6, gcd(54, 24));
        assert_eq!(-6, gcd(-54, 24));
        assert_eq!(6, gcd(54, -24));
        assert_eq!(-6, gcd(-54, -24));
        assert_eq!(600, gcd(239520000, 1293400200));
    }

    #[test]
    fn test_lcm() {
        assert_eq!(91, lcm(7, 13));
        assert_eq!(6, lcm(2, -3));
        assert_eq!(-6, lcm(-2, 3));
        assert_eq!(-6, lcm(-2, -3));
        assert_eq!(-65, lcm(-5, 13));
        assert_eq!(-91, lcm(-7, -13));
        assert_eq!(lcm(0, 7), lcm(7, 0));
        assert_eq!(0, lcm(0, 0));
        assert_eq!(-1, lcm(-1, -1));
    }

    #[test]
    fn test_rational_add() {
        let a = Ratio::new(2, 1);
        let b = Ratio::new(10, 1);
        assert_eq!(Ratio::new(12, 1), a + b);
        let a = Ratio::new(1, 1);
        let b = Ratio::new(1, -1);
        assert_eq!(Ratio::zero(), a + b);
        let a = Ratio::new(1, 1);
        let b = Ratio::new(-1, 1);
        assert_eq!(Ratio::zero(), a + b);
        let a = Ratio::new(1, 1);
        let b = Ratio::new(-1, i64::MAX);
        assert_eq!(Ratio::new(i64::MAX - 1, i64::MAX), a + b);
    }

    #[test]
    fn test_rational_sub() {
        let a = Ratio::new(2, 1);
        let b = Ratio::new(10, 1);
        assert_eq!(Ratio::new(-8, 1), a - b);
        let a = Ratio::new(1, 1);
        let b = Ratio::new(1, 1);
        assert_eq!(Ratio::zero(), a - b);
        let a = Ratio::new(i64::MAX, i64::MAX);
        let b = Ratio::new(1, 1);
        assert_eq!(Ratio::zero(), a - b);
    }

    #[test]
    fn test_rational_mul() {
        let a = Ratio::new(2, 1);
        let b = Ratio::new(10, 1);
        assert_eq!(Ratio::new(20, 1), a * b);
        let a = Ratio::new(0, 1);
        let b = Ratio::new(10, 1);
        assert_eq!(Ratio::zero(), a * b);
        let a = Ratio::new(10, 1);
        let b = Ratio::new(0, 1);
        assert_eq!(Ratio::zero(), a * b);
    }

    #[test]
    fn test_rational_div() {
        let a = Ratio::new(2, 1);
        let b = Ratio::new(10, 1);
        assert_eq!(Ratio::new(1, 5), a / b);
        let a = Ratio::new(2, 1);
        let b = Ratio::new(2, 1);
        assert_eq!(Ratio::new(1, 1), a / b);
        let a = Ratio::new(i64::MAX, i64::MIN + 1);
        let b = Ratio::new(i64::MAX, i64::MIN + 1);
        assert_eq!(Ratio::new(1, 1), a / b);
    }

    #[test]
    fn test_float_new() {
        let a = Ratio::float_new(1.0);
        assert_eq!(Ratio::new(1, 1), a);
        let a = Ratio::float_new(-1.0);
        assert_eq!(Ratio::new(-1, 1), a);
        let a = Ratio::float_new(2.7);
        assert_eq!(Ratio::new(27, 10), a);
        let a = Ratio::float_new(1.0);
        assert_eq!(Ratio::new(1, 1), a);
        let a = Ratio::float_new(0.3333333333);
        assert_eq!(Ratio::new(1, 3), a);
        let a = Ratio::float_new(1.47);
        assert_eq!(Ratio::new(147, 100), a);
    }
}
