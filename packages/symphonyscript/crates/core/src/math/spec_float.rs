use crate::compiled::Builtin;

pub trait SpecFloat: Copy {
    fn spec_rem(self, divisor: Self) -> Self;
    fn spec_pow(self, exponent: Self) -> Self;
    fn spec_min(self, other: Self) -> Self;
    fn spec_max(self, other: Self) -> Self;
    fn spec_builtin(builtin: Builtin, args: &[Self]) -> Self;
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

            fn spec_builtin(builtin: Builtin, args: &[Self]) -> Self {
                let arg = |position: usize| args.get(position).copied().unwrap_or(0.0);
                match builtin {
                    Builtin::Abs => $fabs(arg(0)),
                    Builtin::Min => arg(0).spec_min(arg(1)),
                    Builtin::Max => arg(0).spec_max(arg(1)),
                    Builtin::Clamp => arg(0).spec_max(arg(1)).spec_min(arg(2)),
                    Builtin::Floor => $floor(arg(0)),
                    Builtin::Ceil => $ceil(arg(0)),
                    Builtin::Round => $round(arg(0)),
                    Builtin::Sqrt => $sqrt(arg(0)),
                    Builtin::Sin => $sin(arg(0)),
                    Builtin::Cos => $cos(arg(0)),
                    Builtin::Tan => $tan(arg(0)),
                    Builtin::Exp => $exp(arg(0)),
                    Builtin::Log => $log(arg(0)),
                    Builtin::Lerp => arg(0) + (arg(1) - arg(0)) * arg(2),
                    Builtin::Rand => 0.0,
                }
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
