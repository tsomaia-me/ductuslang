use num_bigint::BigUint;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct FloatLit {
    pub negative: bool,
    pub digits: String,
    pub exponent: i64,
}

impl FloatLit {
    pub fn negated(&self) -> FloatLit {
        FloatLit {
            negative: !self.negative,
            ..self.clone()
        }
    }

    pub fn from_integer(value: i128) -> FloatLit {
        FloatLit {
            negative: value < 0,
            digits: value.unsigned_abs().to_string(),
            exponent: 0,
        }
    }

    pub fn to_f64(&self) -> f64 {
        self.text().parse().unwrap_or(f64::NAN)
    }

    pub fn to_f32(&self) -> f32 {
        self.text().parse().unwrap_or(f32::NAN)
    }

    pub fn scaled(&self, scale: f64) -> FloatLit {
        let bits = scale.to_bits();
        let raw_exponent = ((bits >> 52) & 0x7ff) as i64;
        let fraction = bits & ((1u64 << 52) - 1);
        let (mantissa, binary_exponent) = if raw_exponent == 0 {
            (fraction, -1074)
        } else {
            (fraction | (1u64 << 52), raw_exponent - 1075)
        };
        let digits: BigUint = self.digits.parse().unwrap_or_default();
        let mut product = digits * BigUint::from(mantissa);
        let mut exponent = self.exponent;
        if binary_exponent >= 0 {
            product <<= binary_exponent as u64;
        } else {
            let fives = BigUint::from(5u32).pow(binary_exponent.unsigned_abs() as u32);
            product *= fives;
            exponent = exponent.saturating_add(binary_exponent);
        }
        FloatLit {
            negative: self.negative,
            digits: product.to_string(),
            exponent,
        }
    }

    fn text(&self) -> String {
        let sign = if self.negative { "-" } else { "" };
        format!("{sign}{}e{}", self.digits, self.exponent)
    }
}
