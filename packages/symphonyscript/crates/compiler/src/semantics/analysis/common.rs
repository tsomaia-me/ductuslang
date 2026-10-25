use symphonyscript_core::Scalar;

use super::widens::widens;

pub(crate) fn common(a: Scalar, b: Scalar) -> Option<Scalar> {
    if widens(a, b) {
        return Some(b);
    }
    if widens(b, a) {
        return Some(a);
    }
    let numeric = |scalar| {
        matches!(
            scalar,
            Scalar::U8 | Scalar::I32 | Scalar::I64 | Scalar::F32 | Scalar::F64
        )
    };
    (numeric(a) && numeric(b)).then_some(Scalar::F64)
}
