use std::cmp::Ordering::{self, Equal, Greater, Less};
use std::fmt::Display;
use std::ops::{Add, Div, Mul, Rem, Sub};

#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub struct Ratio {
    numer: i64,
    denom: i64,
}

impl Ratio {
    #[inline]
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

    #[inline]
    pub fn zero() -> Self {
        Self { numer: 0, denom: 1 }
    }

    const EPSILON: f64 = 1e-10;
    #[inline]
    pub fn float_new(f: f64, limit: Option<u64>) -> Self {
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
        let limit = if let Some(limit) = limit {
            if limit >= i32::MAX as u64 {
                i32::MAX as u64
            } else {
                limit
            }
        } else {
            10000
        };
        let mut n: i64 = 1;
        let mut d: i64 = 1;
        let mut error: f64 = (f - n as f64 / d as f64).abs();
        let mut cnt = 0;
        while error > Self::EPSILON {
            if cnt > limit {
                break;
            }
            if f > n as f64 / d as f64 {
                n += 1;
            } else {
                d += 1;
            }
            let new_error = (f - n as f64 / d as f64).abs();
            error = new_error;
            cnt += 1;
        }
        if minus {
            n *= -1;
        }
        Ratio::new(n, d)
    }

    #[inline]
    pub fn to_float(self) -> f64 {
        self.numer as f64 / self.denom as f64
    }

    pub fn denom(&self) -> i64 {
        self.denom
    }
    pub fn numer(&self) -> i64 {
        self.numer
    }

    #[inline]
    pub fn floor(&self) -> Self {
        if *self < Ratio::zero() {
            Ratio::new((self.numer - self.denom + 1) / self.denom, 1)
        } else {
            Ratio::new(self.numer / self.denom, 1)
        }
    }

    #[inline]
    pub fn ceil(&self) -> Self {
        if *self < Ratio::zero() {
            Ratio::new(self.numer / self.denom, 1)
        } else {
            Ratio::new((self.numer + self.denom - 1) / self.denom, 1)
        }
    }

    #[inline]
    pub fn pow(&self, n: u32) -> Self {
        let numer = self.numer.pow(n);
        let denom = self.denom.pow(n);
        Self::new(numer, denom)
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

    fn mul(self, rhs: Self) -> Self {
        let numer = self.numer * rhs.numer;
        let denom = self.denom * rhs.denom;
        Self::new(numer, denom)
    }
}

impl Div for Ratio {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let numer = self.numer / rhs.denom;
        let denom = self.denom / rhs.numer;
        Self::new(numer, denom)
    }
}

impl Rem for Ratio {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        let sd = self.denom;
        let od = rhs.denom;
        Ratio::new((self.numer * od) % (rhs.numer * sd), sd * od)
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
        write!(f, "{}, {}", self.numer, self.denom)
    }
}

fn gcd(x: i64, y: i64) -> i64 {
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
    x * y / gcd(x, y)
}

#[cfg(test)]
mod test {

    use std::f64::consts::PI;

    use super::Ratio;
    use crate::ratio::{gcd, lcm};

    #[test]
    fn test_gcd() {
        let a = 7;
        let b = 2;
        assert_eq!(1, gcd(a, b));
        let a = 84;
        let b = 54;
        assert_eq!(6, gcd(a, b));
        let a = 1;
        let b = 1;
        assert_eq!(1, gcd(a, b));
        let a = -2;
        let b = -3;
        assert_eq!(-1, gcd(a, b));
        let a = -6;
        let b = -12;
        assert_eq!(-6, gcd(a, b));
        let a = 6;
        let b = -12;
        assert_eq!(6, gcd(a, b));
        let a = 0;
        let b = -1;
        assert_eq!(-1, gcd(a, b));
        let a = -1;
        let b = 0;
        assert_eq!(-1, gcd(a, b));
    }

    #[test]
    fn test_lcm() {
        let a = 7;
        let b = 13;
        assert_eq!(91, lcm(a, b));
        let a = 2;
        let b = -3;
        assert_eq!(6, lcm(a, b));
        let a = -5;
        let b = 13;
        assert_eq!(-65, lcm(a, b));
        let a = -7;
        let b = -13;
        assert_eq!(-91, lcm(a, b));
        let a = 0;
        let b = 7;
        assert_eq!(0, lcm(a, b));
        let a = 13;
        let b = 0;
        assert_eq!(0, lcm(a, b));
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
    }

