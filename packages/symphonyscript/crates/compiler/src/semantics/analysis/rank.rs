use symphonyscript_core::Scalar;

pub(crate) fn rank(scalar: Scalar) -> u8 {
    match scalar {
        Scalar::U8 => 0,
        Scalar::I32 => 1,
        Scalar::I64 => 2,
        Scalar::F32 => 3,
        Scalar::F64 => 4,
        Scalar::Bool | Scalar::Enum(_) | Scalar::Str => 5,
    }
}
