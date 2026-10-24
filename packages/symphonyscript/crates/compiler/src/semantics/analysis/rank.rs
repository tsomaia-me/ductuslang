use symphonyscript_core::Scalar;

pub(crate) fn rank(scalar: Scalar) -> u8 {
    match scalar {
        Scalar::I32 => 0,
        Scalar::I64 => 1,
        Scalar::F32 => 2,
        Scalar::F64 => 3,
        Scalar::Bool | Scalar::Enum(_) | Scalar::Str => 4,
    }
}
