use symphonyscript_core::BinaryOp;

pub(super) fn exact_binary(op: BinaryOp, a: i128, b: i128) -> Option<i128> {
    let value = match op {
        BinaryOp::Add => a.checked_add(b)?,
        BinaryOp::Sub => a.checked_sub(b)?,
        BinaryOp::Mul => a.checked_mul(b)?,
        BinaryOp::IntDiv => {
            if b == 0 {
                0
            } else {
                let quotient = a.checked_div(b)?;
                if (a % b != 0) && ((a < 0) != (b < 0)) {
                    quotient.checked_sub(1)?
                } else {
                    quotient
                }
            }
        }
        BinaryOp::Rem => {
            if b == 0 || b == -1 {
                0
            } else {
                let remainder = a % b;
                if remainder != 0 && ((remainder < 0) != (b < 0)) {
                    remainder.checked_add(b)?
                } else {
                    remainder
                }
            }
        }
        BinaryOp::Pow => exact_pow(a, b)?,
        _ => return None,
    };
    (value != i128::MIN).then_some(value)
}

fn exact_pow(base: i128, exponent: i128) -> Option<i128> {
    if exponent < 0 {
        return Some(match base {
            1 => 1,
            -1 => {
                if exponent % 2 == 0 {
                    1
                } else {
                    -1
                }
            }
            _ => 0,
        });
    }
    if base == 0 {
        return Some(if exponent == 0 { 1 } else { 0 });
    }
    if base == 1 {
        return Some(1);
    }
    if base == -1 {
        return Some(if exponent % 2 == 0 { 1 } else { -1 });
    }
    let exponent = u32::try_from(exponent).ok()?;
    base.checked_pow(exponent)
}

pub(super) fn fits(value: i128, bits: u32) -> bool {
    let max = (1i128 << (bits - 1)) - 1;
    let min = -(1i128 << (bits - 1));
    (min..=max).contains(&value)
}

const TWO_TO_THE_127: f64 = 170_141_183_460_469_231_731_687_303_715_884_105_728.0;

pub(crate) fn represents(integer: i128, float: f64) -> bool {
    float.is_finite() && float.abs() < TWO_TO_THE_127 && float as i128 == integer
}
