use std::cmp::Ordering::{self, Equal, Greater, Less};
use std::fmt::Display;
use std::ops::Neg;
use std::ops::{Add, Div, Mul, Rem, Sub};

#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub struct Ratio {
    numer: i64,
    denom: i64,
}

impl Ratio {
    pub const fn new(numer: i64, denom: i64) -> Self {
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
    pub const fn zero() -> Self {
        Self { numer: 0, denom: 0 }
    }

    #[inline]
    pub fn one() -> Self {
        Self { numer: 1, denom: 1 }
    }

    const EPSILON: f64 = 1e-30;
    #[inline]
    pub fn float_new(f: f64) -> Self {
        let mut f = f;
        let mut minus = false;
        match f.partial_cmp(&0f64) {
            Some(Equal) => return Self::zero(),
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

    #[inline]
    pub fn to_float(self) -> f64 {
        self.numer as f64 / self.denom as f64
    }

    #[inline]
    pub fn to_int(self) -> i64 {
        self.numer / self.denom
    }

    #[inline]
    pub fn denom(&self) -> i64 {
        self.denom
    }

    #[inline]
    pub fn numer(&self) -> i64 {
        self.numer
    }

    #[inline]
    pub fn to_le_bytes(&self) -> Vec<u8> {
        [self.numer.to_le_bytes(), self.denom.to_le_bytes()].concat()
    }
}

impl Neg for Ratio {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self::new(-self.numer, self.denom)
    }
}

impl Add for Ratio {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let lcm = lcm(self.denom, rhs.denom);
        let denom = lcm;
        let numer = self.numer * lcm / self.denom + rhs.numer * lcm / rhs.denom;
        Self::new(numer, denom)
    }
}

impl Sub for Ratio {
    type Output = Self;

    #[inline]
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
        let ac = gcd(self.numer, rhs.denom);
        let bd = gcd(rhs.numer, self.denom);
        Self::new(
            self.numer / ac * rhs.numer / bd,
            self.denom / bd * rhs.denom / ac,
        )
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

impl Rem for Ratio {
    type Output = Self;

    #[inline]
    fn rem(self, rhs: Self) -> Self::Output {
        if self == rhs {
            return Self::zero();
        } else if rhs == Self::one() {
            return self;
        }
        let common_denom = gcd(self.denom, rhs.denom);
        let numer =
            (self.numer * (rhs.denom / common_denom)) % (rhs.numer * (self.denom / common_denom));
        let denom = self.denom * (rhs.denom / common_denom);
        Self::new(numer, denom)
    }
}

impl PartialOrd for Ratio {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.numer.partial_cmp(&other.numer) {
            Some(Equal) => {}
            ord => return ord,
        }
        self.denom.partial_cmp(&other.denom)
    }

    #[inline]
    fn lt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Less))
    }

    #[inline]
    fn le(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Less | Equal))
    }

    #[inline]
    fn gt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Greater))
    }

    #[inline]
    fn ge(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Greater | Equal))
    }
}

impl Display for Ratio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.numer, self.denom)
    }
}

pub const RATIO_E: Ratio = Ratio::new(268876667, 98914198);
pub const RATIO_TAU: Ratio = Ratio::new(411557987, 65501488);
pub const RATIO_EGAMMA: Ratio = Ratio::new(240627391, 416876058);
pub const RATIO_PHI: Ratio = Ratio::new(240627391, 416876058);

pub const RATIO_LN_2: Ratio = Ratio::new(49180508, 70952475);
pub const RATIO_LN_2_10: Ratio = Ratio::new(146964308, 44240665);
pub const RATIO_LN_2_E: Ratio = Ratio::new(161546953, 111975815);
pub const RATIO_LN_10: Ratio = Ratio::new(239263565, 103910846);
pub const RATIO_LN_10_2: Ratio = Ratio::new(44240665, 146964308);
pub const RATIO_LN_10_E: Ratio = Ratio::new(118568075, 273013082);

pub const RATIO_PI: Ratio = Ratio::new(245850922, 78256779);
pub const RATIO_PI_2: Ratio = Ratio::new(122925461, 78256779);
pub const RATIO_PI_3: Ratio = Ratio::new(112277827, 107217427);
pub const RATIO_PI_4: Ratio = Ratio::new(101534659, 129277943);
pub const RATIO_PI_6: Ratio = Ratio::new(69496223, 132728009);
pub const RATIO_PI_8: Ratio = Ratio::new(101534659, 258555886);
pub const RATIO_PI_10: Ratio = Ratio::new(122925461, 78256779);
pub const RATIO_FRAC_1_PI: Ratio = Ratio::new(78256779, 245850922);
pub const RATIO_FRAC_2_PI: Ratio = Ratio::new(78256779, 122925461);

pub const RATIO_SQRT_2: Ratio = Ratio::new(131836323, 93222358);
pub const RATIO_FRAC_2_SQRT_PI: Ratio = Ratio::new(37593262, 33316161);
pub const RATIO_FRAC_1_SQRT_2: Ratio = Ratio::new(131836323, 186444716);
pub const RATIO_FRAC_1_SQRT_3: Ratio = Ratio::new(109552575, 189750626);

