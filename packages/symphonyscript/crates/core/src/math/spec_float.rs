use crate::compiled::Builtin;

pub trait SpecFloat: Copy {
    fn spec_rem(self, divisor: Self) -> Self;
    fn spec_pow(self, exponent: Self) -> Self;
    fn spec_min(self, other: Self) -> Self;
    fn spec_max(self, other: Self) -> Self;
    fn spec_builtin(builtin: Builtin, args: &[Self]) -> Option<Self>;
}

macro_rules! spec_float {
    ($float:ty, $fmod:path, $pow:path, $fabs:path, $floor:path, $ceil:path, $round:path,
     $sqrt:path, $sin:path, $cos:path, $tan:path, $exp:path, $log:path) => {
        impl SpecFloat for $float {
            fn spec_rem(self, divisor: Self) -> Self {
                let remainder = $fmod(self, divisor);
                if remainder == 0.0 {
                    return if divisor.is_sign_negative() {
                        -0.0
                    } else {
                        0.0
                    };
                }
                if remainder.is_nan() {
                    return remainder;
                }
                if remainder.is_sign_negative() != divisor.is_sign_negative() {
                    let sum = remainder + divisor;
                    if sum == 0.0 {
                        return if divisor.is_sign_negative() {
                            -0.0
                        } else {
                            0.0
                        };
                    }
                    return sum;
                }
                remainder
            }

            fn spec_pow(self, exponent: Self) -> Self {
                $pow(self, exponent)
            }

            fn spec_min(self, other: Self) -> Self {
                if self.is_nan() {
                    return other;
                }
                if other.is_nan() {
                    return self;
                }
                if self == other {
                    return if self.is_sign_negative() { self } else { other };
                }
                if self < other { self } else { other }
            }

            fn spec_max(self, other: Self) -> Self {
                if self.is_nan() {
                    return other;
                }
                if other.is_nan() {
                    return self;
                }
                if self == other {
                    return if self.is_sign_negative() { other } else { self };
                }
                if self > other { self } else { other }
            }

            fn spec_builtin(builtin: Builtin, args: &[Self]) -> Option<Self> {
                Some(match (builtin, args) {
                    (Builtin::Abs, [x]) => $fabs(*x),
                    (Builtin::Min, [a, b]) => a.spec_min(*b),
                    (Builtin::Max, [a, b]) => a.spec_max(*b),
                    (Builtin::Clamp, [x, lo, hi]) => x.spec_max(*lo).spec_min(*hi),
                    (Builtin::Floor, [x]) => $floor(*x),
                    (Builtin::Ceil, [x]) => $ceil(*x),
                    (Builtin::Round, [x]) => $round(*x),
                    (Builtin::Sqrt, [x]) => $sqrt(*x),
                    (Builtin::Sin, [x]) => $sin(*x),
                    (Builtin::Cos, [x]) => $cos(*x),
                    (Builtin::Tan, [x]) => $tan(*x),
                    (Builtin::Exp, [x]) => $exp(*x),
                    (Builtin::Log, [x]) => $log(*x),
                    (Builtin::Lerp, [a, b, t]) => *a + (*b - *a) * *t,
                    (
                        Builtin::Abs
                        | Builtin::Min
                        | Builtin::Max
                        | Builtin::Clamp
                        | Builtin::Floor
                        | Builtin::Ceil
                        | Builtin::Round
                        | Builtin::Sqrt
                        | Builtin::Sin
                        | Builtin::Cos
                        | Builtin::Tan
                        | Builtin::Exp
                        | Builtin::Log
                        | Builtin::Lerp
                        | Builtin::Rand
                        | Builtin::Len
                        | Builtin::Bytes,
                        _,
                    ) => return None,
                })
            }
        }
    };
}

spec_float!(
    f32,
    libm::fmodf,
    libm::powf,
    libm::fabsf,
    libm::floorf,
    libm::ceilf,
    libm::roundf,
    libm::sqrtf,
    libm::sinf,
    libm::cosf,
    libm::tanf,
    libm::expf,
    libm::logf
);
spec_float!(
    f64,
    libm::fmod,
    libm::pow,
    libm::fabs,
    libm::floor,
    libm::ceil,
    libm::round,
    libm::sqrt,
    libm::sin,
    libm::cos,
    libm::tan,
    libm::exp,
    libm::log
);
