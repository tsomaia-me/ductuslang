use symphonyscript_core::Scalar;

pub(crate) fn widens(from: Scalar, to: Scalar) -> bool {
    from == to
        || matches!(
            (from, to),
            (Scalar::I32, Scalar::I64 | Scalar::F32 | Scalar::F64)
                | (Scalar::I64, Scalar::F64)
                | (Scalar::F32, Scalar::F64)
        )
}