const fn gcd(x: i64, y: i64) -> i64 {
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
    use std::f64::consts::{
        E, FRAC_1_PI, FRAC_1_SQRT_2, FRAC_2_PI, FRAC_PI_2, FRAC_PI_3, FRAC_PI_4, LN_10, LN_2,
        LOG10_2, LOG10_E, LOG2_10, LOG2_E, PI, SQRT_2, TAU,
    };

    use super::Ratio;
    use crate::ratio::{
        gcd, lcm, RATIO_E, RATIO_FRAC_1_PI, RATIO_FRAC_1_SQRT_2, RATIO_FRAC_2_PI, RATIO_LN_10,
        RATIO_LN_10_2, RATIO_LN_10_E, RATIO_LN_2, RATIO_LN_2_10, RATIO_LN_2_E, RATIO_PI,
        RATIO_PI_2, RATIO_PI_3, RATIO_PI_4, RATIO_SQRT_2, RATIO_TAU,
    };

    #[test]
    fn test_gcd() {
        assert_eq!(1, gcd(1, 1));
        assert_eq!(-1, gcd(-1, -1));

        assert_eq!(1, gcd(1, 1));
        assert_eq!(-1, gcd(-5, -7));
        assert_eq!(3, gcd(111, 30));

        assert_eq!(0, gcd(0, 0));
        assert_eq!(5, gcd(0, 5));
        assert_eq!(5, gcd(5, 0));
        assert_eq!(1, gcd(1, 1));
        assert_eq!(gcd(-1, 1), gcd(1, -1));
        assert_eq!(-1, gcd(-1, -1));

        assert_eq!(i64::MAX, gcd(i64::MAX, i64::MAX));
        assert_eq!(i64::MIN, gcd(i64::MIN, i64::MIN));
        assert_eq!(gcd(i64::MIN, i64::MAX), gcd(i64::MAX, i64::MIN));
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
        let a = Ratio::new(3, 2);
        let b = Ratio::new(2, 3);
        assert_eq!(Ratio::new(1, 1), a * b);
        let a = Ratio::new(i64::MAX, 2);
        let b = Ratio::new(2, i64::MAX);
        assert_eq!(Ratio::new(1, 1), a * b);
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
    fn test_rational_rem() {
        let a = Ratio::new(i64::MAX, i64::MIN + 1);
        let b = Ratio::new(i64::MAX, i64::MIN + 1);
        assert_eq!(Ratio::zero(), a % b);
        let a = Ratio::new(i64::MAX, 127);
        let b = Ratio::new(i64::MAX, 7);
        assert_eq!(Ratio::new(72624976668147841, 1), a % b);

        let a = Ratio::new(2, 1);
        let b = Ratio::new(10, 1);
        assert_eq!(Ratio::new(2, 1), a % b);
        let a = Ratio::new(3, 2);
        let b = Ratio::new(3, 2);
        assert_eq!(Ratio::zero(), a % b);
        let a = Ratio::new(5, 2);
        let b = Ratio::new(5, 3);
        assert_eq!(Ratio::new(5, 6), a % b);
        let a = Ratio::new(5, 2);
        let b = Ratio::new(5, 3);
        assert_eq!(Ratio::new(5, 6), a % b);
        let a = Ratio::new(7, 2);
        let b = Ratio::new(2, 5);
        assert_eq!(Ratio::new(3, 10), a % b);
    }

    #[test]
    fn test_float_new() {
        assert_eq!(Ratio::new(-1, 1), Ratio::float_new(-1.0));
        assert_eq!(Ratio::new(3, 10), Ratio::float_new(0.3));
        assert_eq!(Ratio::new(1, 7), Ratio::float_new(0.14285714285714285));
        assert_eq!(Ratio::new(1, 997), Ratio::float_new(0.0010030090270812437));
    }

    #[test]
    fn test_ratio_const_float() {
        assert_eq!(RATIO_E, Ratio::float_new(E));
        assert_eq!(RATIO_PI, Ratio::float_new(PI));
        assert_eq!(RATIO_TAU, Ratio::float_new(TAU));

        assert_eq!(RATIO_SQRT_2, Ratio::float_new(SQRT_2));
        assert_eq!(RATIO_FRAC_1_SQRT_2, Ratio::float_new(FRAC_1_SQRT_2));
        assert_eq!(RATIO_FRAC_1_SQRT_2, Ratio::float_new(FRAC_1_SQRT_2));

        assert_eq!(RATIO_LN_2, Ratio::float_new(LN_2));
        assert_eq!(RATIO_LN_10, Ratio::float_new(LN_10));
        assert_eq!(RATIO_LN_2_10, Ratio::float_new(LOG2_10));
        assert_eq!(RATIO_LN_2_E, Ratio::float_new(LOG2_E));
        assert_eq!(RATIO_LN_10_2, Ratio::float_new(LOG10_2));
        assert_eq!(RATIO_LN_10_E, Ratio::float_new(LOG10_E));

        assert_eq!(RATIO_PI_2, Ratio::float_new(FRAC_PI_2));
        assert_eq!(RATIO_PI_3, Ratio::float_new(FRAC_PI_3));
        assert_eq!(RATIO_PI_4, Ratio::float_new(FRAC_PI_4));
        assert_eq!(RATIO_FRAC_1_PI, Ratio::float_new(FRAC_1_PI));
        assert_eq!(RATIO_FRAC_2_PI, Ratio::float_new(FRAC_2_PI));
        assert_eq!(RATIO_FRAC_1_SQRT_2, Ratio::float_new(FRAC_1_SQRT_2));
    }
}
