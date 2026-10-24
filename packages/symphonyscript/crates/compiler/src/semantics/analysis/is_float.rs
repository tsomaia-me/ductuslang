use symphonyscript_core::Scalar;

pub(crate) fn is_float(scalar: Scalar) -> bool {
    matches!(scalar, Scalar::F32 | Scalar::F64)
}
