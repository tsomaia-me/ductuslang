use std::ops::Range;

use super::syntax_kind::SyntaxKind;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Token {
    pub kind: SyntaxKind,
    pub range: Range<u32>,
}
