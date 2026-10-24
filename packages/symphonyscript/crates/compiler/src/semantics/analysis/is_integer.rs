use symphonyscript_core::Scalar;

pub(crate) fn is_integer(scalar: Scalar) -> bool {
    matches!(scalar, Scalar::I32 | Scalar::I64)
}
