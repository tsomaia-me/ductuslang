use symphonyscript_core::Scalar;

pub(crate) fn is_integer(scalar: Scalar) -> bool {
    matches!(scalar, Scalar::U8 | Scalar::I32 | Scalar::I64)
}
