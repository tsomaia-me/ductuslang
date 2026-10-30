use crate::syntax::syntax_kind::SyntaxKind;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LoopHeader {
    Closed,
    Names,
    Iterable,
}

impl LoopHeader {
    pub(super) fn after(self, kind: SyntaxKind) -> LoopHeader {
        match (self, kind) {
            (_, SyntaxKind::ForKw) => LoopHeader::Names,
            (LoopHeader::Names, SyntaxKind::InKw) => LoopHeader::Iterable,
            (LoopHeader::Iterable, SyntaxKind::Colon) => LoopHeader::Closed,
            (header, _) => header,
        }
    }
}
