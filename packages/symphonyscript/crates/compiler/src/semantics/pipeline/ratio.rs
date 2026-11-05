use std::cmp::Ordering;
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Ratio {
    num: i128,
    den: i128,
}

impl Ratio {
    pub fn int(value: i128) -> Ratio {
        Ratio { num: value, den: 1 }
    }

    pub fn zero() -> Ratio {
        Ratio::int(0)
    }

    fn reduced(num: i128, den: i128) -> Ratio {
        let (num, den) = if den < 0 { (-num, -den) } else { (num, den) };
        let divisor = gcd(num.unsigned_abs(), den.unsigned_abs()).max(1);
        let divisor = i128::try_from(divisor).unwrap_or(1);
        Ratio {
            num: num / divisor,
            den: den / divisor,
        }
    }

    pub fn div(self, other: Ratio) -> Option<Ratio> {
        if other.num == 0 {
            return None;
        }
        Some(Ratio::reduced(
            self.num.saturating_mul(other.den),
            self.den.saturating_mul(other.num),
        ))
    }

    pub fn floor(self) -> i128 {
        self.num.div_euclid(self.den)
    }

    pub fn is_zero(self) -> bool {
        self.num == 0
    }

    pub fn is_negative(self) -> bool {
        self.num < 0
    }
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let rest = a % b;
        a = b;
        b = rest;
    }
    a
}

impl Add for Ratio {
    type Output = Ratio;

    fn add(self, other: Ratio) -> Ratio {
        Ratio::reduced(
            self.num
                .saturating_mul(other.den)
                .saturating_add(other.num.saturating_mul(self.den)),
            self.den.saturating_mul(other.den),
        )
    }
}

impl Sub for Ratio {
    type Output = Ratio;

    fn sub(self, other: Ratio) -> Ratio {
        self + (-other)
    }
}

impl Neg for Ratio {
    type Output = Ratio;

    fn neg(self) -> Ratio {
        Ratio {
            num: self.num.saturating_neg(),
            den: self.den,
        }
    }
}

impl Mul for Ratio {
    type Output = Ratio;

    fn mul(self, other: Ratio) -> Ratio {
        Ratio::reduced(
            self.num.saturating_mul(other.num),
            self.den.saturating_mul(other.den),
        )
    }
}

impl PartialEq for Ratio {
    fn eq(&self, other: &Ratio) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Ratio {}

impl PartialOrd for Ratio {
    fn partial_cmp(&self, other: &Ratio) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Ratio {
    fn cmp(&self, other: &Ratio) -> Ordering {
        self.num
            .saturating_mul(other.den)
            .cmp(&other.num.saturating_mul(self.den))
    }
}
