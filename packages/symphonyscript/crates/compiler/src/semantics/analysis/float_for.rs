use symphonyscript_core::Scalar;

pub(crate) fn float_for(scalar: Scalar) -> Scalar {
    match scalar {
        Scalar::I64 | Scalar::F64 => Scalar::F64,
        Scalar::U8 | Scalar::I32 | Scalar::F32 | Scalar::Bool | Scalar::Enum(_) | Scalar::Str => {
            Scalar::F32
        }
    }
}
