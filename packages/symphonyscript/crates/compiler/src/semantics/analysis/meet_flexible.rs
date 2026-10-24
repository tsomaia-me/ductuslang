use symphonyscript_core::Scalar;

use super::float_for::float_for;
use super::is_integer::is_integer;
use super::kind::Kind;

pub(crate) fn meet_flexible(fixed: Scalar, flexible: Kind) -> Scalar {
    if is_integer(fixed) && flexible == Kind::Float {
        float_for(fixed)
    } else {
        fixed
    }
}
