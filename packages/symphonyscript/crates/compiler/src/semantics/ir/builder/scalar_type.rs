use symphonyscript_core::Scalar;

use crate::syntax::{SyntaxKind, TokenView};

const SCALAR_KEYWORDS: [(SyntaxKind, Scalar); 5] = [
    (SyntaxKind::I32Kw, Scalar::I32),
    (SyntaxKind::I64Kw, Scalar::I64),
    (SyntaxKind::F32Kw, Scalar::F32),
    (SyntaxKind::F64Kw, Scalar::F64),
    (SyntaxKind::BoolKw, Scalar::Bool),
];

pub(super) fn scalar_type(token: &TokenView<'_>) -> Option<Scalar> {
    let kind = token.kind();
    SCALAR_KEYWORDS
        .iter()
        .find(|(keyword, _)| *keyword == kind)
        .map(|(_, scalar)| *scalar)
}
