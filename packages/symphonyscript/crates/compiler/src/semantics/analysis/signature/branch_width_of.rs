use symphonyscript_core::Scalar;

use super::arith_width::arith_width;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::width::Width;

pub(crate) fn branch_width_of(kind: Kind, a: Width, b: Width) -> Width {
    match kind {
        Kind::Int | Kind::Float => {
            let left = Sig::of(kind, a, false, None);
            let right = Sig::of(kind, b, false, None);
            arith_width(&left, &right)
        }
        Kind::Bool => Width::Fixed(Scalar::Bool),
        Kind::Str => Width::Fixed(Scalar::Str),
        Kind::Enum(id) => Width::Fixed(Scalar::Enum(id)),
        Kind::Node | Kind::Bare | Kind::Any | Kind::Error => Width::Flexible,
    }
}