    #[test]
    fn test_rational_sub() {
        let a = Ratio::new(2, 1);
        let b = Ratio::new(10, 1);
        assert_eq!(Ratio::new(-8, 1), a - b);
        let a = Ratio::new(1, 1);
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
        assert_eq!(Ratio::new(0, 0), a * b);
    }

    #[test]
    fn test_rational_div() {
        let a = Ratio::new(2, 1);
        let b = Ratio::new(10, 1);
        assert_eq!(Ratio::new(1, 5), a / b);
        let a = Ratio::new(2, 1);
        let b = Ratio::new(2, 1);
        assert_eq!(Ratio::new(1, 1), a / b);
    }

    #[test]
    fn test_float_new() {
        let a = Ratio::float_new(1.0, None);
        assert_eq!(Ratio::new(1, 1), a);
        let a = Ratio::float_new(-1.0, None);
        assert_eq!(Ratio::new(-1, 1), a);
        let a = Ratio::float_new(2.7, None);
        assert_eq!(Ratio::new(27, 10), a);
        let a = Ratio::float_new(1.0, None);
        assert_eq!(Ratio::new(1, 1), a);
        let a = Ratio::float_new(0.3333333333, None);
        assert_eq!(Ratio::new(1, 3), a);
        let a = Ratio::float_new(1.47, None);
        assert_eq!(Ratio::new(147, 100), a);
    }

    #[test]
    fn test_floor() {
        let a = Ratio::new(1, 3);
        assert_eq!(Ratio::new(0, 0), a.floor());
        let a = Ratio::new(100, 3);
        assert_eq!(Ratio::new(33, 1), a.floor());
        let a = Ratio::new(-100, 3);
        assert_eq!(Ratio::new(-33, 1), a.floor());
        let a = Ratio::float_new(10.0, None);
        assert_eq!(10.0, a.floor().to_float());
        let a = Ratio::float_new(-0.8, None);
        assert_eq!(-1.0, a.floor().to_float());
    }

    #[test]
    fn test_ceil() {
        let a = Ratio::new(1, 3);
        assert_eq!(Ratio::new(1, 1), a.ceil());
        let a = Ratio::new(100, 3);
        assert_eq!(Ratio::new(34, 1), a.ceil());
        let a = Ratio::new(-100, 3);
        assert_eq!(Ratio::new(-32, 1), a.ceil());
        let a = Ratio::float_new(10.5, None);
        assert_eq!(11.0, a.ceil().to_float());
    }

    #[test]
    fn test_pow() {
        let a = Ratio::new(1, 3);
        assert_eq!(Ratio::new(1, 9), a.pow(2));
        assert_eq!(Ratio::new(1, 27), a.pow(3));
        let a = Ratio::new(1, 2);
        assert_eq!(Ratio::new(1, 1024), a.pow(10));
        let a = Ratio::new(1, -4);
        assert_eq!(Ratio::new(1, 16), a.pow(2));
        assert_eq!(Ratio::new(-1, 64), a.pow(3));
    }

    #[test]
    fn test_rem() {
        let a = Ratio::float_new(100.0, None);
        let b = Ratio::float_new(20.0, None);
        assert_eq!(Ratio::zero(), a % b);
        let a = Ratio::float_new(3.0, None);
        let b = Ratio::float_new(2.0, None);
        assert_eq!(Ratio::new(1, 1), a % b);
        let a = Ratio::float_new(3.5, None);
        let b = Ratio::float_new(2.2, None);
        assert_eq!(Ratio::new(13, 10), a % b);
    }

    #[test]
    fn test_limited_float_new() {
        let a = Ratio::float_new(PI, Some(10));
        assert_eq!(Ratio::new(10, 3), a);
        let a = Ratio::float_new(PI, Some(100));
        assert_eq!(Ratio::new(78, 25), a);
        let a = Ratio::float_new(PI, Some(1_000));
        assert_eq!(Ratio::new(761, 242), a);
        let a = Ratio::float_new(PI, Some(10_000));
        assert_eq!(Ratio::new(7587, 2416), a);
    }

    #[test]
    #[should_panic]
    fn test_overflow() {
        let a = Ratio::new(i64::MAX, 1);
        let b = Ratio::new(1, 1);
        let _ = a + b;
    }
}
