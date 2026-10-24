use symphonyscript_core::Scalar;

use super::kind::Kind;

pub(crate) fn kind_of(scalar: Scalar) -> Kind {
    match scalar {
        Scalar::I32 | Scalar::I64 => Kind::Int,
        Scalar::F32 | Scalar::F64 => Kind::Float,
        Scalar::Bool => Kind::Bool,
        Scalar::Enum(id) => Kind::Enum(id),
        Scalar::Str => Kind::Str,
    }
}
