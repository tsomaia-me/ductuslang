use crate::syntax::syntax_kind::SyntaxKind;

#[derive(Clone, Copy)]
pub(crate) struct LineStart {
    pub(crate) first: SyntaxKind,
    pub(crate) next: Option<(SyntaxKind, bool)>,
    pub(crate) top_level: bool,
}
